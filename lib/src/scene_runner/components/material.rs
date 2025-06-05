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
    prelude::*,
};

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

                if existing_material.is_none() {
                    for tex in dcl_material.get_textures().into_iter().flatten() {
                        if let DclSourceTex::Texture(hash) = &tex.source {
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
                        godot_material.set_metallic(0.0);
                        godot_material.set_roughness(0.0);
                        godot_material.set_specular(0.0);

                        godot_material.set_shading_mode(ShadingMode::UNSHADED);
                        godot_material.set_flag(Flags::ALBEDO_TEXTURE_FORCE_SRGB, true);
                        godot_material
                            .set_albedo(unlit.diffuse_color.0.to_godot().linear_to_srgb());
                    }
                    DclMaterial::Pbr(pbr) => {
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
                    mesh_renderer.set_surface_override_material(0, godot_material.upcast());
                }
            } else {
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
        let mut keep_dirty = false;
        let mut dead_materials = HashSet::with_capacity(scene.materials.capacity());
        let mut no_more_waiting_materials = HashSet::new();

        for (entity, item) in scene.materials.iter() {
            let dcl_material = item.dcl_mat.clone();
            if item.waiting_textures {
                let material_item = item.weak_ref.call("get_ref", &[]);
                if material_item.is_nil() {
                    // item.alive = false;
                    dead_materials.insert(*entity);
                    continue;
                }

                let mut material = material_item.to::<Gd<StandardMaterial3D>>();
                let mut ready = true;

                match dcl_material {
                    DclMaterial::Unlit(unlit_material) => {
                        ready &= check_texture(
                            godot::engine::base_material_3d::TextureParam::ALBEDO,
                            &unlit_material.texture,
                            &mut material,
                            content_provider.bind_mut(),
                            scene,
                        );
                    }
                    DclMaterial::Pbr(pbr) => {
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
                    keep_dirty = true;
                } else {
                    // item.waiting_textures = false;
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

fn check_texture(
    param: godot::engine::base_material_3d::TextureParam,
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
            if content_provider.is_resource_from_hash_loaded(GString::from(content_hash)) {
                if let Some(resource) =
                    content_provider.get_texture_from_hash(GString::from(content_hash))
                {
                    material.set_texture(param, resource.upcast());
                }
                return true;
            } else {
                return false;
            }
        }
        DclSourceTex::AvatarTexture(_user_id) => {
            // TODO: implement load avatar texture
        }

        #[cfg(not(feature = "use_ffmpeg"))]
        DclSourceTex::VideoTexture(_video_entity_id) => {
            // TODO: set a texture with a `without-video build` message
        }
        #[cfg(feature = "use_ffmpeg")]
        DclSourceTex::VideoTexture(video_entity_id) => {
            if let Some(node) = _scene
                .godot_dcl_scene
                .get_godot_entity_node(video_entity_id)
            {
                if let Some(data) = &node.video_player_data {
                    material.set_texture(param, data.video_sink.texture.clone().upcast());
                    return true;
                }
            }
            return false;
        }
    }

    true
}
