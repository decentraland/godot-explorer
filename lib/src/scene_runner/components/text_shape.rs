use crate::{
    dcl::{
        components::SceneComponentId,
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    scene_runner::scene::Scene,
};
use godot::{
    classes::{Node, PackedScene},
    prelude::*,
};

const DCL_TEXT_SHAPE_SCENE: &str =
    "res://src/decentraland_components/text_shape/dcl_text_shape.tscn";

pub fn update_text_shape(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    if let Some(text_shape_dirty) = dirty_lww_components.get(&SceneComponentId::TEXT_SHAPE) {
        let text_shape_component = SceneCrdtStateProtoComponents::get_text_shape(crdt_state);

        for entity in text_shape_dirty {
            let new_value = text_shape_component.get(entity);
            if new_value.is_none() {
                continue;
            }

            let new_value = new_value.unwrap();
            let (_godot_entity_node, mut node_3d) = godot_dcl_scene.ensure_node_3d(entity);

            let new_value = new_value.value.clone();
            let existing = node_3d.try_get_node_as::<Node3D>("TextShape");

            if new_value.is_none() {
                if let Some(mut text_shape_node) = existing {
                    text_shape_node.queue_free();
                    node_3d.remove_child(&text_shape_node.upcast::<Node>());
                }
            } else if let Some(new_value) = new_value {
                // All rendering, sizing and tag handling lives in GDScript
                // (DclTextShape + renderers). Rust only instantiates the node and
                // forwards the raw PbTextShape fields.
                let mut text_shape_node = match existing {
                    Some(node) => node,
                    None => {
                        let mut node = godot::tools::load::<PackedScene>(DCL_TEXT_SHAPE_SCENE)
                            .instantiate()
                            .expect("Failed to instantiate dcl_text_shape.tscn")
                            .cast::<Node3D>();
                        node.set_name("TextShape");
                        node_3d.add_child(&node.clone().upcast::<Node>());
                        node
                    }
                };

                let params = build_text_shape_params(&new_value);
                text_shape_node.call("apply", &[params.to_variant()]);
            }
        }
    }
}

/// Builds the `apply(params)` payload from the raw proto fields. Optional fields
/// carry a `has_*` flag so GDScript can apply the exact same defaults the previous
/// Rust path used.
fn build_text_shape_params(
    value: &crate::dcl::components::proto_components::sdk::components::PbTextShape,
) -> VarDictionary {
    let mut params = VarDictionary::new();
    params.set("text", value.text.clone());

    set_opt_int(&mut params, "font", value.font);
    set_opt_float(&mut params, "font_size", value.font_size);
    set_opt_bool(&mut params, "font_auto_size", value.font_auto_size);
    set_opt_int(&mut params, "text_align", value.text_align);
    set_opt_float(&mut params, "width", value.width);
    set_opt_float(&mut params, "height", value.height);
    set_opt_bool(&mut params, "text_wrapping", value.text_wrapping);
    set_opt_float(&mut params, "outline_width", value.outline_width);
    set_opt_float(&mut params, "line_spacing", value.line_spacing);

    let text_color = value
        .text_color
        .as_ref()
        .map(|c| Color::from_rgba(c.r, c.g, c.b, c.a));
    set_opt_color(&mut params, "text_color", text_color);

    let outline_color = value
        .outline_color
        .as_ref()
        .map(|c| Color::from_rgba(c.r, c.g, c.b, 1.0));
    set_opt_color(&mut params, "outline_color", outline_color);

    set_opt_float(&mut params, "shadow_blur", value.shadow_blur);
    set_opt_float(&mut params, "shadow_offset_x", value.shadow_offset_x);
    set_opt_float(&mut params, "shadow_offset_y", value.shadow_offset_y);

    let shadow_color = value
        .shadow_color
        .as_ref()
        .map(|c| Color::from_rgba(c.r, c.g, c.b, 1.0));
    set_opt_color(&mut params, "shadow_color", shadow_color);

    params
}

fn set_opt_int(params: &mut VarDictionary, key: &str, value: Option<i32>) {
    params.set(format!("has_{key}"), value.is_some());
    params.set(key, value.unwrap_or_default());
}

fn set_opt_float(params: &mut VarDictionary, key: &str, value: Option<f32>) {
    params.set(format!("has_{key}"), value.is_some());
    params.set(key, value.unwrap_or_default());
}

fn set_opt_bool(params: &mut VarDictionary, key: &str, value: Option<bool>) {
    params.set(format!("has_{key}"), value.is_some());
    params.set(key, value.unwrap_or_default());
}

fn set_opt_color(params: &mut VarDictionary, key: &str, value: Option<Color>) {
    params.set(format!("has_{key}"), value.is_some());
    params.set(key, value.unwrap_or(Color::from_rgba(1.0, 1.0, 1.0, 1.0)));
}
