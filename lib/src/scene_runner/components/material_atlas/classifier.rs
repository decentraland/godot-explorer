//! Per-MeshInstance classifier for the material atlas.
//!
//! Skip rules mirror the textureless merger but accept textured meshes.

use godot::classes::base_material_3d::{DiffuseMode, Feature, Flags, SpecularMode, TextureParam};
use godot::classes::{
    AnimationPlayer, BaseMaterial3D, MeshInstance3D, Node, ShaderMaterial, Skeleton3D, Texture2D,
};
use godot::prelude::*;

use crate::dcl::components::SceneEntityId;
use crate::godot_classes::dcl_avatar::DclAvatar;
use crate::scene_runner::scene::Scene;

use super::atlas::LayerParams;

#[derive(Debug, Clone, Copy)]
pub enum SkipReason {
    NoMesh,
    NotVisible,
    ColliderName,
    SkeletonAncestor,
    AnimationPlayerAncestor,
    AvatarAncestor,
    BlendShapes,
    ShaderMaterial,
    NoMaterial,
    MultiSurface,
    HasTween,
    HasModifier,
    UnsupportedTransparency,
    UnsupportedFeature,
}

#[derive(Debug, Clone)]
pub enum Classification {
    Mergeable {
        albedo_texture: Option<Gd<Texture2D>>,
        params: LayerParams,
        transparency: i32,
        cull_mode: i32,
    },
    Skip(SkipReason),
}

/// Classify every surface of an MI individually. Returns one Classification
/// per surface; MI-level skip reasons (visibility, ancestors, blend shapes,
/// missing mesh) cause a single-element vec applied to the whole MI by the
/// caller. The single-surface `classify()` below is the v1 entry point and
/// stays for callers that haven't migrated yet.
pub fn classify_per_surface(
    mi: &Gd<MeshInstance3D>,
    scene: &Scene,
    entity: SceneEntityId,
) -> Vec<Classification> {
    if let Some(skip) = mi_level_skip(mi, scene, entity) {
        return vec![Classification::Skip(skip)];
    }

    let Some(mesh) = mi.get_mesh() else {
        return vec![Classification::Skip(SkipReason::NoMesh)];
    };
    if mi.get_blend_shape_count() > 0 {
        return vec![Classification::Skip(SkipReason::BlendShapes)];
    }

    let surface_count = mesh.get_surface_count();
    if surface_count == 0 {
        return vec![Classification::Skip(SkipReason::NoMesh)];
    }

    (0..surface_count)
        .map(|i| {
            let material = mi
                .get_surface_override_material(i)
                .or_else(|| mesh.surface_get_material(i));
            classify_material(material)
        })
        .collect()
}

fn mi_level_skip(
    mi: &Gd<MeshInstance3D>,
    scene: &Scene,
    entity: SceneEntityId,
) -> Option<SkipReason> {
    if scene.tweens.contains_key(&entity) {
        return Some(SkipReason::HasTween);
    }
    if scene
        .gltf_node_modifier_states
        .get(&entity)
        .is_some_and(|state| !state.applied_paths.is_empty())
    {
        return Some(SkipReason::HasModifier);
    }
    if !mi.is_visible_in_tree() {
        return Some(SkipReason::NotVisible);
    }
    if mi
        .get_name()
        .to_string()
        .to_lowercase()
        .contains("collider")
    {
        return Some(SkipReason::ColliderName);
    }
    let mut current: Option<Gd<Node>> = Some(mi.clone().upcast());
    while let Some(node) = current {
        if node.clone().try_cast::<Skeleton3D>().is_ok() {
            return Some(SkipReason::SkeletonAncestor);
        }
        if node.clone().try_cast::<AnimationPlayer>().is_ok() {
            return Some(SkipReason::AnimationPlayerAncestor);
        }
        if node.clone().try_cast::<DclAvatar>().is_ok() {
            return Some(SkipReason::AvatarAncestor);
        }
        current = node.get_parent();
    }
    None
}

