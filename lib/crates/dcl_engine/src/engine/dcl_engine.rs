use std::{rc::Rc, sync::Arc};

use dcl_core::app::{
    AvatarState, ChatLine, ChatProfile, CommandQueue, EngineEvent, HudSnapshot, LoadingDetail,
    Parcel, UiState, Vec3,
};

use godot::classes::{Control, DisplayServer, Engine};
use godot::obj::Singleton;
use godot::prelude::*;

use super::scene_requests::{EmulatorSink, GodotUiSink, PendingResponses, SharedSink};

use crate::godot_classes::dcl_global::DclGlobal;

/// Nodes of the running world (`explorer.tscn`), registered by the explorer when it is ready and
/// dropped when it leaves the tree. Held by instance id: any of them can be freed between frames.
#[derive(Clone, Copy)]
struct WorldHandles {
    explorer: InstanceId,
    player: InstanceId,
    avatar: InstanceId,
    touch_camera_input: InstanceId,
    dialog_stack: InstanceId,
}

/// Owner of the engine-side handles. `Global.engine` exposes it; `explorer.gd` attaches the world.
#[derive(GodotClass)]
#[class(base=Node)]
pub struct DclEngine {
    world: Option<WorldHandles>,
    pub(super) pending: PendingResponses,
    pub(super) sink: SharedSink,
    /// Commands from the core's app; `Some` once this engine attached to it.
    pub(super) core_commands: Option<Arc<CommandQueue>>,
    /// Seconds until the next `[Perf]` line; `None` when `--perf-log` is off.
    perf_countdown: Option<f64>,
    /// A scene text input holds focus, and the keyboard height last reported for it.
    text_input_focused: bool,
    keyboard_height: i32,
    /// Global rect of the focused input when it took focus, in viewport units (before any shift).
    focused_input_rect: Rect2,
    base: Base<Node>,
}

/// Space kept between a focused input and the top of the keyboard, in viewport units.
const KEYBOARD_MARGIN: f32 = 16.0;

/// Seconds between `[Perf]` lines.
const PERF_LOG_INTERVAL_SECS: f64 = 5.0;

#[godot_api]
impl INode for DclEngine {
    fn init(base: Base<Node>) -> Self {
        Self {
            world: None,
            pending: PendingResponses::default(),
            sink: Rc::new(GodotUiSink),
            core_commands: None,
            perf_countdown: None,
            text_input_focused: false,
            keyboard_height: 0,
            focused_input_rect: Rect2::default(),
            base,
        }
    }

    fn ready(&mut self) {
        let perf_log = DclGlobal::try_singleton()
            .is_some_and(|global| global.bind().get_cli().bind().perf_log);
        self.perf_countdown = perf_log.then_some(PERF_LOG_INTERVAL_SECS);
        Self::register_core_services();
        if Self::core_is_driving() {
            self.attach_to_core();
        }
    }

    fn exit_tree(&mut self) {
        dcl_core::scene::set_screenshot_provider(None);
        self.detach_from_core();
    }

    fn process(&mut self, delta: f64) {
        self.run_core_commands();
        self.track_keyboard_height();
        let Some(countdown) = self.perf_countdown.as_mut() else {
            return;
        };
        *countdown -= delta;
        if *countdown > 0.0 {
            return;
        }
        *countdown = PERF_LOG_INTERVAL_SECS;
        if self.has_world() {
            crate::tools::memory_monitor::emit_log(&format!(
                "[Perf] fps={} rss_mb={}",
                Engine::singleton().get_frames_per_second(),
                crate::tools::memory_monitor::used_memory_mb()
            ));
        }
    }
}

fn resolve(id: InstanceId) -> Option<Gd<Node>> {
    Gd::<Node>::try_from_instance_id(id).ok()
}

#[godot_api]
impl DclEngine {
    /// A scene text input took or lost focus. `keyboard_height` is in device pixels and 0 while the
    /// keyboard is hidden or not yet up; it is sent again whenever it changes while focus holds.
    #[signal]
    pub fn text_input_focus(focused: bool, keyboard_height: i32);

    /// A command from the core's app (`enter_world`, `exit_world`, `show_avatar_preview`,
    /// `hide_avatar_preview` and one per shell control, see `core_link::control_payload`) with its
    /// typed arguments; the engine-side GDScript executes it.
    #[signal]
    pub fn core_command(kind: GString, args: VarDictionary);

