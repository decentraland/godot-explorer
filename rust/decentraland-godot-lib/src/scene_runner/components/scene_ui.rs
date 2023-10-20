use taffy::Taffy;

use crate::{
    dcl::{
        components::{SceneComponentId, SceneEntityId},
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    scene_runner::scene::Scene,
};

pub fn update_scene_ui(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let ui_transform_component = SceneCrdtStateProtoComponents::get_ui_transform(&crdt_state);

    let taffy = Taffy::new();

    let mut recalculate_layout = false;

    if let Some(dirty_transform) = dirty_lww_components.get(&SceneComponentId::UI_TRANSFORM) {
        for entity in dirty_transform {
            let value = if let Some(entry) = ui_transform_component.get(*entity) {
                entry.value.clone()
            } else {
                None
            };
            let (godot_entity_node, mut node_ui) = godot_dcl_scene.ensure_node_ui(entity);

            let old_parent = godot_entity_node.desired_parent_ui;
            let mut transform = value.unwrap_or_default();

            transform.parent;
            // ui_transform.right_of: i32,

            // ui_transform.align_content: ::core::option::Option<i32>,
            // ui_transform.align_items: ::core::option::Option<i32>,
            // ui_transform.flex_wrap: ::core::option::Option<i32>,
            // ui_transform.flex_shrink: ::core::option::Option<f32>,
            // ui_transform.position_type: i32,
            // ui_transform.align_self: i32,
            // ui_transform.flex_direction: i32,
            // ui_transform.justify_content: i32,
            // ui_transform.overflow: i32,
            // ui_transform.display: i32,

            // ui_transform.flex_basis_unit: i32,
            // ui_transform.flex_basis: f32,
            // ui_transform.flex_grow: f32,

            // ui_transform.width_unit: i32,
            // ui_transform.width: f32,

            // ui_transform.height_unit: i32,
            // ui_transform.height: f32,

            // ui_transform.min_width_unit: i32,
            // ui_transform.min_width: f32,
            // ui_transform.min_height_unit: i32,
            // ui_transform.min_height: f32,

            // ui_transform.max_width_unit: i32,
            // ui_transform.max_width: f32,
            // ui_transform.max_height_unit: i32,
            // ui_transform.max_height: f32,

            // ui_transform.position_left_unit: i32,
            // ui_transform.position_left: f32,
            // ui_transform.position_top_unit: i32,
            // ui_transform.position_top: f32,
            // ui_transform.position_right_unit: i32,
            // ui_transform.position_right: f32,
            // ui_transform.position_bottom_unit: i32,
            // ui_transform.position_bottom: f32,

            // ui_transform.margin_left_unit: i32,
            // ui_transform.margin_left: f32,
            // ui_transform.margin_top_unit: i32,
            // ui_transform.margin_top: f32,
            // ui_transform.margin_right_unit: i32,
            // ui_transform.margin_right: f32,
            // ui_transform.margin_bottom_unit: i32,
            // ui_transform.margin_bottom: f32,

            // ui_transform.padding_left_unit: i32,
            // ui_transform.padding_left: f32,
            // ui_transform.padding_top_unit: i32,
            // ui_transform.padding_top: f32,
            // ui_transform.padding_right_unit: i32,
            // ui_transform.padding_right: f32,
            // ui_transform.padding_bottom_unit: i32,
            // ui_transform.padding_bottom: f32,

            godot_entity_node.desired_parent_ui = SceneEntityId::from_i32(transform.parent);
            if godot_entity_node.desired_parent_ui != old_parent {
                godot_dcl_scene.unparented_entities_ui.insert(*entity);
                godot_dcl_scene.hierarchy_dirty_ui = true;
            }
        }
    }
}
