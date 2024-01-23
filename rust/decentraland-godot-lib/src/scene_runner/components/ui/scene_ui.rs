use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

use crate::{
    dcl::{
        components::{
            proto_components::sdk::components::{
                PbPointerEventsResult, PbUiCanvasInformation, PbUiDropdownResult, PbUiInputResult,
            },
            SceneComponentId, SceneEntityId,
        },
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, InsertIfNotExists, SceneCrdtState,
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

use super::{ui_dropdown::update_ui_dropdown, ui_input::update_ui_input};

pub struct UiResults {
    pub pointer_event_results: Vec<(SceneEntityId, PbPointerEventsResult)>,
    pub input_results: HashMap<SceneEntityId, PbUiInputResult>,
    pub dropdown_results: HashMap<SceneEntityId, PbUiDropdownResult>,
}

impl UiResults {
    pub fn new_shared() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            pointer_event_results: Vec::new(),
            input_results: HashMap::new(),
            dropdown_results: HashMap::new(),
        }))
    }
}

const UI_COMPONENT_IDS: [SceneComponentId; 5] = [
    SceneComponentId::UI_TRANSFORM,
    SceneComponentId::UI_TEXT,
    SceneComponentId::UI_INPUT,
    SceneComponentId::UI_DROPDOWN,
    SceneComponentId::UI_BACKGROUND,
];

