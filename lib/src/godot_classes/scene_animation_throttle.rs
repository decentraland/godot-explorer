//! Drives scene GLTF animators (GltfContainer `AnimationPlayer` / `MultipleAnimator`) from one
//! per-frame pass instead of each mixer's own idle process: off-screen or hidden ones pause, far
//! ones advance every 2nd/4th frame, and every advance carries the time accumulated since the last.

use std::cell::RefCell;

use godot::builtin::{Aabb, Plane, Transform3D, Vector3};
use godot::classes::animation_mixer::AnimationCallbackModeProcess;
use godot::classes::{
    AnimationMixer, AnimationPlayer, Camera3D, CollisionObject3D, Node, Node3D, VisualInstance3D,
};
use godot::obj::{Gd, InstanceId};

/// SDK `CL_PHYSICS` collision layer bit: bodies the player can stand on or be pushed by.
const CL_PHYSICS: u32 = 2;
/// Animated physics colliders within this range of the camera always run at full rate.
const COLLIDER_FULL_RATE_RANGE_M: f32 = 30.0;
const FULL_RATE_RANGE_M: f32 = 15.0;
const HALF_RATE_RANGE_M: f32 = 40.0;
/// Skinned and node-animated parts leave their rest bounds; pad the sphere to avoid edge pops.
const BOUNDS_MARGIN_M: f32 = 2.0;
const BOUNDS_MARGIN_RATIO: f32 = 0.5;

struct Entry {
    mixer_id: InstanceId,
    root_id: InstanceId,
    /// Bounding sphere in the GLTF root's local space.
    center: Vector3,
    radius: f32,
    physics_collider: bool,
    accum: f64,
    phase: u64,
}

#[derive(Default)]
struct Registry {
    entries: Vec<Entry>,
    frame: u64,
    refresh_cursor: usize,
    next_phase: u64,
}

thread_local! {
    static REGISTRY: RefCell<Registry> = RefCell::new(Registry::default());
}

/// Hands `mixer` (the animator of the GLTF instance `gltf_root`) to the per-frame pass.
pub fn register(mut mixer: Gd<AnimationMixer>, gltf_root: &Gd<Node3D>) {
    let mixer_id = mixer.instance_id();
    REGISTRY.with_borrow_mut(|reg| {
        if reg.entries.iter().any(|e| e.mixer_id == mixer_id) {
            return;
        }
        mixer.set_callback_mode_process(AnimationCallbackModeProcess::MANUAL);
        let (center, radius, physics_collider) = scan(gltf_root);
        let phase = reg.next_phase;
        reg.next_phase = reg.next_phase.wrapping_add(1);
        reg.entries.push(Entry {
            mixer_id,
            root_id: gltf_root.instance_id(),
            center,
            radius,
            physics_collider,
            accum: 0.0,
            phase,
        });
    });
}

