use std::collections::HashMap;

use crate::{
    dcl::{
        components::{
            proto_components::sdk::components::PbUiCanvasInformation, SceneComponentId,
            SceneEntityId,
        },
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    scene_runner::{
        components::ui::{
            ui_background::update_ui_background, ui_text::update_ui_text,
            ui_transform::update_ui_transform,
        },
        scene::Scene,
    },
};

fn update_layout(
    scene: &mut Scene,
    crdt_state: &mut SceneCrdtState,
    ui_canvas_information: &PbUiCanvasInformation,
) {
    let ui_transform_component = SceneCrdtStateProtoComponents::get_ui_transform(crdt_state);
    let godot_dcl_scene = &mut scene.godot_dcl_scene;

    let mut root_node_ui = godot_dcl_scene
        .root_node_ui
        .clone()
        .upcast::<godot::prelude::Node>();

    let mut unprocessed_uis = godot_dcl_scene.ui_entities.clone();
    let mut processed_nodes = HashMap::with_capacity(unprocessed_uis.len());
    let mut processed_nodes_sorted = Vec::with_capacity(unprocessed_uis.len());

    let mut taffy = taffy::Taffy::new();

    let viewport_style = taffy::style::Style {
        display: taffy::style::Display::Grid,
        // Note: Taffy percentages are floats ranging from 0.0 to 1.0.
        // So this is setting width:100% and height:100%
        size: taffy::geometry::Size {
            width: taffy::style::Dimension::Percent(1.0),
            height: taffy::style::Dimension::Percent(1.0),
        },
        align_items: Some(taffy::style::AlignItems::Start),
        justify_items: Some(taffy::style::JustifyItems::Start),
        ..Default::default()
    };

    let root_node = taffy
        .new_leaf(viewport_style)
        .expect("failed to create root node");

    processed_nodes.insert(SceneEntityId::ROOT, root_node);

    let mut modified = true;
    while modified && !unprocessed_uis.is_empty() {
        modified = false;
        unprocessed_uis.retain(|entity| {
            let Some(ui_transform) = ui_transform_component.values.get(entity) else {
                return true;
            };
            let Some(ui_transform) = ui_transform.value.as_ref() else {
                return true;
            };

            // if our rightof is not added, we can't process this node
            if !processed_nodes.contains_key(&SceneEntityId::from_i32(ui_transform.right_of)) {
                return true;
            }

            // if our parent is not added, we can't process this node
            let Some(parent) = processed_nodes.get(&SceneEntityId::from_i32(ui_transform.parent))
            else {
                return true;
            };

            let child = taffy
                .new_leaf(ui_transform.into())
                .expect("failed to create node");

            taffy.add_child(*parent, child).unwrap();
            processed_nodes.insert(*entity, child);
            processed_nodes_sorted.push((*entity, child));

            // mark to continue and remove from unprocessed
            modified = true;
            false
        })
    }

    let size = taffy::prelude::Size {
        width: taffy::style::AvailableSpace::Definite(ui_canvas_information.width as f32),
        height: taffy::style::AvailableSpace::Definite(ui_canvas_information.height as f32),
    };

    taffy
        .compute_layout(root_node, size)
        .expect("failed to compute layout");

    tracing::debug!("number of node to process {}", processed_nodes.len());

    let mut idx = 0;
    for (entity, key_node) in processed_nodes_sorted {
        let ui_node = godot_dcl_scene.get_godot_entity_node(&entity).unwrap();
        let parent_position =
            if let Some(parent) = godot_dcl_scene.get_godot_entity_node(&ui_node.parent_ui) {
                parent.base_ui.as_ref().unwrap().base_control.get_position()
            } else {
                godot::prelude::Vector2::new(0.0, 0.0)
            };

        let ui_node = ui_node.base_ui.as_ref().unwrap();
        let mut control = ui_node.base_control.clone();

        let layout = taffy.layout(key_node).unwrap();
        let computed_position = parent_position
            + godot::prelude::Vector2 {
                x: layout.location.x,
                y: layout.location.y,
            };

        control.set_position(computed_position);
        control.set_size(godot::prelude::Vector2 {
            x: layout.size.width,
            y: layout.size.height,
        });
        if entity != SceneEntityId::ROOT {
            root_node_ui.move_child(control.upcast(), idx);
            idx += 1;
        }
    }
}

pub fn update_scene_ui(
    scene: &mut Scene,
    crdt_state: &mut SceneCrdtState,
    ui_canvas_information: &PbUiCanvasInformation,
) {
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let need_skip = dirty_lww_components
        .get(&SceneComponentId::UI_TRANSFORM)
        .is_none()
        && dirty_lww_components
            .get(&SceneComponentId::UI_BACKGROUND)
            .is_none()
        && dirty_lww_components
            .get(&SceneComponentId::UI_TEXT)
            .is_none();

    let need_update_ui_canvas = {
        let ui_canvas_information_component =
            SceneCrdtStateProtoComponents::get_ui_canvas_information(crdt_state);
        if let Some(entry) = ui_canvas_information_component.get(SceneEntityId::ROOT) {
            if let Some(current_value) = entry.value.as_ref() {
                current_value != ui_canvas_information
            } else {
                true
            }
        } else {
            true
        }
    };

    if need_update_ui_canvas {
        let ui_canvas_information_component =
            SceneCrdtStateProtoComponents::get_ui_canvas_information_mut(crdt_state);
        ui_canvas_information_component
            .put(SceneEntityId::ROOT, Some(ui_canvas_information.clone()));
    }

    if need_skip {
        return;
    }

    update_ui_transform(scene, crdt_state);
    update_ui_background(scene, crdt_state);
    update_ui_text(scene, crdt_state);
    update_layout(scene, crdt_state, ui_canvas_information);
}
