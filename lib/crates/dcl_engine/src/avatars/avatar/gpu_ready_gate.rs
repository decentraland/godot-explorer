//! Keeps a freshly assembled remote avatar drawn but fully dithered out until the renderer reports
//! every mesh drawable, then fades it in as a whole. The dither is the `own_fade` instance uniform:
//! same material, pass and pipelines as the final look, so the hold itself warms them.

use godot::classes::MeshInstance3D;
use godot::prelude::*;

use super::assembly::is_background;
use crate::avatars::proximity_fade::AvatarProximityFade;
use crate::engine::engine_fork;
use crate::engine::local_task::{next_frame, spawn_local};
use crate::godot_classes::dcl_avatar::DclAvatar;

const TIMEOUT_SECONDS: f64 = 3.0;
const FADE_IN_SECONDS: f64 = 0.2;

fn fade_of(avatar: &Gd<DclAvatar>) -> Option<Gd<AvatarProximityFade>> {
    avatar
        .get_node_or_null("AvatarProximityFade")?
        .try_cast()
        .ok()
}

fn bump_generation(avatar: &Gd<DclAvatar>) -> Option<u64> {
    let mut handle = avatar.clone();
    let mut this = handle.bind_mut();
    let assembler = this.mesh_assembler.as_mut()?;
    assembler.gate_generation += 1;
    Some(assembler.gate_generation)
}

fn is_current(avatar: &Gd<DclAvatar>, generation: u64) -> bool {
    avatar.is_instance_valid()
        && avatar
            .bind()
            .mesh_assembler
            .as_ref()
            .is_some_and(|assembler| assembler.gate_generation == generation)
}

/// Stops a running gate and shows the avatar; a new build begins.
pub fn cancel(avatar: &Gd<DclAvatar>) {
    bump_generation(avatar);
    if let Some(mut fade) = fade_of(avatar) {
        fade.bind_mut().set_hold(0.0);
    }
}

pub fn start(avatar: &Gd<DclAvatar>) {
    let Some(generation) = bump_generation(avatar) else {
        return;
    };
    if !is_background(avatar) || !engine_fork::has_draw_ready() {
        return;
    }
    let Some(mut fade) = fade_of(avatar) else {
        return;
    };
    fade.bind_mut().set_hold(1.0);
    spawn_local(run(avatar.clone(), fade, generation));
}

async fn run(avatar: Gd<DclAvatar>, mut fade: Gd<AvatarProximityFade>, generation: u64) {
    let mut waited = 0.0;
    loop {
        next_frame().await;
        if !is_current(&avatar, generation) {
            return;
        }
        let meshes = drawn_meshes(&avatar);
        // The timeout only counts while the meshes would be drawn (on screen, not LOD-hidden).
        if !expects_draw(&avatar, &meshes) {
            continue;
        }
        let pending = meshes
            .iter()
            .filter(|mesh| !engine_fork::instance_geometry_is_draw_ready(mesh.get_instance()))
            .count();
        if pending > 0 {
            waited += avatar.get_process_delta_time();
            if waited < TIMEOUT_SECONDS {
                continue;
            }
        }
        break;
    }

    let mut elapsed = 0.0;
    while elapsed < FADE_IN_SECONDS {
        fade.bind_mut()
            .set_hold((1.0 - elapsed / FADE_IN_SECONDS) as f32);
        next_frame().await;
        if !is_current(&avatar, generation) {
            return;
        }
        elapsed += avatar.get_process_delta_time();
    }
    fade.bind_mut().set_hold(0.0);
}

fn expects_draw(avatar: &Gd<DclAvatar>, meshes: &[Gd<MeshInstance3D>]) -> bool {
    !meshes.is_empty() && avatar.is_visible_in_tree() && avatar.bind().lod.on_screen
}

fn drawn_meshes(avatar: &Gd<DclAvatar>) -> Vec<Gd<MeshInstance3D>> {
    let Some(skeleton) = avatar.bind().body_shape_skeleton() else {
        return Vec::new();
    };
    skeleton
        .get_children()
        .iter_shared()
        .filter_map(|child| child.try_cast::<MeshInstance3D>().ok())
        .filter(|mesh| mesh.is_visible_in_tree() && mesh.get_mesh().is_some())
        .collect()
}
