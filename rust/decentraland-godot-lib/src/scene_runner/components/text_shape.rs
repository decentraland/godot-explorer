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
use godot::{engine::Label3D, prelude::*};

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
            let existing = node_3d.try_get_node_as::<Label3D>(NodePath::from("TextShape"));

            if new_value.is_none() {
                if let Some(text_shape_node) = existing {
                    node_3d.remove_child(text_shape_node.upcast());
                }
            } else if let Some(new_value) = new_value {
                let (mut label_3d, add_to_base) = match existing {
                    Some(label_3d) => (label_3d, false),
                    None => (Label3D::new_alloc(), true),
                };

                let opacity = new_value
                    .text_color
                    .as_ref()
                    .map(|color| color.a.clone())
                    .unwrap_or(1.0);

                let text_color = new_value
                    .text_color
                    .map(|color| Color::from_rgba(color.r, color.g, color.b, opacity))
                    .unwrap_or(Color::from_rgba(1.0, 1.0, 1.0, opacity));

                let outline_color = new_value
                    .outline_color
                    .map(|color| Color::from_rgba(color.r, color.g, color.b, opacity))
                    .unwrap_or(Color::from_rgba(1.0, 1.0, 1.0, opacity));

                let shadow_color = new_value
                    .shadow_color
                    .map(|color| Color::from_rgba(color.r, color.g, color.b, opacity))
                    .unwrap_or(Color::from_rgba(1.0, 1.0, 1.0, opacity));

                label_3d.set_text(GString::from(new_value.text));
                label_3d.set_font_size(12 * new_value.font_size.unwrap_or(3.0) as i32); // TODO: see font size fix
                label_3d.set_outline_size(8); // TODO: see font size fix

                label_3d.set_modulate(text_color);
                label_3d.set_outline_modulate(outline_color);

                if add_to_base {
                    label_3d.set_name(GString::from("TextShape"));
                    node_3d.add_child(label_3d.upcast());
                }
            }
        }
    }
}
