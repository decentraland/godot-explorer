//! Material creation and application for GLTF node modifiers.

use godot::{
    classes::{
        base_material_3d::TextureParam, geometry_instance_3d::ShadowCastingSetting, Material,
        MeshInstance3D, StandardMaterial3D, Texture2D,
    },
    obj::GdMut,
    prelude::*,
};

use crate::{
    content::content_mapping::DclContentMappingAndUrl,
    dcl::components::{
        material::{DclMaterial, DclSourceTex},
        proto_components::sdk::components::PbMaterial,
        SceneEntityId,
    },
    godot_classes::dcl_global::DclGlobal,
    scene_runner::{components::material::apply_dcl_material_properties, scene::Scene},
};

/// Capture original material state for a mesh instance
pub fn capture_original_materials(mesh: &Gd<MeshInstance3D>) -> Vec<Option<Gd<Material>>> {
    let surface_count = mesh.get_surface_override_material_count() as usize;
    let mut materials = Vec::with_capacity(surface_count);

    for i in 0..surface_count {
        materials.push(mesh.get_surface_override_material(i as i32));
    }

    materials
}

/// Restore original materials to a mesh instance
pub fn restore_original_materials(
    mesh: &mut Gd<MeshInstance3D>,
    materials: &[Option<Gd<Material>>],
) {
    for (i, mat) in materials.iter().enumerate() {
        if let Some(material) = mat {
            mesh.set_surface_override_material(i as i32, material);
        } else {
            // Use Variant::nil() to properly clear the override
            mesh.call(
                "set_surface_override_material",
                &[(i as i32).to_variant(), Variant::nil()],
            );
        }
    }
}

/// Get or create a material for modification.
/// Tries to reuse existing StandardMaterial3D from the mesh, duplicating it if found.
/// Falls back to creating a new StandardMaterial3D if none exists or type doesn't match.
fn get_or_create_material(mesh: &Gd<MeshInstance3D>, surface_idx: i32) -> Gd<StandardMaterial3D> {
    // First check if there's already an override material we can reuse
    if let Some(override_mat) = mesh.get_surface_override_material(surface_idx) {
        if let Ok(std_mat) = override_mat.try_cast::<StandardMaterial3D>() {
            // Already have a StandardMaterial3D override, reuse it directly
            return std_mat;
        }
    }

    // Try to get the active material (from mesh resource) and duplicate it
    if let Some(active_mat) = mesh.get_active_material(surface_idx) {
        if let Ok(std_mat) = active_mat.try_cast::<StandardMaterial3D>() {
            // Duplicate to avoid modifying the shared resource
            if let Some(duplicated) = std_mat.duplicate() {
                if let Ok(dup_std) = duplicated.try_cast::<StandardMaterial3D>() {
                    return dup_std;
                }
            }
        }
    }

    // No compatible material found, create a new one
    StandardMaterial3D::new_gd()
}

