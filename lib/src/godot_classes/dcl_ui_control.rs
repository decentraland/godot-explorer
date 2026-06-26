use std::{cell::RefCell, rc::Rc, sync::atomic::Ordering};

use godot::{
    classes::{
        control::{FocusMode, MouseFilter},
        Control, IControl, Input, InputEvent, InputEventMouseButton, InputEventScreenTouch,
    },
    global::MouseButton,
    prelude::*,
};

use crate::{
    dcl::components::{
        proto_components::sdk::components::{
            common::{InputAction, PointerEventType},
            PbPointerEvents, PbPointerEventsResult, PointerFilterMode,
        },
        SceneEntityId,
    },
    scene_runner::{
        components::ui::scene_ui::UiResults,
        input::input_action_to_godot_action,
        scene_manager::{GLOBAL_TICK_NUMBER, GLOBAL_TIMESTAMP},
    },
};

#[derive(GodotClass)]
#[class(base=Control)]
pub struct DclUiControl {
    base: Base<Control>,

    #[export]
    dcl_entity_id: SceneEntityId,

    ui_result: Option<Rc<RefCell<UiResults>>>,

    is_gui_input_signal_connected: bool,
    force_pointer_filter_mode: PointerFilterMode,

    listening_mouse_down: bool,
    listening_mouse_up: bool,

    // Godot input actions (ia_*) fired while this element is pressed (PBUiInputBinding).
    bound_actions: Vec<StringName>,
    // Whether the bound actions are currently held (so we release on press-cancel/up).
    bound_actions_pressed: bool,
}

#[godot_api]
impl IControl for DclUiControl {
    fn init(base: Base<Control>) -> Self {
        Self {
            base,
            is_gui_input_signal_connected: false,
            force_pointer_filter_mode: PointerFilterMode::PfmNone,
            listening_mouse_down: false,
            listening_mouse_up: false,
            bound_actions: Vec::new(),
            bound_actions_pressed: false,
            ui_result: None,
            dcl_entity_id: SceneEntityId::ROOT,
        }
    }

    fn ready(&mut self) {
        self.base_mut().set_focus_mode(FocusMode::NONE);
    }
}

#[godot_api]
impl DclUiControl {
    #[func]
    pub fn _on_gui_input(&mut self, input: Gd<InputEvent>) {
        // Mouse button (desktop, and mobile when touch is emulated as mouse):
        // drives both the SDK pointer-event results and any bound input actions.
        if let Ok(event) = input.clone().try_cast::<InputEventMouseButton>() {
            if event.get_button_index() == MouseButton::LEFT {
                let down_event = event.is_pressed();
                self.push_pointer_result(down_event);
                self.press_bound_actions(down_event);
            }
            return;
        }

        // Screen touch (mobile native): drives bound input actions only.
        if let Ok(event) = input.try_cast::<InputEventScreenTouch>() {
            self.press_bound_actions(event.is_pressed());
        }

        // TODO: it enables HOVER and LEAVE events
        // if let Some(event) = input.try_cast::<InputEventMouseMotion>() {
        // }
    }

    fn push_pointer_result(&mut self, down_event: bool) {
        let state = if self.listening_mouse_down && down_event {
            PointerEventType::PetDown
        } else if self.listening_mouse_up && !down_event {
            PointerEventType::PetUp
        } else {
            return;
        };
        if let Some(ui_result) = self.ui_result.as_ref() {
            ui_result.borrow_mut().pointer_event_results.push((
                self.dcl_entity_id,
                PbPointerEventsResult {
                    button: InputAction::IaPointer as i32,
                    hit: None,
                    state: state as i32,
                    timestamp: GLOBAL_TIMESTAMP.fetch_add(1, Ordering::Relaxed),
                    analog: None,
                    tick_number: GLOBAL_TICK_NUMBER.load(Ordering::Relaxed),
                },
            ));
        }
    }

