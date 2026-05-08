//! Per-MeshInstance classifier for the mesh-LOD pass.

use godot::classes::{AnimationPlayer, ArrayMesh, MeshInstance3D, Node, Skeleton3D};
use godot::prelude::*;

use crate::dcl::components::SceneEntityId;
use crate::godot_classes::dcl_avatar::DclAvatar;
use crate::scene_runner::scene::Scene;

/// Surfaces with fewer indices than this are not worth running through
/// `generate_lods` — the meshoptimizer simplifier needs enough triangles
/// to find redundant ones, and the per-mesh fixed cost dominates the
/// payoff when there's nothing to decimate.
pub const MIN_INDICES_FOR_LOD: i32 = 256;

#[derive(Debug, Clone, Copy)]
pub enum SkipReason {
    NoMesh,
    BlendShapes,
    AvatarAncestor,
    SkinnedAncestor,
    AlreadyHasLods,
    TooSmall,
    HasTween,
    HasModifier,
}

#[derive(Debug, Clone, Copy)]
pub enum Classification {
    Eligible,
    Skip(SkipReason),
}

pub fn classify(mi: &Gd<MeshInstance3D>, scene: &Scene, entity: SceneEntityId) -> Classification {
    if scene.tweens.contains_key(&entity) {
        return Classification::Skip(SkipReason::HasTween);
    }
    if scene
        .gltf_node_modifier_states
        .get(&entity)
        .is_some_and(|state| !state.applied_paths.is_empty())
    {
        return Classification::Skip(SkipReason::HasModifier);
    }

    // Skinned + avatar meshes need bone-aware LOD generation; skip until
    // we wire that path up.
    let mut current: Option<Gd<Node>> = Some(mi.clone().upcast());
    while let Some(node) = current {
        if node.clone().try_cast::<DclAvatar>().is_ok() {
            return Classification::Skip(SkipReason::AvatarAncestor);
        }
        if node.clone().try_cast::<Skeleton3D>().is_ok() {
            return Classification::Skip(SkipReason::SkinnedAncestor);
        }
        if node.clone().try_cast::<AnimationPlayer>().is_ok() {
            // AnimationPlayer alone is fine for LOD, but combined with a
            // skin it isn't — and we already covered the skin case above.
            // Letting AnimationPlayer through means animated transforms on
            // static meshes still get LODs.
        }
        current = node.get_parent();
    }

    let Some(mesh) = mi.get_mesh() else {
        return Classification::Skip(SkipReason::NoMesh);
    };

    if mi.get_blend_shape_count() > 0 {
        return Classification::Skip(SkipReason::BlendShapes);
    }

    // ImporterMesh-LOD only knows about ArrayMesh-shaped data. PrimitiveMesh
    // (BoxMesh / SphereMesh / etc.) generates its own geometry on demand and
    // already exposes a built-in LOD curve via the engine, so we leave them
    // alone.
    let Ok(array_mesh) = mesh.try_cast::<ArrayMesh>() else {
        return Classification::Skip(SkipReason::NoMesh);
    };

    let surface_count = array_mesh.get_surface_count();
    if surface_count == 0 {
        return Classification::Skip(SkipReason::NoMesh);
    }

    // Bail before walking all surfaces if we already baked this mesh (or
    // it shipped with LODs — same meta flag covers both cases since the
    // baker only sets it on outputs of `generate_lods`).
    if array_mesh.has_meta("dcl_mesh_lod_baked") {
        return Classification::Skip(SkipReason::AlreadyHasLods);
    }

    let mut total_indices: i32 = 0;
    for s in 0..surface_count {
        let arrays = array_mesh.surface_get_arrays(s);
        if let Ok(idx) = arrays
            .at(godot::classes::mesh::ArrayType::INDEX.ord() as usize)
            .try_to::<PackedInt32Array>()
        {
            total_indices = total_indices.saturating_add(idx.len() as i32);
        }
    }

    if total_indices < MIN_INDICES_FOR_LOD {
        return Classification::Skip(SkipReason::TooSmall);
    }

    Classification::Eligible
}