    /// Whether the host booted the core before the engine started, i.e. the shell drives the app
    /// and the engine only renders. False for the emulator, tests and desktop runs.
    #[func]
    pub fn is_host_driven(&self) -> bool {
        Self::core_is_driving()
    }

    /// Loading progress of the world being entered, 0..1.
    #[func]
    pub fn report_loading_progress(&self, progress: f32) {
        self.report(EngineEvent::LoadingProgress(progress));
    }

    /// The world finished loading and the player is in it.
    #[func]
    pub fn report_loading_finished(&self, realm: GString, x: i32, y: i32) {
        self.report(EngineEvent::LoadingFinished {
            realm: realm.to_string(),
            parcel: Parcel::new(x, y),
        });
    }

    #[func]
    pub fn report_scene_crashed(&self) {
        self.report(EngineEvent::SceneCrashed);
    }

    /// The player's whereabouts, a few times a second while in a world.
    #[func]
    #[allow(clippy::too_many_arguments)]
    pub fn report_hud(&self, realm: GString, x: i32, y: i32, fps: f32, px: f32, py: f32, pz: f32) {
        self.report(EngineEvent::Hud(HudSnapshot {
            parcel: Parcel::new(x, y),
            realm: realm.to_string(),
            fps,
            position: Vec3 {
                x: px,
                y: py,
                z: pz,
            },
        }));
    }

    /// A chat line; `has_profile` tells whether `name`, `claimed` and `picture` resolve its sender.
    #[func]
    #[allow(clippy::too_many_arguments)]
    pub fn report_chat(
        &self,
        address: GString,
        message: GString,
        timestamp: f64,
        has_profile: bool,
        name: GString,
        claimed: bool,
        picture: GString,
    ) {
        self.report(EngineEvent::Chat(ChatLine {
            address: address.to_string(),
            message: message.to_string(),
            timestamp,
            profile: has_profile.then(|| ChatProfile {
                name: name.to_string(),
                has_claimed_name: claimed,
                profile_picture_url: picture.to_string(),
            }),
        }));
    }

    #[func]
    pub fn report_unread_chat(&self, count: i32) {
        self.report(EngineEvent::UnreadChat(count.max(0) as u32));
    }

    #[func]
    pub fn report_ui_state(&self, hidden: bool, chat_open: bool) {
        self.report(EngineEvent::UiState(UiState { hidden, chat_open }));
    }

    #[func]
    pub fn report_open_backpack(&self, emotes: bool) {
        self.report(EngineEvent::OpenBackpack { emotes });
    }

    #[func]
    pub fn report_loading_detail(&self, loaded: i64, total: i64, mbps: f32, percent: f32) {
        self.report(EngineEvent::LoadingDetail(LoadingDetail {
            loaded: loaded.max(0) as u64,
            total: total.max(0) as u64,
            mbps,
            percent,
        }));
    }

    /// The equipped avatar; colors are `#rrggbb`, an empty emote slot is `""`.
    #[func]
    pub fn report_avatar(
        &self,
        body_shape: GString,
        wearables: PackedStringArray,
        emotes: PackedStringArray,
        skin: GString,
        hair: GString,
        eyes: GString,
    ) {
        let strings = |array: &PackedStringArray| -> Vec<String> {
            array.as_slice().iter().map(|s| s.to_string()).collect()
        };
        self.report(EngineEvent::Avatar(AvatarState {
            body_shape: body_shape.to_string(),
            wearables: strings(&wearables),
            emotes: strings(&emotes),
            skin: skin.to_string(),
            hair: hair.to_string(),
            eyes: eyes.to_string(),
        }));
    }

    /// The engine owned by the `Global` autoload, `None` before it exists.
    pub fn get() -> Option<Gd<DclEngine>> {
        Some(DclGlobal::try_singleton()?.bind().get_engine())
    }

    /// Replaces who answers scene requests (the Godot UI, the emulator, or a test double).
    pub(crate) fn set_sink(&mut self, sink: SharedSink) {
        self.sink = sink;
    }

    /// Whether a scene text input holds focus: the explorer must not take focus back from it.
    #[func]
    pub fn is_text_input_focused(&self) -> bool {
        self.text_input_focused
    }

    /// Called by a scene text input on focus gained (with its rect) or lost.
    pub fn set_text_input_focus(&mut self, focused: bool, input_rect: Rect2) {
        self.text_input_focused = focused;
        if focused {
            self.focused_input_rect = input_rect;
        }
        self.keyboard_height = if focused {
            Self::keyboard_height_now()
        } else {
            0
        };
        let height = self.keyboard_height;
        self.base_mut().emit_signal(
            "text_input_focus",
            &[focused.to_variant(), height.to_variant()],
        );
        self.keep_focused_input_above_keyboard();
    }

