//! `PBAssetLoad` / `PBAssetLoadLoadingState` — scene-driven asset pre-loading.
//!
//! A scene sets `PBAssetLoad { assets: [<path>, ...] }` on an entity to ask the
//! client to pre-load those assets ahead of time, so that when the matching
//! `GltfContainer` (or other consumer) later appears there is no download /
//! processing / GPU-upload stall (no flickering / pop-in). The client reports
//! progress back per asset via `PBAssetLoadLoadingState`.
//!
//! Mirrors the desktop `bevy-explorer` implementation (protocol PR #339,
//! bevy PR #672) on mobile/godot.
//!
//! ## How far "to GPU" this goes
//!
//! For GLTF assets the preload runs the same download + runtime-process (or
//! optimized-asset) path a real `GltfContainer` would, then does the single
//! main-thread `ResourceLoader::load` and **retains the resulting `PackedScene`**
//! in [`AssetLoadState::preloads`] for as long as the `PBAssetLoad` component
//! stays on the entity. Holding that reference keeps the GLB's meshes and
//! textures resident on the GPU (in this engine the GPU upload cost is paid at
//! `ResourceLoader::load`, not at `add_child`), so the later real load is an
//! instant cache hit — no re-download, no re-processing, no re-upload.
//!
//! Texture paths are decoded + GPU-uploaded via the texture cache; other
//! non-GLTF paths only warm the download cache on disk.

use std::time::Instant;

use godot::classes::{PackedScene, ResourceLoader};
use godot::prelude::*;

