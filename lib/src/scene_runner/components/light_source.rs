use std::time::Instant;

use crate::{
    content::content_mapping::ContentMappingAndUrlRef,
    dcl::{
        components::{
            proto_components::{
                common::texture_union,
                sdk::components::{pb_light_source, PbLightSource},
            },
            SceneComponentId,
        },
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation,
            SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    scene_runner::scene::Scene,
};

use godot::{classes::Node, prelude::*};

const DEBUG_DCL_LIGHTS_LOG: bool = false;

fn fmt_v3(v: Vector3) -> String {
    format!("({}, {}, {})", v.x, v.y, v.z)
}

pub fn create_or_update_light(
    content_mapping: &ContentMappingAndUrlRef,
    entity_node: &Gd<Node3D>,
    light_node: &mut Gd<Node3D>,
    light: &PbLightSource,
    entity_debug_name: &str,
) {
    let color = light.color.as_ref();

    let r = color.map(|c| c.r).unwrap_or(1.0);
    let g = color.map(|c| c.g).unwrap_or(1.0);
    let b = color.map(|c| c.b).unwrap_or(1.0);

    let color_variant = Color::from_rgb(r, g, b).to_variant();

    let intensity = light.intensity.unwrap_or(300.0);
    let range = light.range.unwrap_or(15.0);

    let mut projector_src = String::new();
    let mut projector_texture_path = String::new();
    let mut has_shadow_mask_texture = false;

    if let Some(texture_union) = light.shadow_mask_texture.as_ref() {
        has_shadow_mask_texture = true;

        if let Some(texture_union::Tex::Texture(texture)) = texture_union.tex.as_ref() {
            let src = texture.src.clone();
            projector_src = src.clone();

            if !src.is_empty() {
                // Resolve scene-local texture paths through the content mapping.
                // Absolute URLs are passed through as-is.
                let texture_path = if src.starts_with("http://") || src.starts_with("https://") {
                    src.clone()
                } else {
                    let mapping = content_mapping.as_ref();

                    match mapping.get_hash(&src) {
                        Some(hash) => format!("{}{}", mapping.base_url, hash),
                        None => src.clone(),
                    }
                };

                projector_texture_path = texture_path.clone();

                // Always keep the authored source path for debug/display purposes.
                light_node.call(
                    "set_projector_texture_display_path",
                    &[projector_src.to_variant()],
                );

                let current_path = light_node
                    .call("get_projector_texture_path", &[])
                    .try_to::<String>()
                    .unwrap_or_default();

                if current_path != texture_path {
                    light_node.call(
                        "set_projector_texture",
                        &[
                            texture_path.to_variant(),
                            projector_src.to_variant(),
                        ],
                    );
                }
            }
        }
    }

    let light_type_debug = match light.r#type.as_ref() {
        Some(pb_light_source::Type::Spot(_)) => "spot",
        Some(pb_light_source::Type::Point(_)) => "point",
        _ => "point/default",
    };

    let mut inner_angle_debug: Option<f32> = None;
    let mut outer_angle_debug: Option<f32> = None;

    if let Some(pb_light_source::Type::Spot(spot)) = light.r#type.as_ref() {
        inner_angle_debug = Some(spot.inner_angle.unwrap_or(0.0));
        outer_angle_debug = Some(spot.outer_angle.unwrap_or(30.0));
    }

    if DEBUG_DCL_LIGHTS_LOG {
        let entity_local_position = entity_node.get_position();
        let entity_local_rotation = entity_node.get_rotation_degrees();
        let entity_global_position = entity_node.get_global_position();
        let entity_global_rotation = entity_node.get_global_rotation_degrees();
        let entity_scale = entity_node.get_scale();

        let light_local_position = light_node.get_position();
        let light_local_rotation = light_node.get_rotation_degrees();
        let light_scale = light_node.get_scale();

        godot_print!(
            "[DCL LIGHTS] entity={} type={} \
             color=({}, {}, {}) intensity={} range={} \
             inner_angle={:?} outer_angle={:?} \
             texture_src=\"{}\" texture_path=\"{}\" has_shadow_mask_texture={} shadow_enabled=true \
             entity_local_position={} entity_local_rotation={} entity_global_position={} entity_global_rotation={} entity_scale={} \
             light_node_local_position={} light_node_local_rotation={} light_node_scale={}",
            entity_debug_name,
            light_type_debug,
            r,
            g,
            b,
            intensity,
            range,
            inner_angle_debug,
            outer_angle_debug,
            projector_src,
            projector_texture_path,
            has_shadow_mask_texture,
            fmt_v3(entity_local_position),
            fmt_v3(entity_local_rotation),
            fmt_v3(entity_global_position),
            fmt_v3(entity_global_rotation),
            fmt_v3(entity_scale),
            fmt_v3(light_local_position),
            fmt_v3(light_local_rotation),
            fmt_v3(light_scale),
        );
    }

    match light.r#type.as_ref() {
        Some(pb_light_source::Type::Spot(spot)) => {
            let inner_angle = spot.inner_angle.unwrap_or(0.0);
            let outer_angle = spot.outer_angle.unwrap_or(30.0);

            light_node.call(
                "set_spot",
                &[
                    color_variant,
                    intensity.to_variant(),
                    range.to_variant(),
                    inner_angle.to_variant(),
                    outer_angle.to_variant(),
                ],
            );
        }
        Some(pb_light_source::Type::Point(_point)) => {
            light_node.call(
                "set_point",
                &[color_variant, intensity.to_variant(), range.to_variant()],
            );
        }
        _ => {
            light_node.call(
                "set_point",
                &[color_variant, intensity.to_variant(), range.to_variant()],
            );
        }
    }
}

