use godot::{classes::Node, obj::NewAlloc, prelude::Gd};

use crate::{
    dcl::{
        components::{
            proto_components::sdk::components::{scroll_position_value::Value, YgOverflow},
            SceneComponentId, SceneEntityId,
        },
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    godot_classes::dcl_ui_scroll::DclUiScroll,
    scene_runner::scene::Scene,
};

pub fn update_ui_scroll(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let ui_transform_component = SceneCrdtStateProtoComponents::get_ui_transform(crdt_state);

    let Some(dirty_transforms) = dirty_lww_components.get(&SceneComponentId::UI_TRANSFORM) else {
        return;
    };

    let dirty_entities: Vec<SceneEntityId> = dirty_transforms.to_vec();

    for entity in dirty_entities {
        let pb_transform = ui_transform_component
            .get(&entity)
            .and_then(|e| e.value.as_ref())
            .cloned();

        let is_scroll = pb_transform
            .as_ref()
            .map(|v| v.overflow() == YgOverflow::YgoScroll)
            .unwrap_or(false);

        let existing = godot_dcl_scene
            .ensure_node_ui(&entity)
            .base_ui
            .as_mut()
            .unwrap();

        let had_scroll = existing.scroll_container.is_some();

        if !is_scroll {
            if had_scroll {
                if let Some(mut scroll) = existing.scroll_container.take() {
                    existing
                        .base_control
                        .remove_child(&scroll.clone().upcast::<Node>());
                    scroll.queue_free();
                    invalidate_children_parent(godot_dcl_scene, &entity);
                }
            }
            continue;
        }

        if !had_scroll {
            let mut scroll: Gd<DclUiScroll> = DclUiScroll::new_alloc();
            scroll.set_name("scroll");

            existing
                .base_control
                .add_child(&scroll.clone().upcast::<Node>());

            existing.scroll_container = Some(scroll);
            invalidate_children_parent(godot_dcl_scene, &entity);
        }

        let existing = godot_dcl_scene
            .entities
            .get_mut(&entity)
            .and_then(|n| n.base_ui.as_mut())
            .unwrap();

        if let Some(pb) = pb_transform.as_ref() {
            if let Some(scroll) = existing.scroll_container.as_mut() {
                scroll.bind_mut().set_scroll_visible(pb.scroll_visible());

                if let Some(scroll_pos) = pb.scroll_position.as_ref() {
                    if let Some(Value::Position(v)) = scroll_pos.value.as_ref() {
                        scroll.bind_mut().set_scroll_position(v.x, v.y);
                    }
                }
            }
        }
    }
}

fn invalidate_children_parent(
    godot_dcl_scene: &mut crate::scene_runner::godot_dcl_scene::GodotDclScene,
    parent_entity: &SceneEntityId,
) {
    let all_entities: Vec<SceneEntityId> = godot_dcl_scene.ui_entities.iter().copied().collect();
    for child_entity in all_entities {
        if let Some(base_ui) = godot_dcl_scene
            .entities
            .get_mut(&child_entity)
            .and_then(|n| n.base_ui.as_mut())
        {
            if &base_ui.ui_transform.parent == parent_entity {
                base_ui.computed_parent = SceneEntityId::new(u16::MAX, u16::MAX);
            }
        }
    }
}
