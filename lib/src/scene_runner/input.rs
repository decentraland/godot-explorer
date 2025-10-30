use std::collections::{HashMap, HashSet};

use godot::{classes::input::MouseMode, prelude::*};

use crate::{
    dcl::components::proto_components::sdk::components::common::InputAction,
    godot_classes::dcl_global::DclGlobal,
};

pub struct InputState {
    dcl_to_action: HashMap<InputAction, StringName>,
    pub state: HashMap<InputAction, bool>, // for now, only support bool states
}

impl InputState {
    pub fn default() -> Self {
        // TODO: should this be a constant?
        let dcl_to_action = HashMap::from([
            (InputAction::IaPointer, StringName::from("ia_pointer")),
            (InputAction::IaPrimary, StringName::from("ia_primary")),
            (InputAction::IaSecondary, StringName::from("ia_secondary")),
            (InputAction::IaForward, StringName::from("ia_forward")),
            (InputAction::IaBackward, StringName::from("ia_backward")),
            (InputAction::IaRight, StringName::from("ia_right")),
            (InputAction::IaLeft, StringName::from("ia_left")),
            (InputAction::IaJump, StringName::from("ia_jump")),
            (InputAction::IaWalk, StringName::from("ia_walk")),
            (InputAction::IaAction3, StringName::from("ia_action_3")),
            (InputAction::IaAction4, StringName::from("ia_action_4")),
            (InputAction::IaAction5, StringName::from("ia_action_5")),
            (InputAction::IaAction6, StringName::from("ia_action_6")),
        ]);

        let state = HashMap::from_iter(dcl_to_action.keys().map(|k| (*k, false)));

        Self {
            dcl_to_action,
            state,
        }
    }

    pub fn get_new_inputs(&mut self) -> HashSet<(InputAction, bool)> {
        let mut result = HashSet::new();
        let input: Gd<Input> = Input::singleton();

        // Desktop: Skip the actions when you´re without the pointer locked,
        // it means that you are interacting with the UI.
        // Mobile: Ignore that rule
        let is_pointer_locked = input.get_mouse_mode() == MouseMode::CAPTURED;
        if is_pointer_locked || DclGlobal::singleton().bind().is_mobile {
            for (input_action, action_string) in self.dcl_to_action.iter() {
                let current_state = input.is_action_pressed(action_string.clone());
                if self.state[input_action] != current_state {
                    self.state.insert(*input_action, current_state);
                    result.insert((*input_action, current_state));
                }
            }
        }
        result
    }
}
