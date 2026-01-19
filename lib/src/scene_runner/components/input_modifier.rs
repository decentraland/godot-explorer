use crate::{
    dcl::{
        components::{SceneComponentId, SceneEntityId},
        crdt::{
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
        SceneId,
    },
    godot_classes::dcl_global::DclGlobal,
    scene_runner::scene::{Scene, SceneType},
};

/// Updates the global input modifier state based on the InputModifier component
/// set on the PLAYER entity in the current parcel scene.
///
/// The InputModifier component allows scenes to disable specific player inputs:
/// - disable_all: Disables all inputs (walk, jog, run, jump, emote)
/// - disable_walk: Disables walk input
/// - disable_jog: Disables jog input
/// - disable_run: Disables run input
/// - disable_jump: Disables jump input
/// - disable_emote: Disables emote input
///
/// All booleans are false by default (no modification).
/// This component is only processed when set on the PLAYER entity (entity ID 1).
pub fn update_input_modifier(
    scene: &mut Scene,
    crdt_state: &mut SceneCrdtState,
    current_parcel_scene_id: &SceneId,
) {
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let is_current_parcel_scene = scene.scene_id == *current_parcel_scene_id;

    // Determine if we should process the InputModifier for this scene:
    // - For the current parcel scene: always apply (to handle scene entry)
    // - For global scenes: only when the component is dirty
    // - For other parcel scenes: ignore
    let should_process = match scene.scene_type {
        SceneType::Parcel => {
            if is_current_parcel_scene {
                // For current parcel scene, process on every tick to ensure
                // modifiers are applied immediately when entering the scene
                true
            } else {
                false
            }
        }
        SceneType::Global(_) => {
            // For global scenes, only process when component is dirty
            dirty_lww_components
                .get(&SceneComponentId::INPUT_MODIFIER)
                .is_some_and(|dirty| dirty.contains(&SceneEntityId::PLAYER))
        }
    };

    if !should_process {
        return;
    }

    // Get the InputModifier component for the PLAYER entity
    let input_modifier_component = SceneCrdtStateProtoComponents::get_input_modifier(crdt_state);
    let player_input_modifier = input_modifier_component.get(&SceneEntityId::PLAYER);

    // Get access to the global singleton to update input modifier state
    let Some(mut global) = DclGlobal::try_singleton() else {
        return;
    };

    let mut global_bind = global.bind_mut();

    // Extract the input modifier values, or reset if component was removed
    match player_input_modifier.and_then(|entry| entry.value.as_ref()) {
        Some(input_modifier) => {
            // Check if we have a standard input mode
            if let Some(standard) = &input_modifier.mode {
                match standard {
                    crate::dcl::components::proto_components::sdk::components::pb_input_modifier::Mode::Standard(standard_input) => {
                        global_bind.input_modifier_disable_all =
                            standard_input.disable_all.unwrap_or(false);
                        global_bind.input_modifier_disable_walk =
                            standard_input.disable_walk.unwrap_or(false);
                        global_bind.input_modifier_disable_jog =
                            standard_input.disable_jog.unwrap_or(false);
                        global_bind.input_modifier_disable_run =
                            standard_input.disable_run.unwrap_or(false);
                        global_bind.input_modifier_disable_jump =
                            standard_input.disable_jump.unwrap_or(false);
                        global_bind.input_modifier_disable_emote =
                            standard_input.disable_emote.unwrap_or(false);
                    }
                }
            } else {
                // No mode set, reset all modifiers (only if we're the current parcel scene)
                if is_current_parcel_scene {
                    global_bind.reset_input_modifiers();
                }
            }
        }
        None => {
            // Component was removed or not set, reset all modifiers (only if we're the current parcel scene)
            if is_current_parcel_scene {
                global_bind.reset_input_modifiers();
            }
        }
    }
}