pub fn update_light_source(
    scene: &mut Scene,
    crdt_state: &mut SceneCrdtState,
    ref_time: &Instant,
    end_time_us: i64,
) -> bool {
    let mut updated_count = 0;
    let mut current_time_us;

    let content_mapping = scene.content_mapping.clone();
    let godot_dcl_scene = &mut scene.godot_dcl_scene;

    let light_source_dirty = scene
        .current_dirty
        .lww_components
        .remove(&SceneComponentId::LIGHT_SOURCE);

    if let Some(mut light_source_dirty) = light_source_dirty {
        let light_source_component = SceneCrdtStateProtoComponents::get_light_source(crdt_state);

        for entity in light_source_dirty.iter() {
            let Some(new_value) = light_source_component.get(entity) else {
                updated_count += 1;
                continue;
            };
            let (_godot_entity_node, mut node_3d) = godot_dcl_scene.ensure_node_3d(entity);

            let light_value = new_value.value.clone();
            let existing = node_3d.try_get_node_as::<Node3D>("LightSource");

            match light_value {
                None => {
                    if DEBUG_DCL_LIGHTS_LOG {
                        godot_print!(
                            "[DCL LIGHTS] entity={:?} removed=true entity_global_position={} entity_global_rotation={}",
                            entity,
                            fmt_v3(node_3d.get_global_position()),
                            fmt_v3(node_3d.get_global_rotation_degrees()),
                        );
                    }

                    if let Some(mut light_node) = existing {
                        light_node.call("remove_light", &[]);
                        node_3d.remove_child(&light_node.clone().upcast::<Node>());
                        light_node.queue_free();
                    }
                }
                Some(light_value) => {
                    let mut light_node = match existing {
                        Some(light_node) => light_node,
                        None => {
                            let scene = godot::tools::load::<PackedScene>(
                                "res://src/decentraland_components/light_source_component.tscn",
                            );

                            let mut new_light_node = scene.instantiate().unwrap().cast::<Node3D>();
                            new_light_node.set_name("LightSource");
                            // Important:
                            // Add the node to the tree first so Godot runs _ready()
                            // before setting texture or debug display data.
                            node_3d.add_child(&new_light_node.clone().upcast::<Node>());

                            new_light_node
                        }
                    };

                    create_or_update_light(
                        &content_mapping,
                        &node_3d,
                        &mut light_node,
                        &light_value,
                        &format!("{:?}", entity),
                    );
                }
            }

            updated_count += 1;
            current_time_us = (std::time::Instant::now() - *ref_time).as_micros() as i64;

            if current_time_us > end_time_us {
                break;
            }
        }

        if updated_count < light_source_dirty.len() {
            light_source_dirty.drain(0..updated_count);

            scene
                .current_dirty
                .lww_components
                .insert(SceneComponentId::LIGHT_SOURCE, light_source_dirty);

            return false;
        }
    }

    true
}