    fn keyboard_height_now() -> i32 {
        DisplayServer::singleton().virtual_keyboard_get_height()
    }

    /// The keyboard takes a moment to slide up: report its height as it settles.
    fn track_keyboard_height(&mut self) {
        if !self.text_input_focused {
            return;
        }
        let height = Self::keyboard_height_now();
        if height != self.keyboard_height {
            self.keyboard_height = height;
            self.base_mut().emit_signal(
                "text_input_focus",
                &[true.to_variant(), height.to_variant()],
            );
            self.keep_focused_input_above_keyboard();
        }
    }

    /// Slides the scene UI up by the part of the focused input the keyboard would cover, and
    /// back down when the keyboard goes away. The field never moves past the top of the screen.
    fn keep_focused_input_above_keyboard(&self) {
        let Some(explorer) = self.explorer() else {
            return;
        };
        let Some(container) = explorer.get_node_or_null("UI/SceneUIContainer") else {
            return;
        };
        let Ok(mut container) = container.try_cast::<Control>() else {
            return;
        };
        let mut offset = 0.0;
        if self.text_input_focused && self.keyboard_height > 0 {
            if let Some(viewport) = explorer.get_viewport() {
                let screen_height = viewport.get_visible_rect().size.y;
                let keyboard_top = screen_height
                    - Self::device_pixels_to_viewport_units(&explorer, self.keyboard_height as f32);
                let rect = self.focused_input_rect;
                let overlap = rect.end().y + KEYBOARD_MARGIN - keyboard_top;
                offset = overlap.clamp(0.0, rect.position.y.max(0.0));
            }
        }
        let x = container.get_position().x;
        container.set_position(Vector2::new(x, -offset));
    }

    /// Registers the world once the explorer has built its player; the handles are dropped when
    /// the explorer leaves the tree.
    #[func]
    pub fn attach_world(&mut self, mut explorer: Gd<Node>) {
        let node_property =
            |owner: &Gd<Node>, name: &str| owner.get(name).try_to::<Gd<Node>>().ok();
        let (Some(player), Some(touch_camera_input), Some(dialog_stack)) = (
            node_property(&explorer, "player"),
            node_property(&explorer, "mobile_camera_input"),
            explorer.get_node_or_null("UI/DialogStack/Stack"),
        ) else {
            tracing::error!(
                "engine: the explorer is missing the player, camera input or dialog stack"
            );
            return;
        };
        let Some(avatar) = node_property(&player, "avatar") else {
            tracing::error!("engine: the player has no avatar");
            return;
        };
        self.world = Some(WorldHandles {
            explorer: explorer.instance_id(),
            player: player.instance_id(),
            avatar: avatar.instance_id(),
            touch_camera_input: touch_camera_input.instance_id(),
            dialog_stack: dialog_stack.instance_id(),
        });
        let detach = self.base().callable("detach_world");
        explorer.connect_flags(
            "tree_exiting",
            &detach,
            godot::classes::object::ConnectFlags::ONE_SHOT,
        );
    }

    #[func]
    pub fn detach_world(&mut self) {
        self.world = None;
    }

    /// Answers a scene request a sink left pending.
    #[func]
    pub fn answer_scene_request(&mut self, id: u32, accepted: bool) {
        if let Some(responder) = self.pending.take(id) {
            responder.send(if accepted {
                Ok(())
            } else {
                Err("the user declined".to_string())
            });
        }
    }

    /// The emulator accepts what a scene asks instead of showing a modal.
    #[func]
    pub fn set_emulator_mode(&mut self, enabled: bool) {
        self.sink = if enabled {
            Rc::new(EmulatorSink)
        } else {
            Rc::new(GodotUiSink)
        };
    }

    #[func]
    pub fn has_world(&self) -> bool {
        self.world
            .is_some_and(|world| resolve(world.explorer).is_some())
    }

    fn explorer(&self) -> Option<Gd<Node>> {
        resolve(self.world?.explorer)
    }

    pub fn player(&self) -> Option<Gd<Node>> {
        resolve(self.world?.player)
    }

    fn avatar(&self) -> Option<Gd<Node>> {
        resolve(self.world?.avatar)
    }

    pub fn touch_camera_input(&self) -> Option<Gd<Node>> {
        resolve(self.world?.touch_camera_input)
    }