use crate::{
    content::content_mapping::DclContentMappingAndUrl,
    dcl::{
        components::{
            proto_components::sdk::components::{common::LoadingState, PbAssetLoadLoadingState},
            SceneComponentId, SceneEntityId,
        },
        crdt::{
            grow_only_set::GenericGrowOnlySetComponentOperation,
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    godot_classes::{dcl_global::DclGlobal, promise::Promise},
    scene_runner::scene::Scene,
};

/// Per-scene state backing the `PBAssetLoad` component.
#[derive(Default)]
pub struct AssetLoadState {
    /// entity -> the assets it requested via `PBAssetLoad`.
    pub entities: std::collections::HashMap<SceneEntityId, Vec<TrackedAsset>>,
    /// content hash -> shared, scene-local preload (refcounted across the
    /// entities/assets that reference it).
    pub preloads: std::collections::HashMap<String, PreloadEntry>,
    /// Monotonic counter for `PBAssetLoadLoadingState.timestamp`. The component
    /// is a grow-only value set, so every appended transition reaches the scene;
    /// the counter just gives the SDK a stable ordering across assets.
    pub timestamp: u32,
}

/// One asset path requested by one entity.
pub struct TrackedAsset {
    /// The path exactly as the scene wrote it — reported back verbatim.
    pub path: String,
    /// Resolved content hash. Empty when the path is not in the content mapping.
    pub hash: String,
    /// Last `LoadingState` we emitted to the CRDT for this (entity, asset) pair.
    pub reported_state: i32,
}

/// A shared preload keyed by content hash.
pub struct PreloadEntry {
    /// How many tracked (entity, asset) pairs in this scene reference this hash.
    pub refcount: u32,
    /// Whether this is a GLTF asset (taken all the way to a retained `PackedScene`)
    /// vs a generic file (download-cache warmup only).
    pub is_gltf: bool,
    /// In-flight download promise. `None` once resolved (or never started).
    pub promise: Option<Gd<Promise>>,
    /// Resource path the GLTF loads from (`res://glbs/<hash>.scn` for optimized,
    /// or `user://content/<hash>.scn` for runtime — filled from the promise).
    pub scene_path: Option<String>,
    /// Retained loaded scene — pins meshes/textures in RAM + GPU. `None` until FINISHED.
    pub packed_scene: Option<Gd<PackedScene>>,
    /// Current `LoadingState` (Unknown/Loading/NotFound/FinishedWithError/Finished).
    pub state: i32,
}

/// React to `PBAssetLoad` add/change/remove: (un)register preloads and start
/// the shared downloads. Cheap — never blocks; the heavy main-thread load is
/// paced in [`sync_asset_load_loading_state`].
pub fn update_asset_load(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let Some(asset_load_dirty) = scene
        .current_dirty
        .lww_components
        .remove(&SceneComponentId::ASSET_LOAD)
    else {
        return;
    };

    // Snapshot the desired (entity -> paths) up front so the CRDT borrow ends
    // before we start mutating the scene's preload state.
    let desired: Vec<(SceneEntityId, Vec<String>)> = {
        let component = SceneCrdtStateProtoComponents::get_asset_load(crdt_state);
        asset_load_dirty
            .iter()
            .map(|entity| {
                // Keep the paths verbatim so the state we report back matches
                // exactly what the scene requested; casing is only normalized
                // when resolving the content hash.
                let paths = component
                    .get(entity)
                    .and_then(|entry| entry.value.as_ref())
                    .map(|value| value.assets.clone())
                    .unwrap_or_default();
                (*entity, paths)
            })
            .collect()
    };

    for (entity, paths) in desired {
        reconcile_entity_assets(scene, &entity, paths);
    }
}

/// Diff an entity's new asset list against what it tracked before: release the
/// preloads that are gone, start the ones that are new, keep the rest (so their
/// already-reported state is preserved and not re-emitted).
fn reconcile_entity_assets(scene: &mut Scene, entity: &SceneEntityId, new_paths: Vec<String>) {
    let previous = scene.asset_load.entities.remove(entity).unwrap_or_default();

    // Release refs for paths that are no longer requested.
    for old in &previous {
        if !old.hash.is_empty() && !new_paths.iter().any(|p| p == &old.path) {
            release_preload(scene, &old.hash);
        }
    }

    let mut tracked: Vec<TrackedAsset> = Vec::with_capacity(new_paths.len());
    for path in new_paths {
        // Unchanged path: carry the existing tracking so we don't re-emit.
        if let Some(existing) = previous.iter().find(|t| t.path == path) {
            tracked.push(TrackedAsset {
                path: existing.path.clone(),
                hash: existing.hash.clone(),
                reported_state: existing.reported_state,
            });
            continue;
        }

        // New path: resolve to a content hash and start (or attach to) a preload.
        // Content mapping lookups are case-insensitive (callers lowercase).
        let hash = scene
            .content_mapping
            .get_hash(&path.to_lowercase())
            .cloned()
            .unwrap_or_default();

        if !hash.is_empty() {
            acquire_preload(scene, &hash, &path);
        }

        tracked.push(TrackedAsset {
            path,
            hash,
            reported_state: LoadingState::Unknown as i32,
        });
    }

    // If the component was deleted (no paths), the entity is simply dropped.
    if !tracked.is_empty() {
        scene.asset_load.entities.insert(*entity, tracked);
    }
}

/// Attach to an existing preload for `hash` (bumping its refcount) or create one.
fn acquire_preload(scene: &mut Scene, hash: &str, path: &str) {
    if let Some(entry) = scene.asset_load.preloads.get_mut(hash) {
        entry.refcount += 1;
        return;
    }
    let entry = start_preload(scene, hash, path);
    scene.asset_load.preloads.insert(hash.to_string(), entry);
}

/// Detach one reference; drop the shared preload (freeing RAM/GPU) at zero.
fn release_preload(scene: &mut Scene, hash: &str) {
    let Some(entry) = scene.asset_load.preloads.get_mut(hash) else {
        return;
    };
    entry.refcount = entry.refcount.saturating_sub(1);
    if entry.refcount > 0 {
        return;
    }
    // Removing the entry drops the retained `PackedScene`, releasing the GPU
    // resources once nothing else references them.
    scene.asset_load.preloads.remove(hash);
}

/// Kick off the download for a freshly-created preload, mirroring
/// `gltf_container.gd`'s optimized-vs-runtime decision so we warm the exact
/// resource the real `GltfContainer` will later load.
fn start_preload(scene: &Scene, hash: &str, path: &str) -> PreloadEntry {
    let (mut content_provider, cli_gd) = {
        let global = DclGlobal::singleton();
        let global = global.bind();
        (global.get_content_provider(), global.cli.clone())
    };
    let (only_optimized, only_no_optimized) = {
        let cli = cli_gd.bind();
        (cli.only_optimized, cli.only_no_optimized)
    };

    let is_gltf = path.ends_with(".glb") || path.ends_with(".gltf");

    if !is_gltf {
        let mapping = DclContentMappingAndUrl::from_ref(scene.content_mapping.clone());
        let is_texture = path.ends_with(".png")
            || path.ends_with(".jpg")
            || path.ends_with(".jpeg")
            || path.ends_with(".ktx2");
        // Textures are decoded + uploaded through the texture cache (so a later
        // Material reference is a GPU cache hit); other files just warm the
        // download cache on disk.
        let promise = if is_texture {
            content_provider
                .bind_mut()
                .fetch_texture(path.to_godot(), mapping)
        } else {
            content_provider
                .bind_mut()
                .fetch_file(path.to_godot(), mapping)
        };
        return loading_entry(false, Some(promise), None);
    }

    let has_optimized = content_provider
        .bind()
        .optimized_asset_exists(hash.to_godot());

    // --only-optimized with no optimized asset => nothing to preload.
    if only_optimized && !has_optimized {
        return not_found_entry(true);
    }

    let use_optimized = has_optimized && !only_no_optimized;

    if use_optimized {
        let promise = content_provider
            .bind_mut()
            .fetch_optimized_asset_with_dependencies(hash.to_godot());
        return loading_entry(true, Some(promise), Some(format!("res://glbs/{hash}.scn")));
    }

    // Runtime path: download + process the GLB into `user://content/<hash>.scn`.
    // The resolved path comes back on the promise.
    let mapping = DclContentMappingAndUrl::from_ref(scene.content_mapping.clone());
    let promise = content_provider
        .bind_mut()
        .load_scene_gltf(path.to_godot(), mapping);
    match promise {
        Some(promise) => loading_entry(true, Some(promise), None),
        None => not_found_entry(true),
    }
}

fn loading_entry(
    is_gltf: bool,
    promise: Option<Gd<Promise>>,
    scene_path: Option<String>,
) -> PreloadEntry {
    PreloadEntry {
        refcount: 1,
        is_gltf,
        promise,
        scene_path,
        packed_scene: None,
        state: LoadingState::Loading as i32,
    }
}

fn not_found_entry(is_gltf: bool) -> PreloadEntry {
    PreloadEntry {
        refcount: 1,
        is_gltf,
        promise: None,
        scene_path: None,
        packed_scene: None,
        state: LoadingState::NotFound as i32,
    }
}

/// Advance in-flight preloads, then append a `PBAssetLoadLoadingState` for every
/// per-asset transition since last tick.
///
/// Returns `false` when it stopped early because the per-tick budget ran out
/// (so the caller re-enters next tick), `true` when everything for this tick is
/// done.
pub fn sync_asset_load_loading_state(
    scene: &mut Scene,
    crdt_state: &mut SceneCrdtState,
    ref_time: &Instant,
    end_time_us: i64,
) -> bool {
    let completed = advance_preloads(scene, ref_time, end_time_us);

    // Snapshot current per-hash states so the emit loop below can borrow
    // `entities` mutably without conflicting with `preloads`.
    let states: std::collections::HashMap<String, i32> = scene
        .asset_load
        .preloads
        .iter()
        .map(|(hash, entry)| (hash.clone(), entry.state))
        .collect();

    let mut timestamp = scene.asset_load.timestamp;
    let component = SceneCrdtStateProtoComponents::get_asset_load_loading_state_mut(crdt_state);

    // The component is a grow-only value set: every appended value reaches the
    // scene, so we can emit all of an entity's changed assets in one tick.
    for (entity, tracked) in scene.asset_load.entities.iter_mut() {
        for asset in tracked.iter_mut() {
            let current = if asset.hash.is_empty() {
                LoadingState::NotFound as i32
            } else {
                states
                    .get(&asset.hash)
                    .copied()
                    .unwrap_or(LoadingState::Unknown as i32)
            };

            if current != asset.reported_state {
                asset.reported_state = current;
                timestamp = timestamp.wrapping_add(1);
                component.append(
                    *entity,
                    PbAssetLoadLoadingState {
                        current_state: current,
                        asset: asset.path.clone(),
                        timestamp,
                    },
                );
            }
        }
    }

    scene.asset_load.timestamp = timestamp;
    completed
}

/// Poll resolved download promises and, for GLTF preloads, perform the single
/// main-thread `ResourceLoader::load` (the GPU-upload step) — paced against the
/// per-tick budget so many finishing at once don't spike a frame.
///
/// Returns `false` if it bailed early on the budget.
fn advance_preloads(scene: &mut Scene, ref_time: &Instant, end_time_us: i64) -> bool {
    let hashes: Vec<String> = scene.asset_load.preloads.keys().cloned().collect();

    for hash in hashes {
        if (Instant::now() - *ref_time).as_micros() as i64 > end_time_us {
            return false;
        }

        let Some(entry) = scene.asset_load.preloads.get_mut(&hash) else {
            continue;
        };
        // Only entries with a still-pending, now-resolved promise need work.
        let Some(promise) = entry.promise.clone() else {
            continue;
        };
        if !promise.bind().is_resolved() {
            continue;
        }
        entry.promise = None;

        if promise.bind().is_rejected() {
            entry.state = LoadingState::FinishedWithError as i32;
            continue;
        }

        // Non-GLTF: download done, cache warmed, nothing to upload.
        if !entry.is_gltf {
            entry.state = LoadingState::Finished as i32;
            continue;
        }

        // Runtime GLTF resolves with the on-disk scene path.
        if entry.scene_path.is_none() {
            let path = promise
                .bind()
                .get_data()
                .try_to::<GString>()
                .map(|g| g.to_string())
                .unwrap_or_default();
            if path.is_empty() {
                entry.state = LoadingState::FinishedWithError as i32;
                continue;
            }
            entry.scene_path = Some(path);
        }
        let scene_path = entry.scene_path.clone().unwrap();

        // Single main-thread load — retains the resource (meshes/textures to GPU).
        let loaded = ResourceLoader::singleton()
            .load(scene_path.as_str())
            .and_then(|res| res.try_cast::<PackedScene>().ok());

        match loaded {
            Some(packed) => {
                entry.packed_scene = Some(packed);
                entry.state = LoadingState::Finished as i32;
            }
            None => {
                entry.state = LoadingState::FinishedWithError as i32;
            }
        }
    }

    true
}
