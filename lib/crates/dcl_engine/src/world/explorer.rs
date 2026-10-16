//! The root of `explorer.tscn`: the world the engine runs, the player, the scene UI and the
//! on-screen controls. Every screen around it is the shell's. The node itself holds nothing
//! (see `state`); it builds the world once its children are ready and ticks it each frame.

use godot::classes::{Control, INode, InputEvent, Label, Node, PackedScene};
use godot::prelude::*;

use super::state::{self, World};
use super::{controls, tick};
use crate::app::graphics;
use crate::app::position_broadcaster::PositionBroadcaster;
use crate::fetcher::api::DclNavigator;
use crate::fetcher::globals;
use crate::fetcher::loading_session::LoadingSession;
use crate::godot_classes::dcl_global::DclGlobal;
use crate::godot_classes::dcl_realm_rules::DclRealmRules;
use crate::godot_classes::dcl_urls::DclUrls;
use crate::player::body::Player;
use crate::player::camera_controller::DclGlobalCameraController;
use crate::player::mobile_input::MobileCameraInput;
use crate::ui::emote_wheel::EmoteWheel;
use crate::ui::joypad::Joypad;
use crate::ui::joystick::VirtualJoystick;
use crate::ui::pointer_tooltip::PointerTooltip;
use crate::ui::safe_area::SafeMarginContainer;
use crate::ui::touch_controls::SdkTouchControlsApplier;
use crate::ui::ui_sounds;

/// Where a profile with no realm history boots. A dev realm, not genesis: this only happens on a
/// fresh install that has never joined anywhere.
const FALLBACK_BOOT_REALM: &str =
    "https://sdk-team-cdn.decentraland.org/ipfs/goerli-plaza-main-latest";
const PLAYER_SCENE: &str = "res://src/logic/player/player.tscn";
const EXPLORER_READY: &str = "on_explorer_ready";

#[derive(GodotClass)]
#[class(base=Node, init)]
pub struct Explorer {
    base: Base<Node>,
    /// The per-frame callback, disconnected when the explorer leaves the tree.
    frame: Option<Callable>,
}

#[godot_api]
impl INode for Explorer {
    fn ready(&mut self) {
        let explorer = self.to_gd();
        let Some(world) = build_world(&explorer) else {
            godot_error!("Explorer: explorer.tscn is missing a node, the world did not start");
            return;
        };
        let frame = Callable::from_fn("explorer_frame", |_| {
            tick::tick();
            Variant::nil()
        });
        self.base().get_tree().connect("process_frame", &frame);
        self.frame = Some(frame);
        start(explorer, &world);
    }

    fn exit_tree(&mut self) {
        if let Some(frame) = self.frame.take() {
            let mut tree = self.base().get_tree();
            if tree.is_connected("process_frame", &frame) {
                tree.disconnect("process_frame", &frame);
            }
        }
        state::clear();
    }

    fn unhandled_input(&mut self, event: Gd<InputEvent>) {
        controls::unhandled_input(&event);
    }
}

/// What the remaining GDScript tools (debug hub, scene tests, benchmark) read of the world; it
/// goes with them.
#[godot_api]
impl Explorer {
    #[func]
    fn get_player(&self) -> Option<Gd<Player>> {
        state::player()
    }

    #[func]
    fn get_ui_root(&self) -> Option<Gd<Control>> {
        state::current().map(|w| w.ui_root.clone())
    }

    #[func]
    fn get_parcel_position(&self) -> Vector2i {
        state::parcel_position()
    }

    #[func]
    fn set_visible_ui(&self, visible: bool) {
        if let Some(world) = state::current() {
            world.ui_root.clone().set_visible(visible);
        }
    }
}

fn node<T: GodotClass + Inherits<Node>>(explorer: &Gd<Explorer>, path: &str) -> Option<Gd<T>> {
    explorer.try_get_node_as::<T>(path)
}

