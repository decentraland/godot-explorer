//! Per-MeshInstance classifier — decides whether a mesh is mergeable
//! (textureless `BaseMaterial3D`, single surface, no special features) or
//! must stay standalone.
//!
//! Skip-rule order is "cheapest first": scene-state checks (Tween / modifier)
//! beat Godot-side checks because they don't touch a Gd<Node>.

use godot::classes::base_material_3d::TextureParam;
use godot::classes::{
    AnimationPlayer, BaseMaterial3D, MeshInstance3D, Node, ShaderMaterial, Skeleton3D,
};
use godot::prelude::*;

use crate::dcl::components::SceneEntityId;
use crate::godot_classes::dcl_avatar::DclAvatar;
use crate::scene_runner::scene::Scene;

#[derive(Debug, Clone, Copy)]
pub enum SkipReason {
    NoMesh,
    NotVisible,
    ColliderName,
    SkeletonAncestor,
    AnimationPlayerAncestor,
    AvatarAncestor,
    BlendShapes,
    Textured,
    ShaderMaterial,
    NoMaterial,
    MultiSurface,
    HasTween,
    HasModifier,
}

#[derive(Debug, Clone)]
pub enum Classification {
    Mergeable {
        albedo_color: Color,
        transparency: i32,
        cull_mode: i32,
    },
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

    if !mi.is_visible_in_tree() {
        return Classification::Skip(SkipReason::NotVisible);
    }

    if mi
        .get_name()
        .to_string()
        .to_lowercase()
        .contains("collider")
    {
        return Classification::Skip(SkipReason::ColliderName);
    }

    let mut current: Option<Gd<Node>> = Some(mi.clone().upcast());
    while let Some(node) = current {
        if node.clone().try_cast::<Skeleton3D>().is_ok() {
            return Classification::Skip(SkipReason::SkeletonAncestor);
        }
        if node.clone().try_cast::<AnimationPlayer>().is_ok() {
            return Classification::Skip(SkipReason::AnimationPlayerAncestor);
        }
        if node.clone().try_cast::<DclAvatar>().is_ok() {
            return Classification::Skip(SkipReason::AvatarAncestor);
        }
        current = node.get_parent();
    }

    let Some(mesh) = mi.get_mesh() else {
        return Classification::Skip(SkipReason::NoMesh);
    };

    if mi.get_blend_shape_count() > 0 {
        return Classification::Skip(SkipReason::BlendShapes);
    }

    if mesh.get_surface_count() != 1 {
        return Classification::Skip(SkipReason::MultiSurface);
    }

    let material = mi
        .get_active_material(0)
        .or_else(|| mi.get_surface_override_material(0))
        .or_else(|| mesh.surface_get_material(0));

    let Some(material) = material else {
        return Classification::Skip(SkipReason::NoMaterial);
    };

    if material.clone().try_cast::<ShaderMaterial>().is_ok() {
        return Classification::Skip(SkipReason::ShaderMaterial);
    }

    let Ok(base_material) = material.try_cast::<BaseMaterial3D>() else {
        return Classification::Skip(SkipReason::NoMaterial);
    };

    let max_param = TextureParam::MAX.ord();
    for ord in 0..max_param {
        let param = TextureParam::from_ord(ord);
        if base_material.get_texture(param).is_some() {
            return Classification::Skip(SkipReason::Textured);
        }
    }

    Classification::Mergeable {
        albedo_color: base_material.get_albedo(),
        transparency: base_material.get_transparency().ord(),
        cull_mode: base_material.get_cull_mode().ord(),
    }
}
