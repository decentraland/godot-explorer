use crate::{
    dcl::{
        components::SceneComponentId,
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    scene_runner::{godot_dcl_scene::add_own_visual_child, scene::Scene},
    text_shape::{layout::TextShapeParams, node::DclTextShape},
};
use godot::{classes::Node, prelude::*};

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
            let existing = node_3d
                .try_get_node_as::<Node3D>("TextShape")
                .and_then(|n| n.try_cast::<DclTextShape>().ok());

            if new_value.is_none() {
                if let Some(mut text_shape_node) = existing {
                    text_shape_node.queue_free();
                    node_3d.remove_child(&text_shape_node.upcast::<Node>());
                }
            } else if let Some(new_value) = new_value {
                let mut text_shape_node = match existing {
                    Some(node) => node,
                    None => {
                        let mut node = DclTextShape::new_alloc();
                        node.set_name("TextShape");
                        add_own_visual_child(&mut node_3d, &node.clone().upcast::<Node>());
                        node
                    }
                };

                text_shape_node
                    .bind_mut()
                    .apply(&text_shape_params(&new_value));
            }
        }
    }
}

/// The raw proto fields as the renderer's params.
fn text_shape_params(
    value: &crate::dcl::components::proto_components::sdk::components::PbTextShape,
) -> TextShapeParams {
    TextShapeParams {
        text: value.text.clone(),
        font: value.font,
        font_size: value.font_size,
        font_auto_size: value.font_auto_size,
        text_align: value.text_align,
        width: value.width,
        height: value.height,
        text_wrapping: value.text_wrapping,
        outline_width: value.outline_width,
        line_spacing: value.line_spacing,
        text_color: value
            .text_color
            .as_ref()
            .map(|c| Color::from_rgba(c.r, c.g, c.b, c.a)),
        outline_color: value
            .outline_color
            .as_ref()
            .map(|c| Color::from_rgba(c.r, c.g, c.b, 1.0)),
        shadow_blur: value.shadow_blur,
        shadow_offset_x: value.shadow_offset_x,
        shadow_offset_y: value.shadow_offset_y,
        shadow_color: value
            .shadow_color
            .as_ref()
            .map(|c| Color::from_rgba(c.r, c.g, c.b, 1.0)),
    }
}