/// Advances the registered animators that are due this frame. Engine thread, once per frame.
pub fn tick(camera: Option<&Gd<Camera3D>>, delta: f64) {
    let _zone = crate::tools::profiler::Zone::new("SceneAnimationThrottle::tick", "");
    let view = camera.map(|camera| {
        let planes: Vec<Plane> = camera.get_frustum().iter_shared().collect();
        (planes, camera.get_global_position())
    });

    // Taken out of the registry so `advance` callbacks can register new animators.
    let (mut entries, frame, refresh_cursor) = REGISTRY.with_borrow_mut(|reg| {
        reg.frame = reg.frame.wrapping_add(1);
        reg.refresh_cursor = reg.refresh_cursor.wrapping_add(1);
        (
            std::mem::take(&mut reg.entries),
            reg.frame,
            reg.refresh_cursor,
        )
    });

    // Re-scan one entry per frame: collision layers and attached meshes change at runtime.
    if !entries.is_empty() {
        let index = refresh_cursor % entries.len();
        let entry = &mut entries[index];
        if let Ok(root) = Gd::<Node3D>::try_from_instance_id(entry.root_id) {
            (entry.center, entry.radius, entry.physics_collider) = scan(&root);
        }
    }

    entries.retain_mut(|entry| {
        let Ok(mut mixer) = Gd::<AnimationMixer>::try_from_instance_id(entry.mixer_id) else {
            return false;
        };
        let Ok(root) = Gd::<Node3D>::try_from_instance_id(entry.root_id) else {
            return false;
        };
        if !mixer.is_inside_tree() || !root.is_inside_tree() {
            return true;
        }

        let running = match mixer.clone().try_cast::<AnimationPlayer>() {
            Ok(player) => player.is_playing(),
            Err(mixer) => mixer.is_active(),
        };
        if !running {
            entry.accum = 0.0;
            return true;
        }
        entry.accum += delta;

        let interval = match &view {
            Some((planes, camera_position)) => {
                update_interval(entry, &root, planes, *camera_position)
            }
            None => 1,
        };
        if interval > 0 && (frame + entry.phase) % interval == 0 {
            mixer.advance(entry.accum);
            entry.accum = 0.0;
        }
        true
    });

    REGISTRY.with_borrow_mut(|reg| {
        for added in std::mem::take(&mut reg.entries) {
            if !entries.iter().any(|e| e.mixer_id == added.mixer_id) {
                entries.push(added);
            }
        }
        reg.entries = entries;
    });
}

/// 0 = paused, otherwise advance every `n` frames.
fn update_interval(
    entry: &Entry,
    root: &Gd<Node3D>,
    planes: &[Plane],
    camera_position: Vector3,
) -> u64 {
    let xform = root.get_global_transform();
    let scale = xform.basis.get_scale().abs();
    let center = xform * entry.center;
    let radius = entry.radius * scale.x.max(scale.y).max(scale.z);
    let radius = radius + (radius * BOUNDS_MARGIN_RATIO).max(BOUNDS_MARGIN_M);
    let distance = ((center - camera_position).length() - radius).max(0.0);

    if entry.physics_collider && distance < COLLIDER_FULL_RATE_RANGE_M {
        return 1;
    }
    if !root.is_visible_in_tree() {
        return 0;
    }
    if planes
        .iter()
        .any(|plane| plane.distance_to(center) > radius)
    {
        return 0;
    }
    if distance < FULL_RATE_RANGE_M {
        1
    } else if distance < HALF_RATE_RANGE_M {
        2
    } else {
        4
    }
}

/// Root-local bounding sphere of the GLTF's visuals and whether it has an active physics collider.
fn scan(root: &Gd<Node3D>) -> (Vector3, f32, bool) {
    let mut bounds: Option<Aabb> = None;
    let mut physics_collider = false;
    scan_children(
        root.clone().upcast(),
        Transform3D::IDENTITY,
        &mut bounds,
        &mut physics_collider,
    );
    match bounds {
        Some(aabb) => (aabb.center(), aabb.size.length() * 0.5, physics_collider),
        None => (Vector3::ZERO, 1.0, physics_collider),
    }
}

fn scan_children(
    node: Gd<Node>,
    to_root: Transform3D,
    bounds: &mut Option<Aabb>,
    physics_collider: &mut bool,
) {
    for child in node.get_children().iter_shared() {
        let child_to_root = match child.clone().try_cast::<Node3D>() {
            Ok(child_3d) => to_root * child_3d.get_transform(),
            Err(_) => to_root,
        };
        if let Ok(visual) = child.clone().try_cast::<VisualInstance3D>() {
            let aabb = child_to_root * visual.get_aabb();
            *bounds = Some(match bounds.take() {
                Some(merged) => merged.merge(aabb),
                None => aabb,
            });
        }
        if let Ok(body) = child.clone().try_cast::<CollisionObject3D>() {
            if body.get_collision_layer() & CL_PHYSICS != 0 {
                *physics_collider = true;
            }
        }
        scan_children(child, child_to_root, bounds, physics_collider);
    }
}
