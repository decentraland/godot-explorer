//! The handles of the running world. The `Explorer` node holds none of them: everything that
//! reaches the world (scene fetcher, navigation, loading, the shell's controls) goes through
//! here, so no caller ever needs the node's borrow, which the node holds while it sets up.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use godot::classes::{Control, Label, Node3D};
use godot::prelude::*;

use super::explorer::Explorer;
use crate::app::position_broadcaster::PositionBroadcaster;
use crate::fetcher::loading_session::LoadingSession;
use crate::player::body::Player;
use crate::player::mobile_input::MobileCameraInput;
use crate::ui::emote_wheel::EmoteWheel;
use crate::ui::joypad::Joypad;
use crate::ui::joystick::VirtualJoystick;
use crate::ui::pointer_tooltip::PointerTooltip;
use crate::ui::safe_area::SafeMarginContainer;
use crate::ui::touch_controls::SdkTouchControlsApplier;

/// The nodes of `explorer.tscn` and what the world remembers between frames.
pub struct World {
    pub explorer: Gd<Explorer>,
    pub player: Gd<Player>,
    pub ui_root: Gd<Control>,
    pub scene_ui: Gd<Control>,
    pub crosshair: Gd<Label>,
    pub tooltip: Gd<PointerTooltip>,
    pub mobile_ui: Gd<Control>,
    pub camera_input: Gd<MobileCameraInput>,
    pub hud: Gd<SafeMarginContainer>,
    pub joystick: Gd<VirtualJoystick>,
    pub emote_wheel: Gd<EmoteWheel>,
    pub joypad: Gd<Joypad>,
    pub loading: Gd<LoadingSession>,
    pub broadcaster: Gd<PositionBroadcaster>,
    pub sdk_touch: Gd<SdkTouchControlsApplier>,

    pub parcel: Cell<Vector2i>,
    pub parcel_real: Cell<Vector2>,
    pub last_parcel: Cell<Vector2i>,
    pub dirty_save_position: Cell<bool>,
    pub disable_move_to: Cell<bool>,
    /// The shell's chat is focused: the movement controls and the emote wheel stay out of its way.
    pub chat_open: Cell<bool>,
    pub controls_hidden_for_hide_ui: Cell<bool>,
    pub last_outlined_avatar: RefCell<Option<Gd<Node3D>>>,
    pub last_outlined_entity: RefCell<Option<Gd<Node3D>>>,
}

thread_local! {
    static WORLD: RefCell<Option<Rc<World>>> = const { RefCell::new(None) };
}

/// The running world, `None` outside it or once its explorer is freed.
pub fn current() -> Option<Rc<World>> {
    WORLD.with(|world| {
        let mut slot = world.borrow_mut();
        if slot
            .as_ref()
            .is_some_and(|w| !w.explorer.is_instance_valid())
        {
            *slot = None;
        }
        slot.clone()
    })
}

pub(super) fn install(world: World) -> Rc<World> {
    let world = Rc::new(world);
    WORLD.with(|slot| *slot.borrow_mut() = Some(world.clone()));
    world
}

pub(super) fn clear() {
    WORLD.with(|slot| *slot.borrow_mut() = None);
}

/// Drops the world from the services' reach while its explorer is still alive.
pub(super) fn detach() {
    clear();
}

/// The parcel the player stands on; `Vector2i::ZERO` outside a world.
pub fn parcel_position() -> Vector2i {
    current().map(|w| w.parcel.get()).unwrap_or_default()
}

pub fn player() -> Option<Gd<Player>> {
    current().map(|w| w.player.clone())
}

pub fn loading() -> Option<Gd<LoadingSession>> {
    current().map(|w| w.loading.clone())
}