/// Apply a material modifier to a mesh instance
/// If surface_index is Some, only applies to that surface; otherwise applies to all surfaces.
/// Returns the DclMaterial and Godot material if textures need loading.
///
/// This function reuses existing materials when possible to reduce Vulkan descriptor pool usage.
pub fn apply_material_to_mesh(
    mesh: &mut Gd<MeshInstance3D>,
    material: &PbMaterial,
    content_mapping: &crate::content::content_mapping::ContentMappingAndUrlRef,
    surface_index: Option<i32>,
) -> Option<(DclMaterial, Gd<StandardMaterial3D>)> {
    let mat = material.material.as_ref()?;
    let dcl_material = DclMaterial::from_proto(mat, content_mapping);

    // Request texture fetches
    let mut content_provider = DclGlobal::singleton().bind().get_content_provider();
    for tex in dcl_material.get_textures().into_iter().flatten() {
        if let DclSourceTex::Texture(hash) = &tex.source {
            content_provider.call_deferred(
                "fetch_texture_by_hash",
                &[
                    hash.to_godot().to_variant(),
                    DclContentMappingAndUrl::from_ref(content_mapping.clone()).to_variant(),
                ],
            );
        }
    }

    // Track the last material for texture loading (we'll return this one)
    let mut result_material: Option<Gd<StandardMaterial3D>> = None;

    let material_type = match &dcl_material {
        DclMaterial::Unlit(_) => "Unlit",
        DclMaterial::Pbr(_) => "PBR",
    };

    // Apply to specified surface(s)
    if let Some(idx) = surface_index {
        // Apply to specific surface only
        if idx < mesh.get_surface_override_material_count() {
            tracing::debug!(
                "GLTF modifier: applying {} material to surface {}",
                material_type,
                idx
            );
            let mut godot_material = get_or_create_material(mesh, idx);
            apply_dcl_material_properties(&mut godot_material, &dcl_material);
            mesh.set_surface_override_material(idx, &godot_material.clone().upcast::<Material>());
            result_material = Some(godot_material);
        }
    } else {
        // Apply to all surfaces
        let surface_count = mesh.get_surface_override_material_count();
        tracing::debug!(
            "GLTF modifier: applying {} material to all {} surfaces",
            material_type,
            surface_count
        );
        for i in 0..surface_count {
            let mut godot_material = get_or_create_material(mesh, i);
            apply_dcl_material_properties(&mut godot_material, &dcl_material);
            mesh.set_surface_override_material(i, &godot_material.clone().upcast::<Material>());
            // Keep reference to last material for texture loading
            result_material = Some(godot_material);
        }
    }

    // Check if we need to wait for textures
    let has_textures = match &dcl_material {
        DclMaterial::Unlit(unlit) => unlit.texture.is_some(),
        DclMaterial::Pbr(pbr) => {
            pbr.texture.is_some()
                || pbr.bump_texture.is_some()
                || pbr.alpha_texture.is_some()
                || pbr.emissive_texture.is_some()
        }
    };

    if has_textures {
        result_material.map(|m| (dcl_material, m))
    } else {
        None
    }
}

/// Apply shadow casting modifier to a mesh instance
/// If surface_index is Some, this is a per-surface modifier (but shadows apply to whole mesh)
pub fn apply_shadow_to_mesh(
    mesh: &mut Gd<MeshInstance3D>,
    cast_shadows: bool,
    _surface_index: Option<i32>,
) {
    // Note: Shadow casting is per-mesh, not per-surface, so surface_index is ignored
    let setting = if cast_shadows {
        ShadowCastingSetting::ON
    } else {
        ShadowCastingSetting::OFF
    };
    mesh.set_cast_shadows_setting(setting);
}

/// Check and apply pending textures for modifier materials
pub fn update_modifier_textures(scene: &mut Scene) {
    // Early exit if no states have pending materials
    let has_pending = scene
        .gltf_node_modifier_states
        .values()
        .any(|state| !state.pending_materials.is_empty());

    if !has_pending {
        return;
    }

    let mut content_provider = DclGlobal::singleton().bind().get_content_provider();

    for state in scene.gltf_node_modifier_states.values_mut() {
        if state.pending_materials.is_empty() {
            continue;
        }

        // Remove entries where material is no longer valid
        state.pending_materials.retain(|_, item| {
            if !item.waiting_textures {
                return false;
            }

            // Check if material is still valid
            let material_variant = item.weak_ref.call("get_ref", &[]);
            if material_variant.is_nil() {
                return false;
            }

            let Ok(material) = material_variant.try_to::<Gd<StandardMaterial3D>>() else {
                return false;
            };

            // Try to apply textures
            let all_loaded =
                check_and_apply_textures(&item.dcl_material, material, content_provider.bind_mut());
            item.waiting_textures = !all_loaded;

            // Keep if still waiting
            item.waiting_textures
        });
    }
}

/// Check if textures are loaded and apply them to the material
fn check_and_apply_textures(
    dcl_material: &DclMaterial,
    mut material: Gd<StandardMaterial3D>,
    mut content_provider: GdMut<crate::content::content_provider::ContentProvider>,
) -> bool {
    let mut all_loaded = true;

    match dcl_material {
        DclMaterial::Unlit(unlit) => {
            if !check_texture(
                TextureParam::ALBEDO,
                &unlit.texture,
                &mut material,
                &mut content_provider,
            ) {
                all_loaded = false;
            }
        }
        DclMaterial::Pbr(pbr) => {
            if !check_texture(
                TextureParam::ALBEDO,
                &pbr.texture,
                &mut material,
                &mut content_provider,
            ) {
                all_loaded = false;
            }
            if !check_texture(
                TextureParam::NORMAL,
                &pbr.bump_texture,
                &mut material,
                &mut content_provider,
            ) {
                all_loaded = false;
            }
            if !check_texture(
                TextureParam::EMISSION,
                &pbr.emissive_texture,
                &mut material,
                &mut content_provider,
            ) {
                all_loaded = false;
            }
            // Alpha texture uses the same slot as albedo in Godot
            if pbr.alpha_texture.is_some()
                && pbr.texture.is_none()
                && !check_texture(
                    TextureParam::ALBEDO,
                    &pbr.alpha_texture,
                    &mut material,
                    &mut content_provider,
                )
            {
                all_loaded = false;
            }
        }
    }

    all_loaded
}