fn classify_material(material: Option<Gd<godot::classes::Material>>) -> Classification {
    let Some(material) = material else {
        return Classification::Skip(SkipReason::NoMaterial);
    };
    if material.clone().try_cast::<ShaderMaterial>().is_ok() {
        return Classification::Skip(SkipReason::ShaderMaterial);
    }
    let Ok(base) = material.try_cast::<BaseMaterial3D>() else {
        return Classification::Skip(SkipReason::NoMaterial);
    };
    let hard_unsupported = [
        Feature::REFRACTION,
        Feature::SUBSURFACE_SCATTERING,
        Feature::DETAIL,
        Feature::ANISOTROPY,
        Feature::CLEARCOAT,
        Feature::BACKLIGHT,
        Feature::HEIGHT_MAPPING,
    ];
    for feature in hard_unsupported {
        if base.get_feature(feature) {
            return Classification::Skip(SkipReason::UnsupportedFeature);
        }
    }
    if base.get_diffuse_mode() != DiffuseMode::BURLEY {
        return Classification::Skip(SkipReason::UnsupportedFeature);
    }
    if base.get_specular_mode() != SpecularMode::SCHLICK_GGX {
        return Classification::Skip(SkipReason::UnsupportedFeature);
    }
    if base.get_flag(Flags::ALBEDO_FROM_VERTEX_COLOR) {
        return Classification::Skip(SkipReason::UnsupportedFeature);
    }
    let transparency = base.get_transparency().ord();
    if transparency != 0 && transparency != 2 {
        return Classification::Skip(SkipReason::UnsupportedTransparency);
    }
    let albedo_texture = base.get_texture(TextureParam::ALBEDO);
    let params = LayerParams {
        albedo_factor: base.get_albedo(),
        metallic: base.get_metallic(),
        roughness: base.get_roughness(),
        emissive_intensity: if base.get_feature(Feature::EMISSION) {
            base.get_emission_energy_multiplier()
        } else {
            0.0
        },
        alpha_cutoff: if transparency == 2 {
            base.get_alpha_scissor_threshold()
        } else {
            0.5
        },
    };
    Classification::Mergeable {
        albedo_texture,
        params,
        transparency,
        cull_mode: base.get_cull_mode().ord(),
    }
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

    let Ok(base) = material.try_cast::<BaseMaterial3D>() else {
        return Classification::Skip(SkipReason::NoMaterial);
    };

    // v1 atlas shader only samples albedo. We tolerate (i.e. drop visually)
    // normal/ORM/AO/emission textures so the classifier accepts more
    // materials. The albedo + scalar params still replicate; the missing
    // texture maps just degrade visually until we add their atlases.
    // Hard rejects below are features that change shading behavior so
    // dramatically (refraction, subsurface, clearcoat) that ignoring them
    // produces unrecognizable visuals.
    let hard_unsupported = [
        Feature::REFRACTION,
        Feature::SUBSURFACE_SCATTERING,
        Feature::DETAIL,
        Feature::ANISOTROPY,
        Feature::CLEARCOAT,
        Feature::BACKLIGHT,
        Feature::HEIGHT_MAPPING,
    ];
    for feature in hard_unsupported {
        if base.get_feature(feature) {
            return Classification::Skip(SkipReason::UnsupportedFeature);
        }
    }

    // We bind albedo + scalar params; non-default specular/diffuse/etc. modes
    // would diverge visually. Accept only the defaults.
    if base.get_diffuse_mode() != DiffuseMode::BURLEY {
        return Classification::Skip(SkipReason::UnsupportedFeature);
    }
    if base.get_specular_mode() != SpecularMode::SCHLICK_GGX {
        return Classification::Skip(SkipReason::UnsupportedFeature);
    }
    if base.get_flag(Flags::ALBEDO_FROM_VERTEX_COLOR) {
        return Classification::Skip(SkipReason::UnsupportedFeature);
    }

    // Only opaque + alpha_scissor in v1; alpha-blend needs sorting that
    // breaks identity-based batching anyway.
    let transparency = base.get_transparency().ord();
    if transparency != 0 && transparency != 2 {
        return Classification::Skip(SkipReason::UnsupportedTransparency);
    }

    let albedo_texture = base.get_texture(TextureParam::ALBEDO);

    let params = LayerParams {
        albedo_factor: base.get_albedo(),
        metallic: base.get_metallic(),
        roughness: base.get_roughness(),
        emissive_intensity: if base.get_feature(Feature::EMISSION) {
            base.get_emission_energy_multiplier()
        } else {
            0.0
        },
        alpha_cutoff: if transparency == 2 {
            base.get_alpha_scissor_threshold()
        } else {
            0.5
        },
    };

    Classification::Mergeable {
        albedo_texture,
        params,
        transparency,
        cull_mode: base.get_cull_mode().ord(),
    }
}
