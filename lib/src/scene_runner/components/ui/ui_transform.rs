use godot::{classes::Node, obj::NewAlloc, prelude::Gd};

use crate::{
    dcl::{
        components::SceneComponentId,
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
    },
    godot_classes::dcl_ui_border::DclUiBorder,
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
                node.base_control.set_clip_contents(matches!(
                    node.ui_transform.overflow,
                    crate::dcl::components::proto_components::sdk::components::YgOverflow::YgoHidden
                        | crate::dcl::components::proto_components::sdk::components::YgOverflow::YgoScroll
                ));

                let opacity = pb_ui_transform.opacity.unwrap_or(1.0);
                let mut modulate = node.base_control.get_modulate();
                if modulate.a != opacity {
                    modulate.a = opacity;
                    node.base_control.set_modulate(modulate);
                }

                node.base_control
                    .bind_mut()
                    .set_pointer_filter(node.ui_transform.pointer_filter_mode);

                update_border_node(node);
                update_bkg_corner_radii(node);
            }
        }
    }
}

fn update_border_node(node: &mut crate::scene_runner::godot_dcl_scene::UiNode) {
    let want_border = node.ui_transform.has_border;

    if !want_border {
        if let Some(mut existing) = node.border_node.take() {
            existing.queue_free();
            node.base_control.remove_child(&existing.upcast::<Node>());
        }
        node.has_border = false;
        return;
    }

    // Use the cached Gd if present, otherwise create the border node. Newly
    // appended nodes land at the end of base_control's child list by default,
    // which is exactly where we want them (drawn on top of bkg/text/scene-children).
    // update_layout positions scene-children at indices < last_index every frame,
    // so the border floats to the tail naturally — no per-update move_child needed.
    let mut border = if let Some(existing) = node.border_node.clone() {
        existing
    } else {
        let mut new_node: Gd<DclUiBorder> = DclUiBorder::new_alloc();
        new_node.set_name("border");
        node.base_control
            .add_child(&new_node.clone().upcast::<Node>());
        node.border_node = Some(new_node.clone());
        new_node
    };

    border.bind_mut().set_border(
        node.ui_transform.border_widths,
        node.ui_transform.border_radii,
        node.ui_transform.border_colors,
    );
    node.has_border = true;
}

// Propagate border radii to the bkg child if it exists. Background may not exist yet
// on the first frame (update_ui_background runs afterwards and applies them itself),
// but on subsequent UI_TRANSFORM-only updates this is the only path that fires.
fn update_bkg_corner_radii(node: &mut crate::scene_runner::godot_dcl_scene::UiNode) {
    if !node.has_background {
        return;
    }
    if let Some(bkg) = node.bkg_node.as_mut() {
        bkg.bind_mut()
            .set_corner_radii(node.ui_transform.border_radii);
    }
}
