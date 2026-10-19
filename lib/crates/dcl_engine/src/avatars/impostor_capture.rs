//! The node that, once per few frames, ranks the avatars by camera distance to decide who keeps
//! a mesh (the LOD pool), and that fills impostor textures one at a time.

use std::cell::RefCell;
use std::collections::HashSet;

use godot::classes::{INode, Image, Node};
use godot::prelude::*;

use crate::avatars::impostor::{cap_for_rank, LodState, DISTANCE_CHECK_PERIOD_FRAMES, POOL_SIZE};
use crate::content::texture::TextureEntry;
use crate::engine::local_task::{await_promise, spawn_local};
use crate::fetcher::globals;
use crate::godot_classes::dcl_avatar::DclAvatar;
use crate::godot_classes::promise::PromiseError;

/// Number of stock default-avatar profiles the catalyst exposes (`profiles/default1..160`).
const DEFAULT_BODY_COUNT: u64 = 160;

/// Bounding sphere around the avatar: center at the torso, radius covering head and feet.
const FRUSTUM_SPHERE_CENTER_OFFSET: Vector3 = Vector3::new(0.0, 0.9, 0.0);
const FRUSTUM_SPHERE_RADIUS: f32 = 1.2;

thread_local! {
    static AVATARS: RefCell<Vec<Gd<DclAvatar>>> = const { RefCell::new(Vec::new()) };
    static QUEUE: RefCell<Vec<Gd<DclAvatar>>> = const { RefCell::new(Vec::new()) };
    static ENQUEUED: RefCell<HashSet<i64>> = RefCell::new(HashSet::new());
    static IN_PROGRESS: RefCell<bool> = const { RefCell::new(false) };
}

pub fn register(avatar: Gd<DclAvatar>) {
    AVATARS.with(|list| {
        let mut list = list.borrow_mut();
        if !list.iter().any(|a| a.instance_id() == avatar.instance_id()) {
            list.push(avatar);
        }
    });
}

/// Every live avatar: comms peers, scene NPCs, the local player and previews.
pub fn registered() -> Vec<Gd<DclAvatar>> {
    AVATARS.with(|list| {
        list.borrow()
            .iter()
            .filter(|a| a.is_instance_valid())
            .cloned()
            .collect()
    })
}

pub fn unregister(avatar: &Gd<DclAvatar>) {
    AVATARS.with(|list| {
        list.borrow_mut()
            .retain(|a| a.instance_id() != avatar.instance_id());
    });
}

/// Queues the avatar for an impostor capture (deduplicated).
pub fn request_capture(avatar: Gd<DclAvatar>) {
    if !avatar.is_instance_valid() {
        return;
    }
    let id = avatar.instance_id().to_i64();
    if !ENQUEUED.with(|set| set.borrow_mut().insert(id)) {
        return;
    }
    QUEUE.with(|queue| queue.borrow_mut().push(avatar));
}

/// A stable stock-default slot (1..=160) per visual identity, so the same avatar keeps the
/// same default body across recaptures instead of flickering between bodies.
fn default_body_slot(key: &str, instance_id: i64) -> i32 {
    let seed = if key.is_empty() {
        instance_id.unsigned_abs()
    } else {
        // FNV-1a: stable across runs, unlike a hasher seeded per process.
        key.bytes().fold(0xcbf29ce484222325_u64, |h, b| {
            (h ^ b as u64).wrapping_mul(0x100000001b3)
        })
    };
    (seed % DEFAULT_BODY_COUNT) as i32 + 1
}

/// The point of the avatar's bounding sphere closest to the camera's forward axis: if even that
/// falls outside the frustum, the rest of the sphere does too, and one `is_position_in_frustum`
/// call replaces a sphere-versus-planes test.
pub fn sphere_axis_test_point(camera_pos: Vector3, forward: Vector3, center: Vector3) -> Vector3 {
    let to_center = center - camera_pos;
    let depth = to_center.dot(forward);
    if depth <= 0.0 {
        // At or behind the camera: the near plane rejects it.
        return center;
    }
    let axis_point = camera_pos + forward * depth;
    let perp = center - axis_point;
    if perp.length_squared() <= FRUSTUM_SPHERE_RADIUS * FRUSTUM_SPHERE_RADIUS {
        // The axis passes through the sphere.
        return axis_point;
    }
    center - perp.normalized() * FRUSTUM_SPHERE_RADIUS
}

fn texture_image(result: Variant) -> Option<Gd<Image>> {
    if result.try_to::<Gd<PromiseError>>().is_ok() {
        return None;
    }
    // The promise resolves a TextureEntry (image, texture, failed), not a bare texture.
    let entry = result.try_to::<Gd<TextureEntry>>().ok()?;
    let entry = entry.bind();
    if entry.failed {
        return None;
    }
    Some(entry.image.clone())
}

async fn image_for(avatar: &Gd<DclAvatar>) -> Option<Gd<Image>> {
    let (avatar_id, key, instance_id) = {
        let this = avatar.bind();
        (
            this.avatar_id.to_string(),
            this.get_impostor_cache_key(),
            avatar.instance_id().to_i64(),
        )
    };
    if avatar_id.starts_with("0x") {
        let promise = globals::content_provider()
            .bind_mut()
            .fetch_avatar_body_texture(avatar_id.as_str().into());
        if let Some(image) = texture_image(await_promise(promise).await) {
            return Some(image);
        }
    }
    // The avatar can leave during the fetch above: this queue does not own its lifetime.
    if !avatar.is_instance_valid() {
        return None;
    }
    // Never a main-thread local bake here: it blocks the frame (#2206).
    let slot = default_body_slot(&key, instance_id);
    let promise = globals::content_provider()
        .bind_mut()
        .fetch_default_avatar_body_texture(slot);
    texture_image(await_promise(promise).await)
}

