use std::collections::HashSet;

use crate::{
    content::{content_mapping::DclContentMappingAndUrl, content_provider::ContentProvider},
    dcl::{
        components::{
            material::{DclMaterial, DclSourceTex, DclTexture, DclUnlitMaterial},
            SceneComponentId,
        },
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    godot_classes::dcl_global::DclGlobal,
    scene_runner::scene::{MaterialItem, Scene},
};
use godot::{
    classes::{
        base_material_3d::{EmissionOperator, Feature, Flags, ShadingMode, Transparency},
        Material, MeshInstance3D, ResourceLoader, Shader, ShaderMaterial, StandardMaterial3D,
        Texture2D,
    },
    global::weakref,
    prelude::*,
};

use crate::dcl::components::proto_components::sdk::components::MaterialTransparencyMode;

/// Load the unlit alpha shader. Godot's ResourceLoader caches loaded resources internally.
fn get_unlit_alpha_shader() -> Option<Gd<Shader>> {
    ResourceLoader::singleton()
        .load("res://assets/shaders/dcl_unlit_alpha.gdshader")
        .and_then(|res| res.try_cast::<Shader>().ok())
}

pub fn update_material(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let material_component = SceneCrdtStateProtoComponents::get_material(crdt_state);
    let mut content_provider = DclGlobal::singleton().bind().get_content_provider();

    if let Some(material_dirty) = dirty_lww_components.get(&SceneComponentId::MATERIAL) {
        for entity in material_dirty {
            let new_value = material_component.get(entity);
            if new_value.is_none() {
                continue;
            }

            let new_value = new_value.unwrap();
            let dcl_material = if let Some(material) = new_value.value.as_ref() {
                material
                    .material
                    .as_ref()
                    .map(|material| DclMaterial::from_proto(material, &scene.content_mapping))
            } else {
                None
            };

            let (godot_entity_node, node_3d) = godot_dcl_scene.ensure_node_3d(entity);

            if let Some(dcl_material) = dcl_material {
                let previous_dcl_material = godot_entity_node.material.as_ref();
                if let Some(previous_dcl_material) = previous_dcl_material {
                    if previous_dcl_material.eq(&dcl_material) {
                        continue;
                    }
                }

                let existing_material = if let Some(material_item) = scene.materials.get(entity) {
                    let material_item = material_item.weak_ref.call("get_ref", &[]);

                    if material_item.is_nil() {
                        None
                    } else {
                        Some(material_item)
                    }
                } else {
                    None
                };

                // Always request texture fetches for new textures
                for tex in dcl_material.get_textures().into_iter().flatten() {
                    match &tex.source {
                        DclSourceTex::Texture(hash) => {
                            content_provider.call_deferred(
                                "fetch_texture_by_hash",
                                &[
                                    hash.to_godot().to_variant(),
                                    DclContentMappingAndUrl::from_ref(
                                        scene.content_mapping.clone(),
                                    )
                                    .to_variant(),
                                ],
                            );
                        }
                        DclSourceTex::AvatarTexture(user_id) => {
                            // Prefetch avatar texture immediately when material is created
                            content_provider.call_deferred(
                                "fetch_avatar_texture",
                                &[user_id.to_godot().to_variant()],
                            );
                        }
                        DclSourceTex::VideoTexture(_) => {
                            // Video textures are handled separately
                        }
                    }
                }

                let waiting_textures = {
                    match &dcl_material {
                        DclMaterial::Unlit(unlit) => {
                            unlit.texture.is_some() || unlit.alpha_texture.is_some()
                        }
                        DclMaterial::Pbr(pbr) => {
                            pbr.texture.is_some()
                                || pbr.bump_texture.is_some()
                                || pbr.alpha_texture.is_some()
                                || pbr.emissive_texture.is_some()
                        }
                    }
                };

                // Check if we need ShaderMaterial for unlit with alpha_texture
                // (alpha_texture requires transparency pipeline, so we use a custom shader)
                let needs_shader_material = matches!(&dcl_material, DclMaterial::Unlit(unlit)
                    if unlit.alpha_texture.is_some());

                let godot_material: Gd<Material> = if needs_shader_material {
                    // Use ShaderMaterial for unlit with separate alpha texture
                    if let Some(shader) = get_unlit_alpha_shader() {
                        let mut shader_mat = ShaderMaterial::new_gd();
                        shader_mat.set_shader(&shader);

                        // Apply shader properties
                        if let DclMaterial::Unlit(unlit) = &dcl_material {
                            apply_unlit_shader_properties(&mut shader_mat, unlit);
                        }

                        shader_mat.upcast::<Material>()
                    } else {
                        // Fallback to StandardMaterial3D if shader not found
                        let mut mat = StandardMaterial3D::new_gd();
                        apply_dcl_material_properties(&mut mat, &dcl_material);
                        mat.upcast::<Material>()
                    }
                } else if let Some(material) = existing_material {
                    // Try to reuse existing StandardMaterial3D, but if coming from ShaderMaterial
                    // (e.g., switching from unlit+alpha_texture to pbr), create a new one
                    if let Ok(mut mat) = material.try_to::<Gd<StandardMaterial3D>>() {
                        // Clear textures that are no longer present in the new material
                        // This is needed when changing from a textured material to a non-textured one
                        clear_removed_textures(&mut mat, &dcl_material);
                        apply_dcl_material_properties(&mut mat, &dcl_material);
                        mat.upcast::<Material>()
                    } else {
                        // Existing material was ShaderMaterial, create new StandardMaterial3D
                        let mut mat = StandardMaterial3D::new_gd();
                        apply_dcl_material_properties(&mut mat, &dcl_material);
                        mat.upcast::<Material>()
                    }
                } else {
                    let mut mat = StandardMaterial3D::new_gd();
                    apply_dcl_material_properties(&mut mat, &dcl_material);
                    mat.upcast::<Material>()
                };

                // Always update the MaterialItem with the new material definition
                // This is critical for texture changes to be detected and applied
                scene.materials.insert(
                    *entity,
                    MaterialItem {
                        dcl_mat: dcl_material.clone(),
                        weak_ref: weakref(&godot_material.to_variant()),
                        waiting_textures,
                        alive: true,
                    },
                );

                let mesh_renderer = node_3d.try_get_node_as::<MeshInstance3D>("MeshRenderer");
                if let Some(mut mesh_renderer) = mesh_renderer {
                    mesh_renderer.set_surface_override_material(0, &godot_material);
                }

                // Update tracked material for change detection
                godot_entity_node.material = Some(dcl_material);
            } else {
                let mesh_renderer = node_3d.try_get_node_as::<MeshInstance3D>("MeshRenderer");

                if let Some(mut mesh_renderer) = mesh_renderer {
                    mesh_renderer.call(
                        "set_surface_override_material",
                        &[0.to_variant(), Variant::nil()],
                    );
                    godot_entity_node.material.take();
                }
            }
        }

        scene.dirty_materials = true;
    }

    if scene.dirty_materials {
        let mut keep_dirty = false;
        let mut dead_materials = HashSet::with_capacity(scene.materials.capacity());
        let mut no_more_waiting_materials = HashSet::new();

        for (entity, item) in scene.materials.iter() {
            let dcl_material = item.dcl_mat.clone();
            if item.waiting_textures {
                let material_item = item.weak_ref.call("get_ref", &[]);
                if material_item.is_nil() {
                    dead_materials.insert(*entity);
                    continue;
                }

                let mut ready = true;

                match dcl_material {
                    DclMaterial::Unlit(unlit_material) => {
                        if unlit_material.alpha_texture.is_some() {
                            // Has alpha_texture - use ShaderMaterial
                            if let Ok(mut shader_mat) = material_item
                                .to::<Gd<Material>>()
                                .try_cast::<ShaderMaterial>()
                            {
                                // Albedo is optional (shader uses white default)
                                if unlit_material.texture.is_some() {
                                    ready &= check_unlit_shader_texture(
                                        "albedo_texture",
                                        &unlit_material.texture,
                                        &mut shader_mat,
                                        content_provider.bind_mut(),
                                    );
                                }
                                ready &= check_unlit_shader_texture(
                                    "alpha_texture",
                                    &unlit_material.alpha_texture,
                                    &mut shader_mat,
                                    content_provider.bind_mut(),
                                );
                            }
                        } else if unlit_material.texture.is_some() {
                            // Only albedo texture - use StandardMaterial3D (opaque pipeline)
                            let mut material = material_item.to::<Gd<StandardMaterial3D>>();
                            ready &= check_texture(
                                godot::classes::base_material_3d::TextureParam::ALBEDO,
                                &unlit_material.texture,
                                &mut material,
                                content_provider.bind_mut(),
                                scene,
                            );
                        }
                        // No textures case: nothing to load
                    }
                    DclMaterial::Pbr(pbr) => {
                        let mut material = material_item.to::<Gd<StandardMaterial3D>>();
                        ready &= check_texture(
                            godot::classes::base_material_3d::TextureParam::ALBEDO,
                            &pbr.texture,
                            &mut material,
                            content_provider.bind_mut(),
                            scene,
                        );
                        ready &= check_texture(
                            godot::classes::base_material_3d::TextureParam::NORMAL,
                            &pbr.bump_texture,
                            &mut material,
                            content_provider.bind_mut(),
                            scene,
                        );
                        ready &= check_texture(
                            godot::classes::base_material_3d::TextureParam::EMISSION,
                            &pbr.emissive_texture,
                            &mut material,
                            content_provider.bind_mut(),
                            scene,
                        );
                    }
                }

                if !ready {
                    keep_dirty = true;
                } else {
                    no_more_waiting_materials.insert(*entity);
                }
            }
        }

        for materials in no_more_waiting_materials {
            scene
                .materials
                .get_mut(&materials)
                .unwrap()
                .waiting_textures = false;
        }

        scene.materials.retain(|k, _| !dead_materials.contains(k));
        scene.dirty_materials = keep_dirty;
    }
}

/// Apply DCL material properties to an existing Godot StandardMaterial3D.
/// This modifies the material in-place, preserving shader state where possible.
pub fn apply_dcl_material_properties(
    godot_material: &mut Gd<StandardMaterial3D>,
    dcl_material: &DclMaterial,
) {
    match dcl_material {
        DclMaterial::Unlit(unlit) => {
            godot_material.set_metallic(0.0);
            godot_material.set_roughness(0.0);
            godot_material.set_specular(0.0);

            godot_material.set_shading_mode(ShadingMode::UNSHADED);
            godot_material.set_flag(Flags::ALBEDO_TEXTURE_FORCE_SRGB, true);
            // Unity ignores diffuse_color alpha for unlit materials, force alpha to 1.0
            let mut albedo_color = unlit.diffuse_color.0.to_godot().linear_to_srgb();
            albedo_color.a = 1.0;
            godot_material.set_albedo(albedo_color);

            // Apply UV offset/tiling from main texture (only main texture supports this)
            if let Some(texture) = &unlit.texture {
                godot_material.set_uv1_offset(godot::builtin::Vector3::new(
                    texture.offset.0.x,
                    texture.offset.0.y,
                    0.0,
                ));
                godot_material.set_uv1_scale(godot::builtin::Vector3::new(
                    texture.tiling.0.x,
                    texture.tiling.0.y,
                    1.0,
                ));
            } else {
                // Reset UV transform if no texture
                godot_material.set_uv1_offset(godot::builtin::Vector3::new(0.0, 0.0, 0.0));
                godot_material.set_uv1_scale(godot::builtin::Vector3::new(1.0, 1.0, 1.0));
            }

            // Handle transparency for unlit materials
            // Note: Unity ignores diffuse_color alpha for unlit materials
            if unlit.alpha_texture.is_some() || unlit.texture.is_some() {
                // Use alpha blend for smooth transparency
                godot_material.set_transparency(Transparency::ALPHA_DEPTH_PRE_PASS);
            } else {
                godot_material.set_transparency(Transparency::DISABLED);
            }
        }
        DclMaterial::Pbr(pbr) => {
            godot_material.set_metallic(pbr.metallic.0);
            godot_material.set_roughness(pbr.roughness.0);
            godot_material.set_specular(pbr.specular_intensity.0);

            godot_material.set_shading_mode(ShadingMode::PER_PIXEL);
            godot_material.set_emission(pbr.emissive_color.0.to_godot());
            godot_material.set_emission_energy_multiplier(pbr.emissive_intensity.0);
            godot_material.set_feature(Feature::EMISSION, true);

            // Use MULTIPLY operator when there's an emissive texture, ADD otherwise
            if pbr.emissive_texture.is_some() {
                godot_material.set_emission_operator(EmissionOperator::MULTIPLY);
            } else {
                godot_material.set_emission_operator(EmissionOperator::ADD);
            }

            godot_material.set_flag(Flags::ALBEDO_TEXTURE_FORCE_SRGB, true);
            godot_material.set_albedo(pbr.albedo_color.0.to_godot());

            // Apply UV offset/tiling from main texture (only main texture supports this)
            if let Some(texture) = &pbr.texture {
                godot_material.set_uv1_offset(godot::builtin::Vector3::new(
                    texture.offset.0.x,
                    texture.offset.0.y,
                    0.0,
                ));
                godot_material.set_uv1_scale(godot::builtin::Vector3::new(
                    texture.tiling.0.x,
                    texture.tiling.0.y,
                    1.0,
                ));
            } else {
                // Reset UV transform if no texture
                godot_material.set_uv1_offset(godot::builtin::Vector3::new(0.0, 0.0, 0.0));
                godot_material.set_uv1_scale(godot::builtin::Vector3::new(1.0, 1.0, 1.0));
            }

            // Handle transparency mode
            match pbr.transparency_mode {
                MaterialTransparencyMode::MtmOpaque => {
                    godot_material.set_transparency(Transparency::DISABLED);
                }
                MaterialTransparencyMode::MtmAlphaTest => {
                    godot_material.set_transparency(Transparency::ALPHA_SCISSOR);
                    godot_material.set_alpha_scissor_threshold(pbr.alpha_test.0);
                }
                MaterialTransparencyMode::MtmAlphaBlend => {
                    godot_material.set_transparency(Transparency::ALPHA_DEPTH_PRE_PASS);
                }
                MaterialTransparencyMode::MtmAlphaTestAndAlphaBlend => {
                    godot_material.set_transparency(Transparency::ALPHA_DEPTH_PRE_PASS);
                    godot_material.set_alpha_scissor_threshold(pbr.alpha_test.0);
                }
                MaterialTransparencyMode::MtmAuto => {
                    // Auto-detect: use alpha blend if albedo has transparency
                    if pbr.albedo_color.0.a < 1.0 || pbr.texture.is_some() {
                        godot_material.set_transparency(Transparency::ALPHA_DEPTH_PRE_PASS);
                    } else {
                        godot_material.set_transparency(Transparency::DISABLED);
                    }
                }
            }
        }
    }
}

/// Apply DCL unlit material properties to a ShaderMaterial.
fn apply_unlit_shader_properties(shader_mat: &mut Gd<ShaderMaterial>, unlit: &DclUnlitMaterial) {
    // Set diffuse color (Unity ignores alpha for unlit, shader only uses .rgb)
    let mut color = unlit.diffuse_color.0.to_godot();
    color.a = 1.0;
    shader_mat.set_shader_parameter("diffuse_color", &color.to_variant());

    // Set UV offset and scale from main texture
    if let Some(tex) = &unlit.texture {
        shader_mat.set_shader_parameter(
            "uv_offset",
            &Vector2::new(tex.offset.0.x, tex.offset.0.y).to_variant(),
        );
        shader_mat.set_shader_parameter(
            "uv_scale",
            &Vector2::new(tex.tiling.0.x, tex.tiling.0.y).to_variant(),
        );
    } else {
        shader_mat.set_shader_parameter("uv_offset", &Vector2::new(0.0, 0.0).to_variant());
        shader_mat.set_shader_parameter("uv_scale", &Vector2::new(1.0, 1.0).to_variant());
    }
}

/// Clear textures from a material that are no longer present in the new material definition.
/// This ensures that when a texture is set to null, it's actually removed from the Godot material.
fn clear_removed_textures(material: &mut Gd<StandardMaterial3D>, dcl_material: &DclMaterial) {
    use godot::classes::base_material_3d::TextureParam;

    match dcl_material {
        DclMaterial::Unlit(unlit) => {
            // Clear albedo texture if neither main texture nor alpha_texture is present
            if unlit.texture.is_none() && unlit.alpha_texture.is_none() {
                material.call(
                    "set_texture",
                    &[TextureParam::ALBEDO.ord().to_variant(), Variant::nil()],
                );
            }
            // Unlit materials don't have normal/emission textures
            material.call(
                "set_texture",
                &[TextureParam::NORMAL.ord().to_variant(), Variant::nil()],
            );
            material.call(
                "set_texture",
                &[TextureParam::EMISSION.ord().to_variant(), Variant::nil()],
            );
        }
        DclMaterial::Pbr(pbr) => {
            // Clear albedo texture if not present (and no alpha texture either)
            if pbr.texture.is_none() && pbr.alpha_texture.is_none() {
                material.call(
                    "set_texture",
                    &[TextureParam::ALBEDO.ord().to_variant(), Variant::nil()],
                );
            }
            // Clear normal texture if not present
            if pbr.bump_texture.is_none() {
                material.call(
                    "set_texture",
                    &[TextureParam::NORMAL.ord().to_variant(), Variant::nil()],
                );
                // Also disable the normal map feature
                material.set_feature(Feature::NORMAL_MAPPING, false);
            }
            // Clear emission texture if not present
            if pbr.emissive_texture.is_none() {
                material.call(
                    "set_texture",
                    &[TextureParam::EMISSION.ord().to_variant(), Variant::nil()],
                );
            }
        }
    }
}

/// Validates that a texture has valid GPU resources (non-zero dimensions).
/// Empty textures can cause crashes on Android due to invalid descriptor sets.
fn is_valid_texture(texture: &Gd<Texture2D>) -> bool {
    texture.get_width() > 0 && texture.get_height() > 0
}

fn check_texture(
    param: godot::classes::base_material_3d::TextureParam,
    dcl_texture: &Option<DclTexture>,
    material: &mut Gd<StandardMaterial3D>,
    mut content_provider: GdMut<ContentProvider>,
    _scene: &Scene,
) -> bool {
    if dcl_texture.is_none() {
        return true;
    }

    let dcl_texture = dcl_texture.as_ref().unwrap();

    match &dcl_texture.source {
        DclSourceTex::Texture(content_hash) => {
            if content_provider.is_resource_from_hash_loaded(content_hash.to_godot()) {
                if let Some(resource) =
                    content_provider.get_texture_from_hash(content_hash.to_godot())
                {
                    // Validate texture has actual data before using to prevent GPU crashes
                    if is_valid_texture(&resource) {
                        material.set_texture(param, &resource.upcast::<Texture2D>());
                    } else {
                        tracing::warn!(
                            "Skipping invalid texture (0x0) for hash {}, may cause rendering issues",
                            content_hash
                        );
                    }
                }
                true
            } else {
                false
            }
        }
        DclSourceTex::AvatarTexture(user_id) => {
            if content_provider.is_avatar_texture_loaded(user_id.to_godot()) {
                let texture_result = content_provider.get_avatar_texture(user_id.to_godot());
                if let Some(texture) = texture_result {
                    // Validate texture has actual data before using to prevent GPU crashes
                    if is_valid_texture(&texture) {
                        material.set_texture(param, &texture.upcast::<Texture2D>());
                    } else {
                        tracing::warn!(
                            "Skipping invalid avatar texture (0x0) for user {}, may cause rendering issues",
                            user_id
                        );
                    }
                } else {
                    // Promise was rejected (invalid user, no profile, no snapshots, etc.)
                    // Clear the texture to avoid showing stale data
                    let no_texture: Option<&Gd<Texture2D>> = None;
                    material.set_texture(param, no_texture);
                }
                true
            } else {
                // Start fetching if not already in progress (only logs once due to caching)
                content_provider.fetch_avatar_texture(user_id.to_godot());
                false
            }
        }
        DclSourceTex::VideoTexture(_video_entity_id) => {
            // Video textures need special handling:
            // - LiveKit: uses dcl_texture (ImageTexture) which is updated from Rust
            // - ExoPlayer: uses ExternalTexture from GDScript, accessed via get_backend_texture()
            //
            // We return false here to keep the material "dirty" so video textures
            // are re-applied each frame. This ensures texture changes (video frames,
            // ExoPlayer texture resize) are reflected in the material.
            //
            // The actual texture binding happens in update_video_material_textures()
            // which is called after the main material loop.
            false
        }
    }
}

/// Check and set a texture on a ShaderMaterial.
/// Returns true if ready (texture loaded), false if still loading.
fn check_unlit_shader_texture(
    param_name: &str,
    dcl_texture: &Option<DclTexture>,
    shader_mat: &mut Gd<ShaderMaterial>,
    mut content_provider: GdMut<ContentProvider>,
) -> bool {
    let Some(dcl_tex) = dcl_texture else {
        return true;
    };

    match &dcl_tex.source {
        DclSourceTex::Texture(hash) => {
            if content_provider.is_resource_from_hash_loaded(hash.to_godot()) {
                if let Some(tex) = content_provider.get_texture_from_hash(hash.to_godot()) {
                    shader_mat.set_shader_parameter(param_name, &tex.to_variant());
                }
                true
            } else {
                false
            }
        }
        DclSourceTex::AvatarTexture(user_id) => {
            if content_provider.is_avatar_texture_loaded(user_id.to_godot()) {
                if let Some(tex) = content_provider.get_avatar_texture(user_id.to_godot()) {
                    shader_mat.set_shader_parameter(param_name, &tex.to_variant());
                }
                true
            } else {
                content_provider.fetch_avatar_texture(user_id.to_godot());
                false
            }
        }
        DclSourceTex::VideoTexture(_) => {
            // Video textures not yet supported for ShaderMaterial
            false
        }
    }
}

/// Update video textures on materials.
/// This is called separately from check_texture because video textures need mutable access
/// to video_players to call get_backend_texture(), which would conflict with the material
/// iteration loop.
pub fn update_video_material_textures(scene: &mut Scene) {
    // Collect video texture bindings we need to update
    // Format: (material weak_ref, texture_param, video_entity_id)
    let mut video_texture_updates: Vec<(
        Variant,
        godot::classes::base_material_3d::TextureParam,
        crate::dcl::components::SceneEntityId,
    )> = Vec::new();

    for (_entity, item) in scene.materials.iter() {
        if !item.waiting_textures {
            continue;
        }

        let material_ref = item.weak_ref.call("get_ref", &[]);
        if material_ref.is_nil() {
            continue;
        }

        // Check each texture in the material for video textures
        let textures_to_check: Vec<(
            godot::classes::base_material_3d::TextureParam,
            &Option<DclTexture>,
        )> = match &item.dcl_mat {
            DclMaterial::Unlit(unlit) => {
                vec![(
                    godot::classes::base_material_3d::TextureParam::ALBEDO,
                    &unlit.texture,
                )]
            }
            DclMaterial::Pbr(pbr) => {
                vec![
                    (
                        godot::classes::base_material_3d::TextureParam::ALBEDO,
                        &pbr.texture,
                    ),
                    (
                        godot::classes::base_material_3d::TextureParam::NORMAL,
                        &pbr.bump_texture,
                    ),
                    (
                        godot::classes::base_material_3d::TextureParam::EMISSION,
                        &pbr.emissive_texture,
                    ),
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

    // Now apply the video textures (we can mutably borrow video_players here)
    for (material_ref, param, video_entity_id) in video_texture_updates {
        if let Some(video_player) = scene.video_players.get_mut(&video_entity_id) {
            let mut material = material_ref.to::<Gd<StandardMaterial3D>>();

            // Get current texture to check if update is needed
            // This prevents calling set_texture every frame which exhausts descriptor pools
            let current_texture = material.get_texture(param);

            // Try get_backend_texture first (works for ExoPlayer's ExternalTexture)
            let backend_texture = video_player.bind_mut().get_backend_texture();
            if let Some(texture) = backend_texture {
                // Validate texture has actual data before using to prevent GPU crashes
                if is_valid_texture(&texture) {
                    // Only set texture if it's different (compare by instance ID)
                    let needs_update = current_texture
                        .as_ref()
                        .is_none_or(|current| current.instance_id() != texture.instance_id());
                    if needs_update {
                        material.set_texture(param, &texture.upcast::<Texture2D>());
                    }
                }
            } else {
                // Fallback to dcl_texture (works for LiveKit's ImageTexture)
                if let Some(texture) = video_player.bind().get_dcl_texture() {
                    let texture_2d = texture.upcast::<Texture2D>();
                    // Validate texture has actual data before using to prevent GPU crashes
                    if is_valid_texture(&texture_2d) {
                        // Only set texture if it's different (compare by instance ID)
                        let needs_update = current_texture.as_ref().is_none_or(|current| {
                            current.instance_id() != texture_2d.instance_id()
                        });
                        if needs_update {
                            material.set_texture(param, &texture_2d);
                        }
                    }
                }
            }
        }
    }
}