    /// Press or release all bound input actions (PBUiInputBinding) on this element,
    /// like the native on-screen buttons. Idempotent: holds state to avoid double press/release.
    fn press_bound_actions(&mut self, press: bool) {
        if self.bound_actions.is_empty() || press == self.bound_actions_pressed {
            return;
        }
        self.bound_actions_pressed = press;
        let mut input = Input::singleton();
        for action in self.bound_actions.iter() {
            if press {
                input.action_press(action);
            } else {
                input.action_release(action);
            }
        }
    }

    pub fn update_mouse_filter(&mut self) {
        match self.force_pointer_filter_mode {
            PointerFilterMode::PfmNone => {
                if self.is_gui_input_signal_connected {
                    self.base_mut().set_mouse_filter(MouseFilter::STOP);
                } else {
                    self.base_mut().set_mouse_filter(MouseFilter::IGNORE);
                }
            }
            PointerFilterMode::PfmBlock => {
                self.base_mut().set_mouse_filter(MouseFilter::STOP);
            }
        }
    }

    fn set_connect_gui_input(&mut self, connect: bool) {
        if connect != self.is_gui_input_signal_connected {
            self.is_gui_input_signal_connected = connect;

            let callable_on_gui_input = self.base().callable("_on_gui_input").clone();
            if connect {
                self.base_mut().connect("gui_input", &callable_on_gui_input);
            } else {
                self.base_mut()
                    .disconnect("gui_input", &callable_on_gui_input);
            }
            self.update_mouse_filter();
        }
    }

    pub fn set_pointer_events(&mut self, pb_pointer_events: &Option<PbPointerEvents>) {
        let Some(pb_pointer_events) = pb_pointer_events.as_ref() else {
            self.set_connect_gui_input(false);
            return;
        };

        self.listening_mouse_down = pb_pointer_events.pointer_events.iter().any(|pe| {
            pe.event_type() == PointerEventType::PetDown
                && pe
                    .event_info
                    .as_ref()
                    .map(|ei| ei.button() == InputAction::IaPointer)
                    .unwrap_or(false)
        });

        self.listening_mouse_up = pb_pointer_events.pointer_events.iter().any(|pe| {
            pe.event_type() == PointerEventType::PetUp
                && pe
                    .event_info
                    .as_ref()
                    .map(|ei| ei.button() == InputAction::IaPointer)
                    .unwrap_or(false)
        });

        self.refresh_gui_input_connection();
    }

    /// gui_input must be connected when we either report pointer events or have bound actions.
    fn refresh_gui_input_connection(&mut self) {
        let want =
            self.listening_mouse_down || self.listening_mouse_up || !self.bound_actions.is_empty();
        self.set_connect_gui_input(want);
    }

    /// Sets the input actions bound to this UI element (PBUiInputBinding). Unmapped
    /// actions (IaAny / IaModifier) are skipped with a warning.
    pub fn set_input_binding(&mut self, actions: &[i32]) {
        // Release any currently held actions before rebinding to avoid stuck inputs.
        self.press_bound_actions(false);

        self.bound_actions = actions
            .iter()
            .filter_map(|raw| {
                let action = InputAction::from_i32(*raw)?;
                match input_action_to_godot_action(action) {
                    Some(name) => Some(StringName::from(name)),
                    None => {
                        godot_warn!(
                            "PBUiInputBinding: input action {:?} has no bindable mapping, ignoring",
                            action
                        );
                        None
                    }
                }
            })
            .collect();
        self.refresh_gui_input_connection();
    }

    /// Clears all bound input actions, releasing any that are currently held.
    pub fn clear_input_binding(&mut self) {
        self.press_bound_actions(false);
        self.bound_actions.clear();
        self.refresh_gui_input_connection();
    }

    pub fn set_pointer_filter(&mut self, force_pointer_filter_mode: PointerFilterMode) {
        self.force_pointer_filter_mode = force_pointer_filter_mode;
        self.update_mouse_filter();
    }

    pub fn set_ui_result(&mut self, ui_result: Rc<RefCell<UiResults>>) {
        self.ui_result = Some(ui_result);
    }
}
