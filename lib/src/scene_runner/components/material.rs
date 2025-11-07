use std::collections::HashSet;

use crate::{
    content::{content_mapping::DclContentMappingAndUrl, content_provider::ContentProvider},
    dcl::{
        components::{
            material::{DclMaterial, DclSourceTex, DclTexture},
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
    engine::{
        base_material_3d::{Feature, Flags, ShadingMode, Transparency},
        MeshInstance3D, StandardMaterial3D,
    },
    global::weakref,
    prelude::*,
};

pub fn update_material(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let material_component = SceneCrdtStateProtoComponents::get_material(crdt_state);
    let mut content_provider = DclGlobal::singleton().bind().get_content_provider();

    if let Some(material_dirty) = dirty_lww_components.get(&SceneComponentId::MATERIAL) {
        tracing::debug!(
            "Processing {} dirty material entities",
            material_dirty.len()
        );
        for entity in material_dirty {
            tracing::debug!("Processing material update for entity {:?}", entity);
            let new_value = material_component.get(entity);
            if new_value.is_none() {
                tracing::debug!(
                    "Entity {:?} has no material component value, skipping",
                    entity
                );
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

            tracing::debug!(
                "Entity {:?} material parsed: {:?}",
                entity,
                dcl_material.as_ref().map(|m| match m {
                    DclMaterial::Unlit(_) => "Unlit".to_string(),
                    DclMaterial::Pbr(pbr) => format!("PBR({:?})", pbr),
                })
            );

            let (godot_entity_node, node_3d) = godot_dcl_scene.ensure_node_3d(entity);

            if let Some(dcl_material) = dcl_material {
                let previous_dcl_material = godot_entity_node.material.as_ref();
                if let Some(previous_dcl_material) = previous_dcl_material {
                    if previous_dcl_material.eq(&dcl_material) {
                        tracing::debug!("Entity {:?} material unchanged, skipping", entity);
                        continue;
                    }
                }
                tracing::debug!("Entity {:?} has new material, updating", entity);

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

                if let Some(value) = existing_material.as_ref() {
                    scene.materials.get_mut(entity).unwrap().waiting_textures = true;
                } else {
                    let textures: Vec<_> =
                        dcl_material.get_textures().into_iter().flatten().collect();
                    tracing::debug!("Entity {:?} requesting {} textures", entity, textures.len());
                    for tex in textures {
                        if let DclSourceTex::Texture(hash) = &tex.source {
                            tracing::debug!("Entity {:?} fetching texture hash: {}", entity, hash);
                            content_provider.call_deferred(
                                "fetch_texture_by_hash".into(),
                                &[
                                    GString::from(hash).to_variant(),
                                    DclContentMappingAndUrl::from_ref(
                                        scene.content_mapping.clone(),
                                    )
                                    .to_variant(),
                                ],
                            );
                        }
                    }
                }

                let mut godot_material = if let Some(material) = existing_material {
                    tracing::debug!("Entity {:?} using existing Godot material", entity);
                    material.to::<Gd<StandardMaterial3D>>()
                } else {
                    let godot_material = StandardMaterial3D::new_gd();

                    let waiting_textures = {
                        match &dcl_material {
                            DclMaterial::Unlit(unlit) => unlit.texture.is_some(),
                            DclMaterial::Pbr(pbr) => {
                                pbr.texture.is_some()
                                    || pbr.bump_texture.is_some()
                                    || pbr.alpha_texture.is_some()
                                    || pbr.emissive_texture.is_some()
                            }
                        }
                    };

                    tracing::debug!(
                        "Entity {:?} creating new Godot material, waiting_textures: {}",
                        entity,
                        waiting_textures
                    );

                    scene.materials.insert(
                        *entity,
                        MaterialItem {
                            dcl_mat: dcl_material.clone(),
                            weak_ref: weakref(godot_material.to_variant()),
                            waiting_textures,
                            alive: true,
                        },
                    );
                    godot_material
                };

                godot_material.set_transparency(Transparency::ALPHA_DEPTH_PRE_PASS);

                match &dcl_material {
                    DclMaterial::Unlit(unlit) => {
                        tracing::debug!("Entity {:?} applying Unlit material properties", entity);
                        godot_material.set_metallic(0.0);
                        godot_material.set_roughness(0.0);
                        godot_material.set_specular(0.0);

                        godot_material.set_shading_mode(ShadingMode::UNSHADED);
                        godot_material.set_flag(Flags::ALBEDO_TEXTURE_FORCE_SRGB, true);
                        godot_material
                            .set_albedo(unlit.diffuse_color.0.to_godot().linear_to_srgb());
                    }
                    DclMaterial::Pbr(pbr) => {
                        tracing::debug!("Entity {:?} applying PBR material properties (metallic: {}, roughness: {}, specular: {})",
                            entity, pbr.metallic.0, pbr.roughness.0, pbr.specular_intensity.0);
                        godot_material.set_metallic(pbr.metallic.0);
                        godot_material.set_roughness(pbr.roughness.0);
                        godot_material.set_specular(pbr.specular_intensity.0);

                        let emission = pbr
                            .emissive_color
                            .0
                            .clone()
                            .multiply(pbr.emissive_intensity.0);

                        // In the Mobile renderer, HDR will be capped at 2.0 so we'll have to reduce the energy multiplier to be able to see fluctuations in energy
                        godot_material.set_emission_energy_multiplier(0.2);

                        // In the same way, godot uses sRGB instead of linear colors.
                        godot_material.set_emission(emission.to_godot().linear_to_srgb());
                        godot_material.set_feature(Feature::EMISSION, true);
                        godot_material.set_flag(Flags::ALBEDO_TEXTURE_FORCE_SRGB, true);
                        godot_material.set_albedo(pbr.albedo_color.0.to_godot().linear_to_srgb());
                    }
                }
                let mesh_renderer =
                    node_3d.try_get_node_as::<MeshInstance3D>(NodePath::from("MeshRenderer"));
                if let Some(mut mesh_renderer) = mesh_renderer {
                    tracing::debug!("Entity {:?} applying material to MeshRenderer", entity);
                    mesh_renderer.set_surface_override_material(0, godot_material.upcast());
                } else {
                    tracing::debug!("Entity {:?} has no MeshRenderer node", entity);
                }
            } else {
                tracing::debug!(
                    "Entity {:?} removing material (dcl_material is None)",
                    entity
                );
                let mesh_renderer =
                    node_3d.try_get_node_as::<MeshInstance3D>(NodePath::from("MeshRenderer"));

                if let Some(mut mesh_renderer) = mesh_renderer {
                    mesh_renderer.call(
                        "set_surface_override_material".into(),
                        &[0.to_variant(), Variant::nil()],
                    );
                    godot_entity_node.material.take();
                }
            }
        }

        scene.dirty_materials = true;
    }

    if scene.dirty_materials {
        tracing::debug!(
            "Processing dirty materials, total materials tracked: {}, with pending textures {}",
            scene.materials.len(),
            scene
                .materials
                .iter()
                .filter(|v| v.1.waiting_textures)
                .count()
        );
        let mut keep_dirty = false;
        let mut dead_materials = HashSet::with_capacity(scene.materials.capacity());
        let mut no_more_waiting_materials = HashSet::new();

        for (entity, item) in scene.materials.iter() {
            let dcl_material = item.dcl_mat.clone();
            if item.waiting_textures {
                tracing::debug!("Entity {:?} is waiting for textures", entity);
                let material_item = item.weak_ref.call("get_ref", &[]);
                if material_item.is_nil() {
                    tracing::debug!(
                        "Entity {:?} material weak reference is nil, marking as dead",
                        entity
                    );
                    // item.alive = false;
                    dead_materials.insert(*entity);
                    continue;
                }

                let mut material = material_item.to::<Gd<StandardMaterial3D>>();
                let mut ready = true;

                match dcl_material {
                    DclMaterial::Unlit(unlit_material) => {
                        tracing::debug!("Entity {:?} checking Unlit material texture", entity);
                        ready &= check_texture(
                            godot::engine::base_material_3d::TextureParam::ALBEDO,
                            &unlit_material.texture,
                            &mut material,
                            content_provider.bind_mut(),
                            scene,
                        );
                    }
                    DclMaterial::Pbr(pbr) => {
                        tracing::debug!("Entity {:?} checking PBR material textures", entity);
                        ready &= check_texture(
                            godot::engine::base_material_3d::TextureParam::ALBEDO,
                            &pbr.texture,
                            &mut material,
                            content_provider.bind_mut(),
                            scene,
                        );
                        // check_texture(
                        //     godot::engine::base_material_3d::TextureParam::,
                        //     &pbr.alpha_texture,
                        //     item,
                        //     &mut content_provider,
                        // );
                        ready &= check_texture(
                            godot::engine::base_material_3d::TextureParam::NORMAL,
                            &pbr.bump_texture,
                            &mut material,
                            content_provider.bind_mut(),
                            scene,
                        );
                        ready &= check_texture(
                            godot::engine::base_material_3d::TextureParam::EMISSION,
                            &pbr.emissive_texture,
                            &mut material,
                            content_provider.bind_mut(),
                            scene,
                        );
                    }
                }

                if !ready {
                    tracing::debug!(
                        "Entity {:?} textures not ready yet, keeping dirty state",
                        entity
                    );
                    keep_dirty = true;
                } else {
                    tracing::debug!("Entity {:?} all textures loaded successfully", entity);
                    // item.waiting_textures = false;
                    no_more_waiting_materials.insert(*entity);
                }
            }
        }

        if !no_more_waiting_materials.is_empty() {
            tracing::debug!(
                "Marking {} materials as no longer waiting for textures",
                no_more_waiting_materials.len()
            );
        }
        for materials in no_more_waiting_materials {
            scene
                .materials
                .get_mut(&materials)
                .unwrap()
                .waiting_textures = false;
        }

        if !dead_materials.is_empty() {
            tracing::debug!("Removing {} dead materials", dead_materials.len());
        }
        scene.materials.retain(|k, _| !dead_materials.contains(k));
        scene.dirty_materials = keep_dirty;
        tracing::debug!(
            "Dirty materials processing complete, keep_dirty: {}",
            keep_dirty
        );
    }
}

fn check_texture(
    param: godot::engine::base_material_3d::TextureParam,
    dcl_texture: &Option<DclTexture>,
    material: &mut Gd<StandardMaterial3D>,
    mut content_provider: GdMut<ContentProvider>,
    _scene: &Scene,
) -> bool {
    if dcl_texture.is_none() {
        tracing::debug!("No texture specified for param {:?}", param);
        return true;
    }

    let dcl_texture = dcl_texture.as_ref().unwrap();

    match &dcl_texture.source {
        DclSourceTex::Texture(content_hash) => {
            tracing::debug!(
                "Checking texture param {:?} with hash: {}",
                param,
                content_hash
            );
            if content_provider.is_resource_from_hash_loaded(GString::from(content_hash)) {
                if let Some(resource) =
                    content_provider.get_texture_from_hash(GString::from(content_hash))
                {
                    tracing::debug!(
                        "Texture {} loaded, applying to material param {:?}",
                        content_hash,
                        param
                    );
                    material.set_texture(param, resource.upcast());
                } else {
                    tracing::debug!(
                        "Texture {} marked as loaded but get_texture_from_hash returned None",
                        content_hash
                    );
                }
                return true;
            } else {
                tracing::debug!("Texture {} not yet loaded, waiting", content_hash);
                return false;
            }
        }
        DclSourceTex::AvatarTexture(user_id) => {
            tracing::debug!(
                "Avatar texture requested for param {:?}, user_id: {:?} (not yet implemented)",
                param,
                user_id
            );
            // TODO: implement load avatar texture
        }

        #[cfg(not(feature = "use_ffmpeg"))]
        DclSourceTex::VideoTexture(video_entity_id) => {
            tracing::debug!(
                "Video texture requested for param {:?}, entity: {:?} (ffmpeg not enabled)",
                param,
                video_entity_id
            );
            // TODO: set a texture with a `without-video build` message
        }
        #[cfg(feature = "use_ffmpeg")]
        DclSourceTex::VideoTexture(video_entity_id) => {
            tracing::debug!(
                "Video texture requested for param {:?}, entity: {:?}",
                param,
                video_entity_id
            );
            if let Some(node) = _scene
                .godot_dcl_scene
                .get_godot_entity_node(video_entity_id)
            {
                if let Some(data) = &node.video_player_data {
                    tracing::debug!(
                        "Video texture found for entity {:?}, applying to material",
                        video_entity_id
                    );
                    material.set_texture(param, data.video_sink.texture.clone().upcast());
                    return true;
                } else {
                    tracing::debug!("Entity {:?} has no video_player_data", video_entity_id);
                }
            } else {
                tracing::debug!("Entity {:?} not found in scene", video_entity_id);
            }
            return false;
        }
    }

    true
}
