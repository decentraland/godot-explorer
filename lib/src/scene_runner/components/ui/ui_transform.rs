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

use super::style::UiTransform;

pub fn update_ui_transform(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let ui_transform_component = SceneCrdtStateProtoComponents::get_ui_transform(crdt_state);

    if let Some(dirty_transform) = dirty_lww_components.get(&SceneComponentId::UI_TRANSFORM) {
        for entity in dirty_transform {
            let ui_transform = if let Some(entry) = ui_transform_component.get(entity) {
                entry.value.as_ref()
            } else {
                None
            };

            if let Some(pb_ui_transform) = ui_transform {
                let node = godot_dcl_scene
                    .ensure_node_ui(entity)
                    .base_ui
                    .as_mut()
                    .unwrap();

                let old_parent = node.ui_transform.parent;
                node.ui_transform = UiTransform::from(pb_ui_transform);
                let style = &node.ui_transform.taffy_style;
                tracing::debug!(
                    "[UI_TRANSFORM] Entity {:?} - update: parent={:?}->{:?}, size=({:?}, {:?}), min_size=({:?}, {:?}), has_bkg={}",
                    entity,
                    old_parent,
                    node.ui_transform.parent,
                    style.size.width,
                    style.size.height,
                    style.min_size.width,
                    style.min_size.height,
                    node.has_background
                );
                node.base_control.set_clip_contents(
                    node.ui_transform.overflow == crate::dcl::components::proto_components::sdk::components::YgOverflow::YgoHidden,
                );

                let opacity = pb_ui_transform.opacity.unwrap_or(1.0);
                let mut modulate = node.base_control.get_modulate();
                if modulate.a != opacity {
                    modulate.a = opacity;
                    node.base_control.set_modulate(modulate);
                }

                node.base_control
                    .bind_mut()
                    .set_pointer_filter(node.ui_transform.pointer_filter_mode);
            }
        }
    }
}
