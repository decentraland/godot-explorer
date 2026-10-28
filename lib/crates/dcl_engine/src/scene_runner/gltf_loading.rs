//! Shared, source-deduplicated glTF loading, with a two-stage pipeline that decouples network
//! concurrency from main-thread work.
//!
//! N entities that reference the same asset used to be N independent loads, each queueing and
//! downloading the same bytes (~42 s median queue wait per instance on Genesis Plaza). Now a
//! content hash is one `LoadGroup`:
//!
//!  1. DOWNLOAD stage, throttled by `MAX_CONCURRENT_DOWNLOADS` (network-bound, off-thread): one
//!     fetch per hash, shared by every waiter. The real HTTP parallelism is capped by the content
//!     provider's queue (12); this only keeps the fetch stage from being the wall.
//!  2. LOAD+REALIZE stage, a single main-thread pump with a per-frame time budget (at least one
//!     source per frame): the one main-thread `ResourceLoader.load` (the GPU texture upload must
//!     be on the main thread, or it deadlocks the RenderingServer on Mali), then every waiter is
//!     instantiated (a cheap CPU-only clone of the shared PackedScene) and added in that frame.
//!
//! Each container flips its own `dcl_gltf_loading_state`, which the scene runner polls to dismiss
//! the loading screen.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use godot::classes::resource_loader::CacheMode;
use godot::classes::{FileAccess, PackedScene, ResourceLoader, Time};
use godot::prelude::*;

use crate::content::content_mapping::DclContentMappingAndUrl;
use crate::engine::frame_budget;
use crate::engine::local_task::{await_promise, next_frame, spawn_local};
use crate::fetcher::globals;
use crate::godot_classes::dcl_gltf_container::DclGltfContainer;
use crate::godot_classes::promise::PromiseError;

/// Network fetch concurrency. High on purpose: the HTTP cap lives in the content provider.
const MAX_CONCURRENT_DOWNLOADS: usize = 24;

/// Main-thread budget per frame for the load+realize pump: sources are pulled until it is spent
/// (at least one per frame), so a queue of cheap sources drains in a few frames.
const LOAD_PUMP_BUDGET_USEC: u64 = 12_000;

/// Milliseconds before a container that has not finished is failed.
pub const LOAD_TIMEOUT_MS: u64 = 120_000;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum GroupState {
    /// Created, waiting for a download slot.
    Pending,
    /// Holding a download slot, fetching.
    Downloading,
    /// Bytes on disk, queued for the main-thread load and realize.
    Fetched,
    /// Being loaded, instantiated and added this frame.
    Realizing,
    /// Every current waiter realized.
    Done,
    /// The fetch or load failed; the waiters are notified.
    Error,
}

struct LoadGroup {
    hash: String,
    src: String,
    content_mapping: Gd<DclContentMappingAndUrl>,
    optimized: bool,
    state: GroupState,
    scene_path: String,
    /// Bypass ResourceLoader's cache: a preview hot-reload keeps the hash (it hashes file paths)
    /// and so the `.scn` path, and a cached load would hand back the previous version.
    force_fresh: bool,
    packed_scene: Option<Gd<PackedScene>>,
    waiters: Vec<Waiter>,
}

/// A container waiting on a group, with its scene: read without binding it, since the container
/// that requests a load is bound while it does.
#[derive(Clone)]
struct Waiter {
    container: Gd<DclGltfContainer>,
    scene_id: i32,
}

type Group = Rc<RefCell<LoadGroup>>;

#[derive(Default)]
struct Coordinator {
    groups: HashMap<String, Group>,
    /// Hashes of pending groups waiting for a download slot.
    download_queue: Vec<String>,
    downloading_count: usize,
    /// Groups whose bytes are on disk, waiting for the load pump.
    load_queue: Vec<Group>,
    load_pump_running: bool,
    stats: Stats,
}

/// Cumulative counters since boot (times in microseconds).
#[derive(Default, Clone)]
pub struct Stats {
    pub groups: u64,
    pub download_usec: u64,
    pub download_max_usec: u64,
    pub loads: u64,
    pub load_usec: u64,
    pub load_max_usec: u64,
    pub instances: u64,
    pub instantiate_usec: u64,
    pub pump_frames: u64,
    pub pump_usec: u64,
    pub errors: u64,
}

