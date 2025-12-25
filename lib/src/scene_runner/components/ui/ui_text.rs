use godot::{classes::Node, obj::NewAlloc, prelude::Gd};

use crate::{
    dcl::{
        components::SceneComponentId,
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
        ui_text_tags::{convert_unity_to_godot, ConversionResult},
    },
    godot_classes::{dcl_rich_ui_text::DclRichUiText, dcl_ui_text::DclUiText},
    scene_runner::scene::Scene,
};

pub fn update_ui_text(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let ui_text_component = SceneCrdtStateProtoComponents::get_ui_text(crdt_state);

    if let Some(dirty_ui_text) = dirty_lww_components.get(&SceneComponentId::UI_TEXT) {
        for entity in dirty_ui_text {
            let value = if let Some(entry) = ui_text_component.get(entity) {
                entry.value.clone()
            } else {
                None
            };

            let existing_ui_text = godot_dcl_scene
                .ensure_node_ui(entity)
                .base_ui
                .as_mut()
                .unwrap();

            if value.is_none() {
                if let Some(mut node) = existing_ui_text
                    .base_control
                    .get_node_or_null("text")
                {
                    node.queue_free();
                    existing_ui_text.base_control.remove_child(&node);
                }
                existing_ui_text.text_size = None;
                continue;
            }

            let value = value.as_ref().unwrap();

            // Check if text needs Unity to Godot BBCode conversion
            let text_str = value.value.as_str();
            let conversion_result = convert_unity_to_godot(text_str);

            let existing_node = existing_ui_text
                .base_control
                .get_node_or_null("text");

            // Determine if we need to swap the control type
            // Only swap when Modified and current node is not DclRichUiText
            let should_swap_control = matches!(&conversion_result, ConversionResult::Modified(_))
                && existing_node
                    .as_ref()
                    .map(|node| node.clone().try_cast::<DclRichUiText>().is_err())
                    .unwrap_or(false);

            // Swap control if needed (remove old node)
            if should_swap_control {
                if let Some(node) = existing_node.as_ref() {
                    node.clone().queue_free();
                    existing_ui_text.base_control.remove_child(&node.clone());
                }
            }

            // Create or reuse the appropriate control and update values
            match conversion_result {
                ConversionResult::Modified(converted_text) => {
                    // Create new DclRichUiText if needed
                    #[allow(clippy::unnecessary_unwrap)] // clippy is not taking the two brnaches
                    let mut rich_text_control = if should_swap_control || existing_node.is_none() {
                        let mut new_node: Gd<DclRichUiText> = DclRichUiText::new_alloc();
                        new_node.set_name("text");
                        new_node
                            .set_anchors_preset(godot::classes::control::LayoutPreset::FULL_RECT);
                        existing_ui_text
                            .base_control
                            .add_child(&new_node.clone().upcast::<Node>());
                        existing_ui_text
                            .base_control
                            .move_child(&new_node.clone().upcast::<Node>(), 1);
                        new_node
                    } else {
                        existing_node.unwrap().cast::<DclRichUiText>()
                    };

                    // Update the DclRichUiText with converted text
                    rich_text_control
                        .bind_mut()
                        .change_value(value, &converted_text);
                    existing_ui_text.text_size = Some(rich_text_control.get_minimum_size());
                }
                ConversionResult::NonModified => {
                    // For NonModified, we can use either DclUiText or DclRichUiText
                    // Only create a new DclUiText if there's no existing node
                    match existing_node {
                        Some(node) => {
                            // Reuse existing node, whether it's DclUiText or DclRichUiText
                            if let Ok(mut ui_text) = node.clone().try_cast::<DclUiText>() {
                                ui_text.bind_mut().change_value(value);
                                existing_ui_text.text_size = Some(ui_text.get_minimum_size());
                            } else if let Ok(mut rich_ui_text) =
                                node.clone().try_cast::<DclRichUiText>()
                            {
                                // Use the plain text (not converted) for RichTextLabel
                                rich_ui_text.bind_mut().change_value(value, text_str);
                                existing_ui_text.text_size = Some(rich_ui_text.get_minimum_size());
                            }
                        }
                        None => {
                            // Create new DclUiText
                            let mut new_node: Gd<DclUiText> = DclUiText::new_alloc();
                            new_node.set_name("text");
                            new_node.set_anchors_preset(
                                godot::classes::control::LayoutPreset::FULL_RECT,
                            );
                            existing_ui_text
                                .base_control
                                .add_child(&new_node.clone().upcast::<Node>());
                            existing_ui_text
                                .base_control
                                .move_child(&new_node.clone().upcast::<Node>(), 1);
                            new_node.bind_mut().change_value(value);
                            existing_ui_text.text_size = Some(new_node.get_minimum_size());
                        }
                    }
                }
            }
        }
    }
}
