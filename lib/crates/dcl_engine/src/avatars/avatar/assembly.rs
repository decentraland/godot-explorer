//! Paces an avatar build over several frames through the frame budget: each wearable mesh moved
//! onto the skeleton is one step, and a newer build stops an older one at its next step.

use godot::classes::{MeshInstance3D, Skeleton3D};
use godot::prelude::*;

use super::gpu_ready_gate;
use crate::avatars::avatar_scene::AvatarScene;
use crate::engine::frame_budget;
use crate::godot_classes::dcl_avatar::DclAvatar;

/// Remote avatars (children of the AvatarScene) wait in line; every other avatar (local player,
/// previews, scene NPCs) has priority.
pub fn is_background(avatar: &Gd<DclAvatar>) -> bool {
    avatar
        .get_parent()
        .is_some_and(|parent| parent.try_cast::<AvatarScene>().is_ok())
}

/// Waits for this avatar's turn in the frame budget.
pub async fn budget_step(avatar: &Gd<DclAvatar>) {
    frame_budget::acquire(Some(avatar.instance_id()), !is_background(avatar)).await;
}

/// Starts a build and returns its id; any earlier build stops at its next step.
pub fn begin_build(avatar: &Gd<DclAvatar>) -> u64 {
    let build = {
        let mut handle = avatar.clone();
        let mut this = handle.bind_mut();
        match this.mesh_assembler.as_mut() {
            Some(assembler) => {
                assembler.build_id += 1;
                assembler.build_id
            }
            None => 0,
        }
    };
    gpu_ready_gate::cancel(avatar);
    frame_budget::begin_build(avatar.instance_id());
    build
}

pub fn is_current(avatar: &Gd<DclAvatar>, build: u64) -> bool {
    avatar.is_instance_valid()
        && avatar
            .bind()
            .mesh_assembler
            .as_ref()
            .is_some_and(|assembler| assembler.build_id == build)
}

pub fn end_build(avatar: &Gd<DclAvatar>) {
    frame_budget::end_build(avatar.instance_id());
    gpu_ready_gate::start(avatar);
}

/// Waits for a budget turn; false when the avatar is gone or a newer build superseded `build`.
pub async fn step(avatar: &Gd<DclAvatar>, build: u64) -> bool {
    budget_step(avatar).await;
    is_current(avatar, build)
}

/// Moves the meshes of a freshly instantiated wearable onto the base skeleton, one budget step
/// per mesh, then frees `object`. False when a newer build superseded `build`.
pub async fn attach_wearable(
    avatar: &Gd<DclAvatar>,
    mut object: Gd<Node3D>,
    name_suffix: &str,
    build: u64,
) -> bool {
    let mut attached = is_current(avatar, build);
    let skeletons = if attached {
        object.find_children("Skeleton3D")
    } else {
        Array::new()
    };
    'skeletons: for found in skeletons.iter_shared() {
        let Ok(mut wearable_skeleton) = found.try_cast::<Skeleton3D>() else {
            continue;
        };
        // Spring bones (ADR-316) and other extras must exist on the base skeleton before any mesh
        // moves, or the skins reference bone indices that do not exist there.
        {
            let mut handle = avatar.clone();
            let mut this = handle.bind_mut();
            if let Some(assembler) = this.mesh_assembler.as_mut() {
                assembler.merge_extra_bones(&wearable_skeleton);
            }
        }
        for mut child in wearable_skeleton.get_children().iter_shared() {
            if let Ok(mut mesh) = child.clone().try_cast::<MeshInstance3D>() {
                if !step(avatar, build).await {
                    attached = false;
                    break 'skeletons;
                }
                if let Some(assembler) = avatar.bind().mesh_assembler.as_ref() {
                    assembler.rebind_skin_by_name(&mut mesh, &wearable_skeleton);
                }
            }
            let Some(mut skeleton) = avatar.bind().body_shape_skeleton() else {
                attached = false;
                break 'skeletons;
            };
            wearable_skeleton.remove_child(&child);
            child.set_owner(Gd::null_arg());
            let name = format!(
                "{}{name_suffix}",
                child.get_name().to_string().to_lowercase()
            );
            child.set_name(name.as_str());
            skeleton.add_child(&child);
        }
    }
    object.queue_free();
    attached
}
