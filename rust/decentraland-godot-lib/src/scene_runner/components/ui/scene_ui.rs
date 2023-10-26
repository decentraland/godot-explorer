use std::collections::{HashMap, HashSet};

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
        SceneId,
    },
    scene_runner::{
        components::ui::{
            ui_background::update_ui_background, ui_text::update_ui_text,
            ui_transform::update_ui_transform,
        },
        scene::{Scene, SceneType},
    },
};

const UI_COMPONENT_IDS: [SceneComponentId; 5] = [
    SceneComponentId::UI_TRANSFORM,
    SceneComponentId::UI_TEXT,
    SceneComponentId::UI_INPUT,
    SceneComponentId::UI_DROPDOWN,
    SceneComponentId::UI_BACKGROUND,
];

fn update_layout(scene: &mut Scene, ui_canvas_information: &PbUiCanvasInformation) {
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
            let Some(entity_node) = godot_dcl_scene.get_godot_entity_node(entity) else {
                return true;
            };
            let Some(ui_node) = entity_node.base_ui.as_ref() else {
                return true;
            };

            // if our rightof is not added, we can't process this node
            if !processed_nodes.contains_key(&ui_node.ui_transform.right_of) {
                return true;
            }

            // if our parent is not added, we can't process this node
            let Some(parent) = processed_nodes.get(&ui_node.ui_transform.parent) else {
                return true;
            };

            let child = taffy
                .new_leaf(ui_node.ui_transform.taffy_style.clone())
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
        let ui_node = godot_dcl_scene.get_node_ui(&entity).unwrap();
        let parent_position =
            if let Some(parent) = godot_dcl_scene.get_node_ui(&ui_node.ui_transform.parent) {
                parent.base_control.get_position()
            } else {
                godot::prelude::Vector2::new(0.0, 0.0)
            };

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
    current_parcel_scene_id: &SceneId,
) {
    let ui_is_visible = if let SceneType::Parcel = scene.scene_type {
        &scene.scene_id == current_parcel_scene_id
    } else {
        true
    };

    if !ui_is_visible {
        for component_id in UI_COMPONENT_IDS {
            if let Some(dirty) = scene.current_dirty.lww_components.get(&component_id) {
                let hidden_dirty = scene
                    .godot_dcl_scene
                    .hidden_dirty
                    .entry(component_id)
                    .or_insert(HashSet::with_capacity(dirty.len()));

                dirty.iter().for_each(|entity_id| {
                    hidden_dirty.insert(*entity_id);
                });
            }
        }
        return;
    } else if !scene.godot_dcl_scene.hidden_dirty.is_empty() {
        for component_id in UI_COMPONENT_IDS {
            if let Some(hidden_dirty) = scene.godot_dcl_scene.hidden_dirty.get(&component_id) {
                let dirty = scene
                    .current_dirty
                    .lww_components
                    .entry(component_id)
                    .or_insert(HashSet::with_capacity(hidden_dirty.len()));

                hidden_dirty.iter().for_each(|entity_id| {
                    if !crdt_state.entities.is_dead(entity_id) {
                        dirty.insert(*entity_id);
                    }
                });
            }
        }

        scene.godot_dcl_scene.hidden_dirty.clear();
    }

    let dirty_lww_components = &scene.current_dirty.lww_components;
    let need_skip: bool = dirty_lww_components
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
    update_layout(scene, ui_canvas_information);
}