thread_local! {
    static COORDINATOR: RefCell<Coordinator> = RefCell::new(Coordinator::default());
    static DEADLINES: RefCell<Deadlines> = RefCell::new(Deadlines::default());
    static DEADLINE_LOOP: RefCell<bool> = const { RefCell::new(false) };
}

fn with<R>(f: impl FnOnce(&mut Coordinator) -> R) -> R {
    COORDINATOR.with(|c| f(&mut c.borrow_mut()))
}

/// The pipeline's counters since boot.
pub fn stats() -> (Stats, usize) {
    with(|c| (c.stats.clone(), c.groups.len()))
}

fn now_usec() -> u64 {
    Time::singleton().get_ticks_usec()
}

// ---- timeouts ----

/// Load deadlines by container instance id, drained once per frame. One map replaces a Timer node
/// per container (1419 of them in Genesis Plaza).
#[derive(Default)]
pub struct Deadlines {
    by_container: HashMap<InstanceId, u64>,
}

impl Deadlines {
    pub fn schedule(&mut self, id: InstanceId, deadline_ms: u64) {
        self.by_container.insert(id, deadline_ms);
    }

    pub fn cancel(&mut self, id: InstanceId) {
        self.by_container.remove(&id);
    }

    /// The containers whose deadline passed; they are removed.
    pub fn drain_due(&mut self, now_ms: u64) -> Vec<InstanceId> {
        let due: Vec<InstanceId> = self
            .by_container
            .iter()
            .filter(|(_, deadline)| **deadline <= now_ms)
            .map(|(id, _)| *id)
            .collect();
        for id in &due {
            self.by_container.remove(id);
        }
        due
    }

    pub fn is_empty(&self) -> bool {
        self.by_container.is_empty()
    }
}

pub fn schedule_timeout(container: &Gd<DclGltfContainer>, timeout_ms: u64) {
    let deadline = Time::singleton().get_ticks_msec() + timeout_ms;
    DEADLINES.with(|d| d.borrow_mut().schedule(container.instance_id(), deadline));
    // The drain runs off the frame loop, not a `_process` on `Global`: adding one there
    // reproducibly crashed the Android driver (a VkThread NULL deref, Samsung A54; bisected).
    if !DEADLINE_LOOP.with(|running| running.replace(true)) {
        spawn_local(drain_deadlines());
    }
}

pub fn cancel_timeout(container: &Gd<DclGltfContainer>) {
    DEADLINES.with(|d| d.borrow_mut().cancel(container.instance_id()));
}

async fn drain_deadlines() {
    loop {
        next_frame().await;
        let due = DEADLINES.with(|d| {
            let mut d = d.borrow_mut();
            if d.is_empty() {
                return Vec::new();
            }
            d.drain_due(Time::singleton().get_ticks_msec())
        });
        for id in due {
            if let Ok(mut container) = Gd::<DclGltfContainer>::try_from_instance_id(id) {
                container.bind_mut().on_load_timeout();
            }
        }
    }
}

// ---- public API, called by the container ----

/// Registers a container as a waiter on its content hash. The first requester creates the group
/// and enqueues it for download; later ones attach and are realized when the shared load
/// completes (or queued at once when the bytes are already fetched).
pub fn request(
    container: &Gd<DclGltfContainer>,
    scene_id: i32,
    hash: &str,
    src: &str,
    optimized: bool,
    content_mapping: Gd<DclContentMappingAndUrl>,
    force_fresh: bool,
) {
    let existing = with(|c| c.groups.get(hash).cloned());
    if let (Some(group), true) = (existing.as_ref(), force_fresh) {
        let mut group = group.borrow_mut();
        group.force_fresh = true;
        group.packed_scene = None;
    }

    let Some(group) = existing else {
        let group = Rc::new(RefCell::new(LoadGroup {
            hash: hash.to_string(),
            src: src.to_string(),
            content_mapping,
            optimized,
            state: GroupState::Pending,
            scene_path: String::new(),
            force_fresh,
            packed_scene: None,
            waiters: vec![Waiter {
                container: container.clone(),
                scene_id,
            }],
        }));
        with(|c| {
            c.groups.insert(hash.to_string(), group);
            c.download_queue.push(hash.to_string());
        });
        pump_downloads();
        return;
    };

    let state = {
        let mut group = group.borrow_mut();
        if group
            .waiters
            .iter()
            .any(|w| w.container.instance_id() == container.instance_id())
        {
            return;
        }
        group.waiters.push(Waiter {
            container: container.clone(),
            scene_id,
        });
        group.state
    };
    match state {
        // Realized in the batch when the shared download and load complete.
        GroupState::Pending | GroupState::Downloading => {}
        GroupState::Fetched | GroupState::Realizing => {
            // The bytes are on disk: this late waiter attaches to the shared load.
            queue_for_load(&group);
            ensure_load_pump();
        }
        GroupState::Error => {
            container
                .clone()
                .bind_mut()
                .on_shared_load_error("shared load failed");
        }
        GroupState::Done => {}
    }
}

