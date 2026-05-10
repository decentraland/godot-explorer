//! Eligibility for auto-occluder generation.

use godot::classes::base_material_3d::Transparency;
use godot::classes::{BaseMaterial3D, MeshInstance3D, Node};
use godot::prelude::*;

use crate::godot_classes::dcl_avatar::DclAvatar;

/// Below this AABB diagonal (meters), a mesh isn't worth being an occluder.
pub const MIN_AABB_DIAG_M: f32 = 5.0;
/// If smallest AABB axis < (THIN_RATIO × largest), the mesh is too flat to
/// reliably occlude — a thin wall could false-cull on either side.
pub const THIN_RATIO: f32 = 0.15;

#[derive(Debug, Clone, Copy)]
pub enum SkipReason {
    NoMesh,
    NotVisible,
    AvatarAncestor,
    HudOrUi,
    TooSmall,
    TooThin,
    Transparent,
    AlreadyOccluded,
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

    let Some(mesh) = mi.get_mesh() else {
        return Classification::Skip(SkipReason::NoMesh);
    };

    if mi.has_meta("dcl_occluder_added") {
        return Classification::Skip(SkipReason::AlreadyOccluded);
    }

    if mi.get_layer_mask() != 1 {
        return Classification::Skip(SkipReason::HudOrUi);
    }

    let mut current: Option<Gd<Node>> = Some(mi.clone().upcast());
    while let Some(node) = current {
        if node.clone().try_cast::<DclAvatar>().is_ok() {
            return Classification::Skip(SkipReason::AvatarAncestor);
        }
        current = node.get_parent();
    }

    let aabb = mesh.get_aabb();
    let diag =
        (aabb.size.x * aabb.size.x + aabb.size.y * aabb.size.y + aabb.size.z * aabb.size.z).sqrt();
    if diag < MIN_AABB_DIAG_M {
        return Classification::Skip(SkipReason::TooSmall);
    }

    let max_axis = aabb.size.x.max(aabb.size.y).max(aabb.size.z);
    let min_axis = aabb.size.x.min(aabb.size.y).min(aabb.size.z);
    if max_axis > 0.0 && min_axis / max_axis < THIN_RATIO {
        return Classification::Skip(SkipReason::TooThin);
    }

    // Transparent / alpha-tested materials don't fully occlude — even if
    // their geometry is solid the engine wouldn't draw what's behind anyway,
    // but the conservative thing is to skip them as occluders.
    let material = mi
        .get_active_material(0)
        .or_else(|| mi.get_surface_override_material(0))
        .or_else(|| mesh.surface_get_material(0));
    if let Some(mat) = material {
        if let Ok(base) = mat.try_cast::<BaseMaterial3D>() {
            if base.get_transparency() != Transparency::DISABLED {
                return Classification::Skip(SkipReason::Transparent);
            }
        }
    }

    Classification::Eligible
}
