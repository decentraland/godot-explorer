use crate::dcl::{
    components::SceneComponentId,
    crdt::{
        last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
        SceneCrdtStateProtoComponents,
    },
};
use crate::scene_runner::scene::Scene;

/// Applies the PBUiInputBinding component to scene UI controls. While a bound element is
/// pressed (mouse or touch), the listed InputActions are held down via Godot's Input,
/// driving both player movement and scene InputAction listeners — like the native buttons.
pub fn update_ui_input_binding(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;

    let Some(dirty) = dirty_lww_components.get(&SceneComponentId::UI_INPUT_BINDING) else {
        return;
    };

    let component = SceneCrdtStateProtoComponents::get_ui_input_binding(crdt_state);

    for entity in dirty {
        let new_value = component.get(entity).and_then(|v| v.value.clone());

        let godot_entity_node = godot_dcl_scene.ensure_godot_entity_node(entity);
        let Some(base_ui) = godot_entity_node.base_ui.as_mut() else {
            continue;
        };

        match new_value {
            Some(binding) => {
                base_ui
                    .base_control
                    .bind_mut()
                    .set_input_binding(&binding.actions);
            }
            None => {
                base_ui.base_control.bind_mut().clear_input_binding();
            }
        }
    }
}