/// Detaches a container (it left the tree or reloads a new src).
pub fn unregister(container: &Gd<DclGltfContainer>, hash: &str) {
    if let Some(group) = with(|c| c.groups.get(hash).cloned()) {
        group
            .borrow_mut()
            .waiters
            .retain(|w| w.container.instance_id() != container.instance_id());
    }
}

fn queue_for_load(group: &Group) {
    with(|c| {
        if !c.load_queue.iter().any(|g| Rc::ptr_eq(g, group)) {
            c.load_queue.push(group.clone());
        }
    });
}

// ---- download stage ----

/// A deeplink param, the benchmark knobs (`max-groups`, `threaded-load`).
fn deep_link_param(name: &str) -> Option<i64> {
    let link = crate::godot_classes::dcl_global::DclGlobal::singleton()
        .bind()
        .deep_link_obj
        .clone();
    let params = link.bind().get_params();
    params.get(name)?.to_string().parse::<i64>().ok()
}

fn max_concurrent_downloads() -> usize {
    thread_local! {
        static CACHE: RefCell<Option<usize>> = const { RefCell::new(None) };
    }
    CACHE.with(|cache| {
        *cache.borrow_mut().get_or_insert_with(|| {
            deep_link_param("max-groups")
                .filter(|v| *v > 0)
                .map_or(MAX_CONCURRENT_DOWNLOADS, |v| v as usize)
        })
    })
}

fn has_current_scene_waiter(group: &Group) -> bool {
    let current = globals::scene_runner().bind().get_current_parcel_scene_id();
    group
        .borrow()
        .waiters
        .iter()
        .any(|w| w.container.is_instance_valid() && w.scene_id == current)
}

/// The next hash to download: a group with a current-scene waiter first, else the oldest.
fn pop_next_download() -> Option<String> {
    let queue = with(|c| c.download_queue.clone());
    for (index, hash) in queue.iter().enumerate() {
        let group = with(|c| c.groups.get(hash).cloned());
        if group.is_some_and(|g| has_current_scene_waiter(&g)) {
            with(|c| c.download_queue.remove(index));
            return Some(hash.clone());
        }
    }
    loop {
        let hash = with(|c| (!c.download_queue.is_empty()).then(|| c.download_queue.remove(0)))?;
        if with(|c| c.groups.contains_key(&hash)) {
            return Some(hash);
        }
    }
}

fn pump_downloads() {
    while with(|c| c.downloading_count) < max_concurrent_downloads()
        && with(|c| !c.download_queue.is_empty())
    {
        let Some(hash) = pop_next_download() else {
            break;
        };
        let Some(group) = with(|c| c.groups.get(&hash).cloned()) else {
            continue;
        };
        if group.borrow().state != GroupState::Pending {
            continue;
        }
        group
            .borrow_mut()
            .waiters
            .retain(|w| w.container.is_instance_valid());
        if group.borrow().waiters.is_empty() {
            with(|c| c.groups.remove(&hash));
            continue;
        }
        group.borrow_mut().state = GroupState::Downloading;
        with(|c| c.downloading_count += 1);
        spawn_local(download_group(group));
    }
}