fn update_layout(scene: &mut Scene, ui_canvas_information: &PbUiCanvasInformation) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;

    let mut unprocessed_uis = godot_dcl_scene.ui_entities.clone();
    let mut processed_nodes = HashMap::with_capacity(unprocessed_uis.len());
    let mut processed_nodes_sorted = Vec::with_capacity(unprocessed_uis.len());

    let mut taffy: taffy::Taffy<()> = taffy::Taffy::new();
    let root_node = taffy
        .new_leaf(taffy::style::Style {
            size: taffy::Size {
                width: taffy::style::Dimension::Length(ui_canvas_information.width as f32),
                height: taffy::style::Dimension::Length(ui_canvas_information.height as f32),
            },
            ..Default::default()
        })
        .expect("failed to create root node");

    processed_nodes.insert(SceneEntityId::ROOT, (root_node, 0));

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

            if ui_node.computed_parent != ui_node.ui_transform.parent {
                if let Some(new_parent) = godot_dcl_scene.get_node_ui(&ui_node.ui_transform.parent)
                {
                    ui_node
                        .base_control
                        .clone()
                        .reparent(new_parent.base_control.clone().upcast());
                }
            }

            let child = taffy
                .new_leaf(ui_node.ui_transform.taffy_style.clone())
                .expect("failed to create node");
            if let Some(text_size) = ui_node.text_size {
                let size_child = taffy
                    .new_leaf(taffy::style::Style {
                        size: taffy::Size {
                            width: taffy::style::Dimension::Length(text_size.x),
                            height: taffy::style::Dimension::Length(text_size.y),
                        },
                        ..Default::default()
                    })
                    .expect("failed to create node");

                let _ = taffy.add_child(child, size_child);
            }

            let _ = taffy.add_child(parent.0, child);
            processed_nodes.insert(*entity, (child, 0));
            processed_nodes_sorted.push((*entity, child));

            // mark to continue and remove from unprocessed
            modified = true;
            false
        })
    }

    for unprocessed_entity in unprocessed_uis.iter() {
        if let Some(ui_node) = godot_dcl_scene.get_node_ui_mut(unprocessed_entity) {
            ui_node.base_control.hide();
        }
    }

    let size = taffy::prelude::Size {
        width: taffy::style::AvailableSpace::Definite(ui_canvas_information.width as f32),
        height: taffy::style::AvailableSpace::Definite(ui_canvas_information.height as f32),
    };

    taffy
        .compute_layout(root_node, size)
        .expect("failed to compute layout");

    tracing::debug!("number of node to process {}", processed_nodes.len());

    for (entity, key_node) in processed_nodes_sorted.iter() {
        let ui_node = godot_dcl_scene.get_node_ui(entity).unwrap();
        let parent_node = processed_nodes
            .get_mut(&ui_node.ui_transform.parent)
            .expect("parent not found, it was processed before");

        tracing::debug!(
            "entity {} was parented to {} as child index {} ",
            entity,
            ui_node.ui_transform.parent,
            parent_node.1
        );

        if let Some(parent) = godot_dcl_scene.get_node_ui(&ui_node.ui_transform.parent) {
            parent.base_control.clone().move_child(
                ui_node.base_control.clone().upcast(),
                parent_node.1 + parent.control_offset(),
            );
            parent_node.1 += 1;
        }

        let ui_node = godot_dcl_scene.get_node_ui_mut(entity).unwrap();
        ui_node.computed_parent = ui_node.ui_transform.parent;

        let mut control = ui_node.base_control.clone();
        let layout = taffy.layout(*key_node).unwrap();
        control.set_position(godot::prelude::Vector2 {
            x: layout.location.x,
            y: layout.location.y,
        });
        control.set_size(godot::prelude::Vector2 {
            x: layout.size.width,
            y: layout.size.height,
        });
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
                    .or_insert(Vec::with_capacity(hidden_dirty.len()));

                hidden_dirty.iter().for_each(|entity_id| {
                    if !crdt_state.entities.is_dead(entity_id) {
                        dirty.insert_if_not_exists(*entity_id);
                    }
                });
            }
        }

        scene.godot_dcl_scene.hidden_dirty.clear();
    }

    let dirty_lww_components = &scene.current_dirty.lww_components;
    let need_update_ui_canvas = {
        let ui_canvas_information_component =
            SceneCrdtStateProtoComponents::get_ui_canvas_information(crdt_state);
        if let Some(entry) = ui_canvas_information_component.get(&SceneEntityId::ROOT) {
            if let Some(current_value) = entry.value.as_ref() {
                current_value != ui_canvas_information
            } else {
                true
            }
        } else {
            true
        }
    };
    let need_skip: bool = dirty_lww_components
        .get(&SceneComponentId::UI_TRANSFORM)
        .is_none()
        && dirty_lww_components
            .get(&SceneComponentId::UI_BACKGROUND)
            .is_none()
        && dirty_lww_components
            .get(&SceneComponentId::UI_TEXT)
            .is_none()
        && dirty_lww_components
            .get(&SceneComponentId::UI_DROPDOWN)
            .is_none()
        && dirty_lww_components
            .get(&SceneComponentId::UI_INPUT)
            .is_none()
        && !need_update_ui_canvas;

    if need_update_ui_canvas {
        let ui_canvas_information_component =
            SceneCrdtStateProtoComponents::get_ui_canvas_information_mut(crdt_state);
        ui_canvas_information_component
            .put(SceneEntityId::ROOT, Some(ui_canvas_information.clone()));
    }

    if need_skip {
        update_input_result(scene, crdt_state);
    } else {
        update_ui_transform(scene, crdt_state);
        update_ui_background(scene, crdt_state);
        update_ui_text(scene, crdt_state);
        update_ui_input(scene, crdt_state);
        update_ui_dropdown(scene, crdt_state);
        update_layout(scene, ui_canvas_information);

        update_input_result(scene, crdt_state);
    }
}

fn update_input_result(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let mut ui_results = scene.godot_dcl_scene.ui_results.borrow_mut();

    let input_results = ui_results.input_results.drain();
    let ui_input_result_mut = SceneCrdtStateProtoComponents::get_ui_input_result_mut(crdt_state);
    for (entity, value) in input_results {
        ui_input_result_mut.put(entity, Some(value));
    }

    let input_results = ui_results.dropdown_results.drain();
    let ui_dropdown_result_mut =
        SceneCrdtStateProtoComponents::get_ui_dropdown_result_mut(crdt_state);
    for (entity, value) in input_results {
        ui_dropdown_result_mut.put(entity, Some(value));
    }
}
