use std::{cell::RefCell, rc::Rc, sync::atomic::Ordering};

use godot::{
    engine::{
        control::{FocusMode, MouseFilter},
        global::MouseButton,
        Control, InputEvent, InputEventMouseButton,
    },
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
    scene_runner::{components::ui::scene_ui::UiResults, scene_manager::GLOBAL_TICK_NUMBER},
};

#[derive(GodotClass)]
#[class(base=Control)]
pub struct DclUiControl {
    #[base]
    base: Base<Control>,

    #[export]
    dcl_entity_id: SceneEntityId,

    ui_result: Option<Rc<RefCell<UiResults>>>,

    is_gui_input_signal_connected: bool,
    force_pointer_filter_mode: PointerFilterMode,

    listening_mouse_down: bool,
    listening_mouse_up: bool,
}

#[godot_api]
impl INode for DclUiControl {
    fn init(base: Base<Control>) -> Self {
        Self {
            base,
            is_gui_input_signal_connected: false,
            force_pointer_filter_mode: PointerFilterMode::PfmNone,
            listening_mouse_down: false,
            listening_mouse_up: false,
            ui_result: None,
            dcl_entity_id: SceneEntityId::ROOT,
        }
    }

    fn ready(&mut self) {
        self.base.set_focus_mode(FocusMode::FOCUS_NONE);
    }
}

#[godot_api]
impl DclUiControl {
    #[func]
    pub fn _on_gui_input(&mut self, input: Gd<InputEvent>) {
        let global_tick_number = GLOBAL_TICK_NUMBER.load(Ordering::Relaxed);
        if let Ok(event) = input.try_cast::<InputEventMouseButton>() {
            let is_left_button = event.get_button_index() == MouseButton::MOUSE_BUTTON_LEFT;
            let down_event = event.is_pressed();

            if self.listening_mouse_down && is_left_button && down_event {
                if let Some(ui_result) = self.ui_result.as_ref() {
                    ui_result.borrow_mut().pointer_event_results.push((
                        self.dcl_entity_id,
                        PbPointerEventsResult {
                            button: InputAction::IaPointer as i32,
                            hit: None,
                            state: PointerEventType::PetDown as i32,
                            timestamp: global_tick_number,
                            analog: None,
                            tick_number: global_tick_number,
                        },
                    ));
                }
            } else if self.listening_mouse_up && is_left_button && !down_event {
                if let Some(ui_result) = self.ui_result.as_ref() {
                    ui_result.borrow_mut().pointer_event_results.push((
                        self.dcl_entity_id,
                        PbPointerEventsResult {
                            button: InputAction::IaPointer as i32,
                            hit: None,
                            state: PointerEventType::PetUp as i32,
                            timestamp: global_tick_number,
                            analog: None,
                            tick_number: global_tick_number,
                        },
                    ));
                }
            }
        }

        // TODO: it enables HOVER and LEAVE events
        // if let Some(event) = input.try_cast::<InputEventMouseMotion>() {
        // }
    }

    pub fn update_mouse_filter(&mut self) {
        match self.force_pointer_filter_mode {
            PointerFilterMode::PfmNone => {
                if self.is_gui_input_signal_connected {
                    self.base.set_mouse_filter(MouseFilter::MOUSE_FILTER_STOP);
                } else {
                    self.base.set_mouse_filter(MouseFilter::MOUSE_FILTER_IGNORE);
                }
            }
            PointerFilterMode::PfmBlock => {
                self.base.set_mouse_filter(MouseFilter::MOUSE_FILTER_STOP);
            }
        }
    }

    fn set_connect_gui_input(&mut self, connect: bool) {
        if connect != self.is_gui_input_signal_connected {
            self.is_gui_input_signal_connected = connect;

            if connect {
                self.base
                    .clone()
                    .connect("gui_input".into(), self.base.callable("_on_gui_input"));
            } else {
                self.base
                    .clone()
                    .disconnect("gui_input".into(), self.base.callable("_on_gui_input"));
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

        self.set_connect_gui_input(self.listening_mouse_down || self.listening_mouse_up);
    }

    pub fn set_pointer_filter(&mut self, force_pointer_filter_mode: PointerFilterMode) {
        self.force_pointer_filter_mode = force_pointer_filter_mode;
        self.update_mouse_filter();
    }

    pub fn set_ui_result(&mut self, ui_result: Rc<RefCell<UiResults>>) {
        self.ui_result = Some(ui_result);
    }
}