/// Looks the scene's nodes up and instantiates the player under `%world`.
fn build_world(explorer: &Gd<Explorer>) -> Option<std::rc::Rc<World>> {
    let mut world_root: Gd<godot::classes::Node3D> = node(explorer, "%world")?;
    let player = load::<PackedScene>(PLAYER_SCENE).instantiate_as::<Player>();
    let mut broadcaster: Gd<PositionBroadcaster> = node(explorer, "%Timer_BroadcastPosition")?;
    let mut wheel: Gd<EmoteWheel> = node(explorer, "%EmoteWheel")?;
    let joystick: Gd<VirtualJoystick> = node(explorer, "%VirtualJoystick_Left")?;
    let crosshair: Gd<Label> = node(explorer, "%Label_Crosshair")?;
    let tooltip: Gd<PointerTooltip> = node(explorer, "%Control_PointerTooltip")?;

    let mut player_node = player.clone();
    player_node.set_name("Player");
    world_root.add_child(&player_node);
    broadcaster.bind_mut().player_node = Some(player.clone());
    if let Some(avatar) = player.bind().avatar.clone() {
        wheel.bind_mut().set_avatar_node(Some(avatar));
    }
    let sdk_touch = SdkTouchControlsApplier::create(
        Some(joystick.clone()),
        Some(crosshair.clone().upcast::<Control>()),
        Some(player.clone()),
        Some(tooltip.clone().upcast::<Control>()),
    );

    Some(state::install(World {
        explorer: explorer.clone(),
        player,
        ui_root: node(explorer, "%UI")?,
        scene_ui: node(explorer, "%SceneUIContainer")?,
        crosshair,
        tooltip,
        mobile_ui: node(explorer, "%MobileUI")?,
        camera_input: node::<MobileCameraInput>(explorer, "%MobileCameraInput")?,
        hud: node::<SafeMarginContainer>(explorer, "%SafeAreaHud")?,
        joystick,
        emote_wheel: wheel,
        joypad: node::<Joypad>(explorer, "%Joypad")?,
        loading: node::<LoadingSession>(explorer, "%LoadingSession")?,
        broadcaster,
        sdk_touch,
        parcel: Default::default(),
        parcel_real: Default::default(),
        last_parcel: std::cell::Cell::new(Vector2i::MAX),
        dirty_save_position: Default::default(),
        disable_move_to: Default::default(),
        chat_open: Default::default(),
        controls_hidden_for_hide_ui: Default::default(),
        last_outlined_avatar: Default::default(),
        last_outlined_entity: Default::default(),
    }))
}

/// Where this boot is heading: the command line, then a preview or deeplink, else nothing.
struct Destination {
    realm: Option<String>,
    location: Option<Vector2i>,
}

fn destination() -> Destination {
    let cli = globals::cli();
    let global = DclGlobal::singleton();
    let link = global.bind().deep_link_obj.clone();
    let link = link.bind();

    let mut realm = Some(cli.bind().realm.to_string()).filter(|r| !r.is_empty());
    let mut location = Some(cli.bind().get_location_vector()).filter(|l| *l != Vector2i::MAX);
    // A preview deeplink wins: its realm is the one hot reload develops against.
    for candidate in [link.preview.to_string(), link.realm.to_string()] {
        if realm.is_none() && !candidate.is_empty() {
            realm = Some(candidate);
        }
    }
    if link.is_location_defined() && location.is_none() {
        location = Some(link.location);
        if realm.is_none() {
            realm = Some(DclUrls::main_realm().to_string());
        }
    }
    Destination { realm, location }
}

fn setting_str(name: &str) -> String {
    globals::engine()
        .bind()
        .setting(name.into())
        .try_to::<GString>()
        .map(|s| s.to_string())
        .unwrap_or_default()
}

fn setting_i32(name: &str) -> i32 {
    globals::engine()
        .bind()
        .setting(name.into())
        .try_to::<i64>()
        .unwrap_or_default() as i32
}

/// Numeric settings arrive as ints or floats depending on the key.
fn setting_f32(name: &str) -> f32 {
    let value = globals::engine().bind().setting(name.into());
    value
        .try_to::<f64>()
        .or_else(|_| value.try_to::<i64>().map(|v| v as f64))
        .unwrap_or_default() as f32
}

/// What `_ready` did once the nodes exist: place the player, wire the world to the services and
/// start the navigation.
fn start(explorer: Gd<Explorer>, world: &World) {
    globals::metrics().bind_mut().set_flush_interval(10.0);
    graphics::apply_full_processor_mode();
    DclGlobal::set_orientation_landscape();
    ui_sounds::install_audio_recursive(&explorer.clone().upcast());
    let music = DclGlobal::singleton().bind().music_player.clone();
    if let Some(mut music) = music {
        music.bind_mut().stop();
    }
    controls::connect_signals(&explorer);

    // The episode is opened by the navigation below, which knows the realm this boot is actually
    // heading for: last_realm_joined is the wrong one for a deeplink.
    world.loading.bind().begin(
        GString::from(setting_str("last_realm_joined").as_str()),
        EXPLORER_READY.into(),
        false,
    );
    let destination = destination();
    let global = DclGlobal::singleton();
    let scene_inspector_file = {
        let g = global.bind();
        (g.deep_link_obj.bind().scene_inspector_file && !DclGlobal::is_production())
            || globals::cli().bind().scene_inspector_file
    };
    if scene_inspector_file {
        globals::scene_inspector_dispatcher()
            .bind_mut()
            .set_file_logging(true);
    }
    // After the initial setup, so a first resume does not teleport again.
    let mut router = global.bind().deep_link_router.clone();
    router.bind_mut().clear_deep_link();

    controls::prepare_controls(world);
    let start_parcel = destination.location.unwrap_or_else(|| {
        Vector2i::new(setting_i32("last_parcel_x"), setting_i32("last_parcel_y"))
    });
    place_player(world, start_parcel);
    connect_world(world);
    globals::scene_fetcher()
        .bind_mut()
        .update_position(start_parcel, true);
    navigate(destination.realm, start_parcel);
    globals::scene_runner().set_process_mode(godot::classes::node::ProcessMode::INHERIT);
    connect_identity(&explorer);
    finish(world, &explorer);
}