    /// Where dialogs opened by scenes are parented.
    pub fn dialog_stack(&self) -> Option<Gd<Node>> {
        resolve(self.world?.dialog_stack)
    }

    /// Whether the explorer UI has focus; `true` when there is no world to ask.
    pub fn ui_has_focus(&self) -> bool {
        match self.explorer() {
            Some(mut explorer) if explorer.has_method("ui_has_focus") => {
                explorer.call("ui_has_focus", &[]).to::<bool>()
            }
            _ => true,
        }
    }

    // ---- facade for the host (the JSON bridge today, the shell later)

    /// Moves the local player to the centre of `parcel` in the current realm.
    #[func]
    pub fn teleport(&self, parcel: Vector2i) {
        if let Some(mut explorer) = self.explorer() {
            explorer.call("teleport_to", &[parcel.to_variant()]);
        }
    }

    /// Plays an emote on the local avatar.
    #[func]
    pub fn play_local_emote(&self, urn: GString) {
        if let Some(mut avatar) = self.avatar() {
            avatar.call("async_play_emote", &[urn.to_variant()]);
        }
    }

    /// Where the host's native controls sit, in device pixels: the HUD and scene UI keep clear
    /// of these margins.
    #[func]
    pub fn set_hud_insets(&self, top: f32, right: f32, bottom: f32, left: f32) {
        let Some(mut explorer) = self.explorer() else {
            return;
        };
        let Ok(mut hud) = explorer.get("safe_area_hud").try_to::<Gd<Control>>() else {
            return;
        };
        for (side, pixels) in [
            ("top", top),
            ("right", right),
            ("bottom", bottom),
            ("left", left),
        ] {
            let units = Self::device_pixels_to_viewport_units(&explorer, pixels);
            hud.add_theme_constant_override(&format!("margin_{side}"), units as i32);
        }
        explorer.call("_push_scene_interactable_area", &[]);
    }

    /// The host reports device pixels; Godot lays out in its scaled viewport units.
    fn device_pixels_to_viewport_units(explorer: &Gd<Node>, pixels: f32) -> f32 {
        let window_width = DisplayServer::singleton().window_get_size().x;
        let viewport_width = explorer
            .get_viewport()
            .map(|viewport| viewport.get_visible_rect().size.x);
        match viewport_width {
            Some(width) if window_width > 0 => pixels * width / window_width as f32,
            _ => pixels,
        }
    }

    // ---- local player

    pub fn move_player_to(&self, position: Vector3, skip_loading: bool) {
        if let Some(mut explorer) = self.explorer() {
            explorer.call(
                "move_to",
                &[position.to_variant(), skip_loading.to_variant()],
            );
        }
    }

    pub fn player_look_at(&self, target: Vector3) {
        if let Some(mut explorer) = self.explorer() {
            explorer.call("player_look_at", &[target.to_variant()]);
        }
    }

    pub fn camera_look_at(&self, target: Vector3) {
        if let Some(mut explorer) = self.explorer() {
            explorer.call("camera_look_at", &[target.to_variant()]);
        }
    }

    pub fn avatar_look_at_independent(&self, target: Vector3) {
        if let Some(mut explorer) = self.explorer() {
            explorer.call("avatar_look_at_independent", &[target.to_variant()]);
        }
    }

    // ---- local avatar emotes

    pub fn play_emote(&self, emote_id: &str, mask: i64, scene_id: i32) {
        if let Some(mut avatar) = self.avatar() {
            avatar.call(
                "async_play_emote",
                &[
                    emote_id.to_variant(),
                    mask.to_variant(),
                    scene_id.to_variant(),
                ],
            );
        }
    }

    /// Lets the avatar find the content of a scene emote by the scene's id.
    pub fn register_scene_emote_content(
        &self,
        scene_entity_id: &str,
        base_url: &str,
        glb_hash: &str,
        audio_hash: &str,
    ) {
        if let Some(mut avatar) = self.avatar() {
            avatar.call(
                "register_scene_emote_content",
                &[
                    scene_entity_id.to_variant(),
                    base_url.to_variant(),
                    glb_hash.to_variant(),
                    audio_hash.to_variant(),
                ],
            );
        }
    }

    pub fn stop_emote_from_scene(&self, scene_id: i32) {
        if let Some(mut avatar) = self.avatar() {
            avatar.call("stop_emote_from_scene", &[scene_id.to_variant()]);
        }
    }
}