async fn process_next(avatar: Gd<DclAvatar>) {
    let id = avatar.instance_id().to_i64();
    ENQUEUED.with(|set| set.borrow_mut().remove(&id));
    let image = image_for(&avatar).await;
    if let Some(image) = image {
        if avatar.is_instance_valid() {
            let key = GString::from(&avatar.bind().get_impostor_cache_key());
            globals::avatars_scene()
                .bind_mut()
                .set_impostor_texture(id, image, key);
        }
    }
    IN_PROGRESS.with(|flag| *flag.borrow_mut() = false);
}

#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct AvatarLodSystem {
    base: Base<Node>,
}

#[godot_api]
impl INode for AvatarLodSystem {
    fn process(&mut self, _delta: f64) {
        self.start_next_capture();
        self.update_pool();
    }
}

impl AvatarLodSystem {
    fn start_next_capture(&self) {
        if IN_PROGRESS.with(|flag| *flag.borrow()) {
            return;
        }
        let next = QUEUE.with(|queue| {
            let mut queue = queue.borrow_mut();
            (!queue.is_empty()).then(|| queue.remove(0))
        });
        let Some(avatar) = next else {
            return;
        };
        if !avatar.is_instance_valid() {
            return;
        }
        IN_PROGRESS.with(|flag| *flag.borrow_mut() = true);
        spawn_local(process_next(avatar));
    }

    /// Ranks the in-frustum avatars by camera distance: the closest run full-rate, the rest of
    /// the pool keep a throttled mesh, everyone beyond is a billboard. Off-screen avatars only
    /// release their impostor slot, so slots go to avatars that are on screen.
    fn update_pool(&self) {
        if AVATARS.with(|list| list.borrow().is_empty()) {
            return;
        }
        if !(godot::classes::Engine::singleton().get_frames_drawn() as u64)
            .is_multiple_of(DISTANCE_CHECK_PERIOD_FRAMES)
        {
            return;
        }
        let Some(camera) = self.base().get_viewport().and_then(|v| v.get_camera_3d()) else {
            return;
        };
        let camera_pos = camera.get_global_position();
        let forward = -camera.get_global_transform().basis.col_c();

        let avatars: Vec<Gd<DclAvatar>> = AVATARS.with(|list| {
            let mut list = list.borrow_mut();
            list.retain(|a| a.is_instance_valid());
            list.clone()
        });
        let mut ranked: Vec<(f32, Gd<DclAvatar>)> = Vec::new();
        for mut avatar in avatars {
            // Visible in the tree, not the node's own flag: avatars hidden by a parent entity
            // must not consume budget or get an impostor slot.
            let in_tree = avatar.is_visible_in_tree();
            let position = avatar.get_global_position();
            let mut this = avatar.bind_mut();
            this.lod.off_frustum = false;
            if this.is_local_player || this.hidden || !in_tree || !this.avatar_ready {
                this.lod.rank_cap = LodState::Full;
                this.lod.use_overflow = false;
                continue;
            }
            let center = position + FRUSTUM_SPHERE_CENTER_OFFSET;
            let point = sphere_axis_test_point(camera_pos, forward, center);
            if !camera.is_position_in_frustum(point) {
                // Only flag the slot for release. The cap keeps its last value so a briefly
                // off-screen FULL avatar does not pop its mesh out: the GPU cull already
                // skips it, so leaving the state alone costs nothing.
                this.lod.off_frustum = true;
                continue;
            }
            drop(this);
            ranked.push((camera_pos.distance_to(position), avatar));
        }
        ranked.sort_by(|a, b| a.0.total_cmp(&b.0));

        // Real impostor layers are finite VRAM: past pool + layers, avatars borrow another
        // slot's texture and render fully tinted.
        let max_real = globals::avatars_scene().bind().impostor_max_layers().max(0) as usize;
        for (rank, (_, mut avatar)) in ranked.into_iter().enumerate() {
            let (cap, overflow) = cap_for_rank(rank, POOL_SIZE, max_real);
            let mut this = avatar.bind_mut();
            this.lod.rank_cap = cap;
            this.lod.use_overflow = overflow;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_slot_is_stable_and_in_range() {
        let a = default_body_slot("some|key", 7);
        assert_eq!(a, default_body_slot("some|key", 99));
        assert!((1..=160).contains(&a));
        assert!((1..=160).contains(&default_body_slot("", 12345)));
        assert_eq!(default_body_slot("", 12345), default_body_slot("", -12345));
    }

    #[test]
    fn test_point_on_the_axis_when_it_passes_through_the_sphere() {
        let cam = Vector3::ZERO;
        let fwd = Vector3::new(0.0, 0.0, -1.0);
        let center = Vector3::new(0.5, 0.0, -10.0);
        assert_eq!(
            sphere_axis_test_point(cam, fwd, center),
            Vector3::new(0.0, 0.0, -10.0)
        );
    }

    #[test]
    fn test_point_is_pulled_toward_the_axis_by_the_radius() {
        let cam = Vector3::ZERO;
        let fwd = Vector3::new(0.0, 0.0, -1.0);
        let center = Vector3::new(5.0, 0.0, -10.0);
        let point = sphere_axis_test_point(cam, fwd, center);
        assert!((point.x - (5.0 - FRUSTUM_SPHERE_RADIUS)).abs() < 1e-5);
        assert_eq!(point.z, -10.0);
    }

    #[test]
    fn behind_the_camera_tests_the_center() {
        let center = Vector3::new(0.0, 0.0, 10.0);
        assert_eq!(
            sphere_axis_test_point(Vector3::ZERO, Vector3::new(0.0, 0.0, -1.0), center),
            center
        );
    }
}