fn place_player(world: &World, parcel: Vector2i) {
    let mut player = world.player.clone();
    player.set_position(
        16.0 * Vector3::new(parcel.x as f32, 0.1, -(parcel.y as f32))
            + Vector3::new(8.0, 0.0, -8.0),
    );
    player.look_at(16.0 * Vector3::new(parcel.x as f32 + 1.0, 0.0, -(parcel.y as f32 + 1.0)));
}

/// Hands the player to the services that follow it and puts the scene UI under the HUD.
fn connect_world(world: &World) {
    let mut global = DclGlobal::singleton();
    if let Some(mut camera) = world.player.bind().camera.clone() {
        camera.set_far(setting_f32("view_distance"));
        global.bind_mut().player_camera_node = Some(camera);
    }
    let avatar = world.player.bind().avatar.clone();
    let mut runner = globals::scene_runner();
    {
        let mut runner = runner.bind_mut();
        runner.set_player_avatar_node(avatar.map(|a| a.upcast()));
        runner.set_player_body_node(Some(world.player.clone().upcast()));
        // The previous instance is freed with the old explorer on logout.
        runner.recreate_base_ui();
    }
    let base_ui = runner.bind().base_ui.clone();
    let mut scene_ui = world.scene_ui.clone();
    scene_ui.add_child(&base_ui);
    scene_ui.move_child(&base_ui, 0);
    controls::connect_world_signals(world);
}

fn navigate(realm: Option<String>, parcel: Vector2i) {
    let Some(realm) = realm else {
        let mut boot = setting_str("last_realm_joined");
        if boot.is_empty() {
            boot = FALLBACK_BOOT_REALM.to_string();
        }
        DclNavigator::go_restore(
            boot.as_str().into(),
            parcel.to_variant(),
            EXPLORER_READY.into(),
        );
        return;
    };
    let preview = DclGlobal::singleton()
        .bind()
        .deep_link_obj
        .bind()
        .preview
        .clone();
    if DclRealmRules::is_dcl_ens(realm.as_str().into()) && preview.is_empty() {
        DclNavigator::join_world(realm.as_str().into());
        return;
    }
    DclNavigator::go_restore(
        realm.as_str().into(),
        parcel.to_variant(),
        EXPLORER_READY.into(),
    );
    if !preview.is_empty() {
        globals::scene_fetcher()
            .bind_mut()
            .set_preview_url(realm.as_str().into());
    }
}

fn connect_identity(explorer: &Gd<Explorer>) {
    let identity = globals::player_identity();
    controls::connect_identity(explorer, &identity);
    // The profile may have been announced before this world existed: say it again, so the player
    // and the avatar scene take it.
    let profile = identity.bind().get_profile_or_null();
    if let Some(profile) = profile {
        crate::engine::local_task::defer(move || {
            let identity = identity;
            identity.signals().profile_changed().emit(&profile);
        });
    }
}

fn finish(world: &World, explorer: &Gd<Explorer>) {
    let identity = globals::player_identity();
    let (address, is_guest) = {
        let identity = identity.bind();
        (identity.get_address_str().to_string(), identity.is_guest)
    };
    if DclGlobal::singleton().bind().testing_scene_mode {
        identity.clone().bind_mut().create_disposable_account();
    }
    globals::metrics()
        .bind_mut()
        .update_identity(address, is_guest);
    globals::scene_runner().bind_mut().set_pause(false);
    let mut ui_root = world.ui_root.clone();
    crate::engine::local_task::defer(move || ui_root.grab_focus());
    let mut host = explorer.clone();
    host.add_child(&DclGlobalCameraController::new_alloc());
}
