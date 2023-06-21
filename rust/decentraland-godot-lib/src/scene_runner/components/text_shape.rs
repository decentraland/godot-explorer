use crate::{
    dcl::{
        components::SceneComponentId,
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    scene_runner::scene_manager::Scene,
};
use godot::{
    engine::{node::InternalMode, Label3D},
    prelude::*,
};

pub fn update_text_shape(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    if let Some(text_shape_dirty) = dirty_lww_components.get(&SceneComponentId::TEXT_SHAPE) {
        let text_shape_component = SceneCrdtStateProtoComponents::get_text_shape(crdt_state);

        for entity in text_shape_dirty {
            let new_value = text_shape_component.get(*entity);
            if new_value.is_none() {
                continue;
            }

            let new_value = new_value.unwrap();
            let node = godot_dcl_scene.ensure_node_mut(entity);

            let new_value = new_value.value.clone();
            let existing = node
                .base
                .try_get_node_as::<Label3D>(NodePath::from("TextShape"));

            if new_value.is_none() {
                if let Some(text_shape_node) = existing {
                    node.base.remove_child(text_shape_node.upcast());
                }
            } else if let Some(new_value) = new_value {
                let (mut label_3d, add_to_base) = match existing {
                    Some(label_3d) => (label_3d, false),
                    None => (Label3D::new_alloc(), true),
                };

                label_3d.set_text(GodotString::from(new_value.text));
                label_3d.set_font_size(8 * new_value.font_size.unwrap_or(24.0) as i64); // TODO: see font size fix

                if add_to_base {
                    label_3d.set_name(GodotString::from("TextShape"));
                    node.base.add_child(
                        label_3d.upcast(),
                        false,
                        InternalMode::INTERNAL_MODE_DISABLED,
                    );
                }
            }
        }
    }
}
