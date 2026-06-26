use std::{cell::RefCell, rc::Rc, sync::atomic::Ordering};

use godot::{
    classes::{
        control::{FocusMode, MouseFilter},
        Control, IControl, InputEvent, InputEventMouseButton, InputEventScreenDrag,
        InputEventScreenTouch, Node,
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
    godot_classes::dcl_global::DclGlobal,
    scene_runner::{
        components::ui::scene_ui::UiResults,
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

    // Mobile swipe-off handoff: Godot drag-captures a touch to the control that
    // received the press, so once a finger presses this (interactive) control we
    // own the whole gesture. We track it here and, when the finger leaves our
    // rect, fire PetUp and forward the gesture to MobileCameraInput.
    pressed_touch_index: i32,
    press_position: Vector2,
    broke_out: bool,
    mobile_camera_input: Option<Gd<Node>>,
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
            ui_result: None,
            dcl_entity_id: SceneEntityId::ROOT,
            pressed_touch_index: -1,
            press_position: Vector2::ZERO,
            broke_out: false,
            mobile_camera_input: None,
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
        // On mobile we react to real touch events instead of the emulated mouse, because
        // mouse-emulation-from-touch is single-pointer: while a finger is holding the
        // joystick or rotating the camera it occupies the emulated mouse, so a second
        // finger over scene UI would never produce a mouse button event. Handling
        // InputEventScreenTouch directly lets each finger be hit-tested independently.
        // To avoid double-firing (both emulate_mouse_from_touch and
        // emulate_touch_from_mouse can be active) we gate by platform: touch on mobile,
        // mouse on desktop.
        let is_mobile = DclGlobal::singleton().bind().is_mobile;

        if is_mobile {
            match input.try_cast::<InputEventScreenTouch>() {
                Ok(event) => self.handle_touch(&event),
                Err(input) => {
                    if let Ok(event) = input.try_cast::<InputEventScreenDrag>() {
                        self.handle_drag(&event);
                    }
                }
            }
        } else if let Ok(event) = input.try_cast::<InputEventMouseButton>() {
            let is_left_button = event.get_button_index() == MouseButton::LEFT;
            let down_event = event.is_pressed();

            if self.listening_mouse_down && is_left_button && down_event {
                self.push_pointer_result(PointerEventType::PetDown);
            } else if self.listening_mouse_up && is_left_button && !down_event {
                self.push_pointer_result(PointerEventType::PetUp);
            }
        }

        // TODO: it enables HOVER and LEAVE events
        // if let Some(event) = input.try_cast::<InputEventMouseMotion>() {
        // }
    }

    fn handle_touch(&mut self, event: &Gd<InputEventScreenTouch>) {
        let index = event.get_index();
        if event.is_pressed() {
            if self.listening_mouse_down {
                self.push_pointer_result(PointerEventType::PetDown);
            }
            self.pressed_touch_index = index;
            self.press_position = self.to_global_position(event.get_position());
            self.broke_out = false;
        } else if index == self.pressed_touch_index {
            if self.broke_out {
                self.release_adopted_touch();
            } else if self.listening_mouse_up {
                self.push_pointer_result(PointerEventType::PetUp);
            }
            self.pressed_touch_index = -1;
            self.broke_out = false;
        }
    }

    fn handle_drag(&mut self, event: &Gd<InputEventScreenDrag>) {
        if event.get_index() != self.pressed_touch_index {
            return;
        }
        let global_position = self.to_global_position(event.get_position());
        if self.broke_out {
            self.update_adopted_touch(global_position, event.get_relative());
        } else if !self
            .base()
            .get_global_rect()
            .contains_point(global_position)
        {
            // Finger left the element: end the UI press (PetUp) and hand the
            // gesture off to the camera/joystick from the original touch point.
            self.broke_out = true;
            if self.listening_mouse_up {
                self.push_pointer_result(PointerEventType::PetUp);
            }
            let index = self.pressed_touch_index;
            let press_position = self.press_position;
            if let Some(mut mci) = self.get_mobile_camera_input() {
                mci.call(
                    "adopt_touch",
                    &[
                        index.to_variant(),
                        press_position.to_variant(),
                        global_position.to_variant(),
                        event.get_relative().to_variant(),
                    ],
                );
            }
        }
    }

    fn update_adopted_touch(&mut self, global_position: Vector2, relative: Vector2) {
        let index = self.pressed_touch_index;
        if let Some(mut mci) = self.get_mobile_camera_input() {
            mci.call(
                "update_adopted_touch",
                &[
                    index.to_variant(),
                    global_position.to_variant(),
                    relative.to_variant(),
                ],
            );
        }
    }

    fn release_adopted_touch(&mut self) {
        let index = self.pressed_touch_index;
        if let Some(mut mci) = self.get_mobile_camera_input() {
            mci.call("release_adopted_touch", &[index.to_variant()]);
        }
    }

    /// Converts a `gui_input` position (local to this control) into the global
    /// canvas space used by `get_global_rect` and the joystick zone test.
    fn to_global_position(&self, local_position: Vector2) -> Vector2 {
        self.base().get_global_transform() * local_position
    }

    fn get_mobile_camera_input(&mut self) -> Option<Gd<Node>> {
        if let Some(node) = self.mobile_camera_input.as_ref() {
            if node.is_instance_valid() {
                return Some(node.clone());
            }
        }
        let node = self
            .base()
            .get_node_or_null("/root/explorer/UI/MobileCameraInput")?;
        self.mobile_camera_input = Some(node.clone());
        Some(node)
    }

    fn push_pointer_result(&mut self, state: PointerEventType) {
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
