use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

use godot::{
    engine::text_server::{JustificationFlag, LineBreakFlag},
    obj::Gd,
};

use crate::{
    dcl::{
        components::{
            proto_components::sdk::components::{
                PbPointerEventsResult, PbUiCanvasInformation, PbUiDropdownResult, PbUiInputResult,
                TextWrap,
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

enum ContextNode {
    UiText(bool, Gd<godot::engine::Label>),
}

fn update_layout(
    scene: &mut Scene,
    crdt_state: &SceneCrdtState,
    ui_canvas_information: &PbUiCanvasInformation,
) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;

    let mut unprocessed_uis = godot_dcl_scene.ui_entities.clone();
    let mut processed_nodes = HashMap::with_capacity(unprocessed_uis.len());
    let mut processed_nodes_sorted = Vec::with_capacity(unprocessed_uis.len());

    let ui_text_components = SceneCrdtStateProtoComponents::get_ui_text(crdt_state);

    let mut taffy: taffy::TaffyTree<ContextNode> = taffy::TaffyTree::new();
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
                if let Some(new_parent) =
                    godot_dcl_scene.get_node_or_null_ui(&ui_node.ui_transform.parent)
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

            if let Some(ui_text_control) = ui_node
                .base_control
                .try_get_node_as::<godot::engine::Label>("text")
            {
                let text_wrapping = if let Some(ui_text) = ui_text_components
                    .get(entity)
                    .and_then(|v| v.value.as_ref())
                {
                    ui_text.text_wrap_compat() == TextWrap::TwWrap
                } else {
                    false
                };

                let _ = taffy.set_node_context(
                    child,
                    Some(ContextNode::UiText(text_wrapping, ui_text_control)),
                );
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
        if let Some(ui_node) = godot_dcl_scene.get_node_or_null_ui_mut(unprocessed_entity) {
            ui_node.base_control.hide();
        }
    }

    let size = taffy::prelude::Size {
        width: taffy::style::AvailableSpace::Definite(ui_canvas_information.width as f32),
        height: taffy::style::AvailableSpace::Definite(ui_canvas_information.height as f32),
    };

    taffy
        .compute_layout_with_measure(
            root_node,
            size,
            |size, available, node_id, node_context, _style| match node_context {
                Some(ContextNode::UiText(wrapping, text_node)) => {
                    let Some(font) = text_node.get_theme_font("font".into()) else {
                        return taffy::Size::ZERO;
                    };
                    let line_width = match size.width {
                        Some(value) => value,
                        None => match available.width {
                            taffy::AvailableSpace::Definite(v) => v,
                            taffy::AvailableSpace::MinContent => 1.0,
                            taffy::AvailableSpace::MaxContent => -1.0,
                        },
                    };

                    let font_size = text_node.get_theme_font_size("font_size".into());
                    let font_rect = if *wrapping {
                        font.get_multiline_string_size_ex(text_node.get_text())
                            .max_lines(-1)
                            .width(line_width)
                            .font_size(font_size)
                            .alignment(text_node.get_horizontal_alignment())
                            .justification_flags(JustificationFlag::NONE)
                            .brk_flags(LineBreakFlag::WORD_BOUND | LineBreakFlag::MANDATORY)
                            .done()
                    } else {
                        font.get_string_size_ex(text_node.get_text())
                            .width(line_width)
                            .alignment(text_node.get_horizontal_alignment())
                            .justification_flags(JustificationFlag::NONE)
                            .font_size(font_size)
                            .done()
                    };

                    let width = match size.width {
                        Some(value) => value,
                        None => match available.width {
                            taffy::AvailableSpace::Definite(v) => v.clamp(0.0, font_rect.x),
                            taffy::AvailableSpace::MinContent => 1.0,
                            taffy::AvailableSpace::MaxContent => font_rect.x,
                        },
                    };

                    let height = match size.height {
                        Some(value) => value,
                        None => match available.height {
                            taffy::AvailableSpace::Definite(v) => v.clamp(0.0, font_rect.y),
                            taffy::AvailableSpace::MinContent => 1.0,
                            taffy::AvailableSpace::MaxContent => font_rect.y,
                        },
                    };

                    tracing::debug!(
                        "text node {:?}, wrapping {:?}, size: {:?}, font_rect {:?}, available {:?}",
                        node_id,
                        *wrapping,
                        size,
                        font_rect,
                        available
                    );

                    taffy::Size { width, height }
                }
                None => taffy::Size::ZERO,
            },
        )
        .expect("failed to compute layout");

    for (entity, key_node) in processed_nodes_sorted.iter() {
        let ui_node = godot_dcl_scene.get_node_or_null_ui(entity).unwrap();
        let parent_node = processed_nodes
            .get_mut(&ui_node.ui_transform.parent)
            .expect("parent not found, it was processed before");

        if let Some(parent) = godot_dcl_scene.get_node_or_null_ui(&ui_node.ui_transform.parent) {
            parent.base_control.clone().move_child(
                ui_node.base_control.clone().upcast(),
                parent_node.1 + parent.control_offset(),
            );
            parent_node.1 += 1;
        }

        let ui_node = godot_dcl_scene.get_node_or_null_ui_mut(entity).unwrap();
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
        let is_hidden = taffy.style(*key_node).unwrap().display == taffy::style::Display::None;
        control.set_visible(!is_hidden);

        tracing::debug!(
            "node {:?}, entity: {:?}, location: {:?}, size: {:?}",
            key_node,
            entity,
            layout.location,
            layout.size
        );
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

    if need_update_ui_canvas {
        let ui_canvas_information_component =
            SceneCrdtStateProtoComponents::get_ui_canvas_information_mut(crdt_state);
        ui_canvas_information_component
            .put(SceneEntityId::ROOT, Some(ui_canvas_information.clone()));
    }

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

    if need_skip {
        update_input_result(scene, crdt_state);
    } else {
        update_ui_transform(scene, crdt_state);
        update_ui_background(scene, crdt_state);
        update_ui_text(scene, crdt_state);
        update_ui_input(scene, crdt_state);
        update_ui_dropdown(scene, crdt_state);
        update_layout(scene, crdt_state, ui_canvas_information);

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