async fn download_group(group: Group) {
    let started = now_usec();
    with(|c| c.stats.groups += 1);
    let (optimized, hash, src, mapping) = {
        let g = group.borrow();
        (
            g.optimized,
            g.hash.clone(),
            g.src.clone(),
            g.content_mapping.clone(),
        )
    };
    let scene_path;
    if optimized {
        let promise = globals::content_provider()
            .bind_mut()
            .fetch_optimized_asset_with_dependencies(hash.as_str().into());
        let result = await_promise(promise).await;
        if result.try_to::<Gd<PromiseError>>().is_ok() {
            fail_download(&group, "failed to download optimized asset dependencies");
            return;
        }
        scene_path = globals::content_provider()
            .bind()
            .get_optimized_scene_path(hash.as_str().into())
            .to_string();
        if !FileAccess::file_exists(scene_path.as_str()) {
            fail_download(&group, &format!("optimized scene not found: {scene_path}"));
            return;
        }
    } else {
        let promise = globals::content_provider()
            .bind_mut()
            .load_scene_gltf(src.as_str().into(), mapping);
        let Some(promise) = promise else {
            fail_download(&group, "failed to start loading");
            return;
        };
        let result = await_promise(promise).await;
        if let Ok(error) = result.try_to::<Gd<PromiseError>>() {
            fail_download(&group, &error.bind().get_error().to_string());
            return;
        }
        match result.try_to::<GString>() {
            Ok(path) if !path.is_empty() => scene_path = path.to_string(),
            _ => {
                fail_download(&group, "invalid scene path");
                return;
            }
        }
    }

    let elapsed = now_usec().saturating_sub(started);
    with(|c| {
        c.stats.download_usec += elapsed;
        c.stats.download_max_usec = c.stats.download_max_usec.max(elapsed);
    });
    {
        let mut g = group.borrow_mut();
        g.scene_path = scene_path;
        g.state = GroupState::Fetched;
    }
    release_download_slot();
    queue_for_load(&group);
    ensure_load_pump();
}

fn fail_download(group: &Group, reason: &str) {
    release_download_slot();
    notify_group_error(group, reason);
}

fn release_download_slot() {
    with(|c| c.downloading_count = c.downloading_count.saturating_sub(1));
    pump_downloads();
}

// ---- load and realize stage ----

fn ensure_load_pump() {
    let start = with(|c| !std::mem::replace(&mut c.load_pump_running, true));
    if start {
        spawn_local(run_load_pump());
    }
}

async fn run_load_pump() {
    while with(|c| !c.load_queue.is_empty()) {
        // Off the loading screen the pump is one frame-budget step that batches while time is
        // left; priority, so scene models never queue behind remote avatar builds.
        if !frame_budget::loading() {
            frame_budget::acquire(None, true).await;
        }
        let frame_start = now_usec();
        let mut realized: Vec<(Group, Vec<Gd<DclGltfContainer>>)> = Vec::new();

        while let Some(group) = with(|c| (!c.load_queue.is_empty()).then(|| c.load_queue.remove(0)))
        {
            let batch = load_and_realize_group(&group);
            if !batch.is_empty() {
                realized.push((group, batch));
            }
            if now_usec().saturating_sub(frame_start) >= LOAD_PUMP_BUDGET_USEC {
                break;
            }
            if !frame_budget::loading() && !frame_budget::has_time() {
                break;
            }
        }
        let spent = now_usec().saturating_sub(frame_start);
        with(|c| {
            c.stats.pump_frames += 1;
            c.stats.pump_usec += spent;
        });

        // The render frame between the add and the completion lets colliders and rendering be
        // ready when the container reports FINISHED.
        next_frame().await;

        for (group, batch) in realized {
            complete_realized_group(&group, batch);
        }
    }
    with(|c| c.load_pump_running = false);
}

/// For one source: the single main-thread `ResourceLoader.load` (first pass only), then every
/// not-yet-realized waiter is instantiated and added. Returns the realized waiters; they are
/// marked FINISHED after the next render frame.
fn load_and_realize_group(group: &Group) -> Vec<Gd<DclGltfContainer>> {
    if group.borrow().packed_scene.is_none() {
        // Synchronous, once per source: the optimized .scn embeds texture atlases that upload to
        // the GPU during load, and doing that on the worker pool raced the render thread over the
        // RenderingServer command lock and deadlocked on Mali.
        let (path, force_fresh) = {
            let g = group.borrow();
            (g.scene_path.clone(), g.force_fresh)
        };
        let mode = if force_fresh {
            CacheMode::REPLACE
        } else {
            CacheMode::REUSE
        };
        let started = now_usec();
        let resource = ResourceLoader::singleton()
            .load_ex(path.as_str())
            .cache_mode(mode)
            .done();
        let elapsed = now_usec().saturating_sub(started);
        with(|c| {
            c.stats.loads += 1;
            c.stats.load_usec += elapsed;
            c.stats.load_max_usec = c.stats.load_max_usec.max(elapsed);
        });
        match resource.and_then(|r| r.try_cast::<PackedScene>().ok()) {
            Some(scene) => group.borrow_mut().packed_scene = Some(scene),
            None => {
                notify_group_error(group, "loaded resource is null");
                return Vec::new();
            }
        }
    }

    group.borrow_mut().state = GroupState::Realizing;
    let waiters = group.borrow().waiters.clone();
    let batch: Vec<Gd<DclGltfContainer>> = waiters
        .into_iter()
        .map(|w| w.container)
        .filter(|w| w.is_instance_valid() && w.bind().needs_realize())
        .collect();
    if batch.is_empty() {
        retire_group(group);
        return Vec::new();
    }
    let started = now_usec();
    let Some(scene) = group.borrow().packed_scene.clone() else {
        return Vec::new();
    };
    for waiter in &batch {
        waiter.clone().bind_mut().instantiate_and_add(&scene);
    }
    let elapsed = now_usec().saturating_sub(started);
    with(|c| {
        c.stats.instances += batch.len() as u64;
        c.stats.instantiate_usec += elapsed;
    });
    batch
}