/// Check if a single texture is loaded and apply it
fn check_texture(
    param: TextureParam,
    dcl_texture: &Option<crate::dcl::components::material::DclTexture>,
    material: &mut Gd<StandardMaterial3D>,
    content_provider: &mut GdMut<crate::content::content_provider::ContentProvider>,
) -> bool {
    let Some(dcl_texture) = dcl_texture else {
        return true;
    };

    match &dcl_texture.source {
        DclSourceTex::Texture(content_hash) => {
            if content_provider.is_resource_from_hash_loaded(content_hash.to_godot()) {
                if let Some(resource) =
                    content_provider.get_texture_from_hash(content_hash.to_godot())
                {
                    material.set_texture(param, &resource.upcast::<Texture2D>());
                }
                true
            } else {
                false
            }
        }
        DclSourceTex::VideoTexture(_) => {
            // Video textures need to be updated every frame
            // Return false to keep the material in pending state
            // Actual texture binding happens in update_modifier_video_textures()
            false
        }
        DclSourceTex::AvatarTexture(_) => {
            // Avatar textures not supported in GltfNodeModifiers
            true
        }
    }
}

/// Update video textures on modifier materials.
/// This is called separately because video textures need mutable access to video_players.
pub fn update_modifier_video_textures(scene: &mut Scene) {
    // Early exit if no states have pending materials
    let has_pending = scene
        .gltf_node_modifier_states
        .values()
        .any(|state| !state.pending_materials.is_empty());

    if !has_pending {
        return;
    }

    // Collect video texture bindings we need to update
    // Format: (material weak_ref, texture_param, video_entity_id)
    let mut video_texture_updates: Vec<(Variant, TextureParam, SceneEntityId)> = Vec::new();

    for state in scene.gltf_node_modifier_states.values() {
        if state.pending_materials.is_empty() {
            continue;
        }

        for item in state.pending_materials.values() {
            if !item.waiting_textures {
                continue;
            }

            let material_ref = item.weak_ref.call("get_ref", &[]);
            if material_ref.is_nil() {
                continue;
            }

            // Collect video textures based on material type
            let textures_to_check: Vec<(
                TextureParam,
                &Option<crate::dcl::components::material::DclTexture>,
            )> = match &item.dcl_material {
                DclMaterial::Unlit(unlit) => {
                    vec![(TextureParam::ALBEDO, &unlit.texture)]
                }
                DclMaterial::Pbr(pbr) => {
                    vec![
                        (TextureParam::ALBEDO, &pbr.texture),
                        (TextureParam::NORMAL, &pbr.bump_texture),
                        (TextureParam::EMISSION, &pbr.emissive_texture),
                    ]
                }
            };

            for (param, dcl_texture) in textures_to_check {
                if let Some(tex) = dcl_texture {
                    if let DclSourceTex::VideoTexture(video_entity_id) = &tex.source {
                        video_texture_updates.push((material_ref.clone(), param, *video_entity_id));
                    }
                }
            }
        }
    }

    // Now apply the video textures (we can mutably borrow video_players here)
    for (material_ref, param, video_entity_id) in video_texture_updates {
        if let Some(video_player) = scene.video_players.get_mut(&video_entity_id) {
            let mut material = material_ref.to::<Gd<StandardMaterial3D>>();

            // Try get_backend_texture first (works for ExoPlayer's ExternalTexture)
            let backend_texture = video_player.bind_mut().get_backend_texture();
            if let Some(texture) = backend_texture {
                material.set_texture(param, &texture.upcast::<Texture2D>());
            } else {
                // Fallback to dcl_texture (works for LiveKit's ImageTexture)
                if let Some(texture) = video_player.bind().get_dcl_texture() {
                    material.set_texture(param, &texture.upcast::<Texture2D>());
                }
            }
        }
    }
}
