//! Per-MeshInstance classifier for auto-distance-cull.

use godot::classes::{MeshInstance3D, Node};
use godot::prelude::*;

use crate::godot_classes::dcl_avatar::DclAvatar;

#[derive(Debug, Clone, Copy)]
pub enum SkipReason {
    NoMesh,
    AvatarAncestor,
    AlreadyRangeSet,
    NotVisible,
    HudOrUi,
}

#[derive(Debug, Clone, Copy)]
pub enum Classification {
    Eligible,
    Skip(SkipReason),
}

pub fn classify(mi: &Gd<MeshInstance3D>) -> Classification {
    if !mi.is_visible_in_tree() {
        return Classification::Skip(SkipReason::NotVisible);
    }

    let Some(_) = mi.get_mesh() else {
        return Classification::Skip(SkipReason::NoMesh);
    };

    if mi.get_visibility_range_end() > 0.0 {
        return Classification::Skip(SkipReason::AlreadyRangeSet);
    }

    // Avatars route through their own LOD/impostor pipeline.
    let mut current: Option<Gd<Node>> = Some(mi.clone().upcast());
    while let Some(node) = current {
        if node.clone().try_cast::<DclAvatar>().is_ok() {
            return Classification::Skip(SkipReason::AvatarAncestor);
        }
        current = node.get_parent();
    }

    // HUD/UI MeshInstance3Ds shouldn't get distance-culled — they live in
    // a SubViewport at fixed projection. We can't easily detect that here
    // without traversing parents, but render layers help: in DCL, scene
    // meshes use the default layer (1). HUD layers are 2+.
    let layers = mi.get_layer_mask();
    if layers != 1 {
        return Classification::Skip(SkipReason::HudOrUi);
    }

    Classification::Eligible
}