/// After the render frame following a realize pass: the batch is FINISHED, and the group retires
/// or is queued again.
fn complete_realized_group(group: &Group, batch: Vec<Gd<DclGltfContainer>>) {
    for mut waiter in batch {
        if waiter.is_instance_valid() {
            waiter.bind_mut().complete_shared_load();
        }
    }
    // A waiter may have attached during the render frame: realize it next pass.
    let waiters = group.borrow().waiters.clone();
    if waiters
        .iter()
        .any(|w| w.container.is_instance_valid() && w.container.bind().needs_realize())
    {
        group.borrow_mut().state = GroupState::Fetched;
        queue_for_load(group);
        return;
    }
    retire_group(group);
}

/// Drops the shared PackedScene and the group once every waiter is realized, so it is not pinned
/// for the app's lifetime (it would leak on OOM-sensitive mobile). A later entity referencing this
/// hash creates a fresh group and loads again.
fn retire_group(group: &Group) {
    let hash = {
        let mut g = group.borrow_mut();
        g.state = GroupState::Done;
        g.packed_scene = None;
        g.hash.clone()
    };
    with(|c| {
        if c.groups.get(&hash).is_some_and(|g| Rc::ptr_eq(g, group)) {
            c.groups.remove(&hash);
        }
    });
}

fn notify_group_error(group: &Group, reason: &str) {
    let (hash, waiters) = {
        let mut g = group.borrow_mut();
        g.state = GroupState::Error;
        (g.hash.clone(), g.waiters.clone())
    };
    with(|c| c.stats.errors += 1);
    // Feeds the loading funnel, so the load's `assets_errored` reflects this failure.
    globals::scene_runner()
        .bind_mut()
        .loading_note_asset_failure(1);
    for waiter in waiters {
        let mut container = waiter.container;
        if container.is_instance_valid() {
            container.bind_mut().on_shared_load_error(reason);
        }
    }
    // The group goes, so a later request for this hash can retry: a transient failure must not
    // blacklist the asset.
    with(|c| {
        if c.groups.get(&hash).is_some_and(|g| Rc::ptr_eq(g, group)) {
            c.groups.remove(&hash);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: i64) -> InstanceId {
        InstanceId::from_i64(n)
    }

    #[test]
    fn deadlines_fire_once_when_due() {
        let mut deadlines = Deadlines::default();
        deadlines.schedule(id(1), 1000);
        deadlines.schedule(id(2), 2000);
        assert!(deadlines.drain_due(999).is_empty());
        assert_eq!(deadlines.drain_due(1000), vec![id(1)]);
        assert!(deadlines.drain_due(1500).is_empty());
        assert_eq!(deadlines.drain_due(5000), vec![id(2)]);
        assert!(deadlines.is_empty());
    }

    #[test]
    fn a_cancelled_deadline_never_fires() {
        let mut deadlines = Deadlines::default();
        deadlines.schedule(id(1), 10);
        deadlines.cancel(id(1));
        assert!(deadlines.drain_due(100).is_empty());
    }

    #[test]
    fn rescheduling_replaces_the_deadline() {
        let mut deadlines = Deadlines::default();
        deadlines.schedule(id(1), 10);
        deadlines.schedule(id(1), 500);
        assert!(deadlines.drain_due(100).is_empty());
        assert_eq!(deadlines.drain_due(500), vec![id(1)]);
    }
}
