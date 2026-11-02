use std::{rc::Rc, sync::Arc};

use dcl_core::app::{
    AvatarState, ChatLine, ChatProfile, CommandQueue, DisconnectReason, EngineEvent, EngineNotice,
    EntryRefusal, HudSnapshot, LoadingDetail, Parcel, UiState,
};

use godot::classes::{DisplayServer, Engine};
use godot::obj::Singleton;
use godot::prelude::*;

use super::scene_requests::{EmulatorSink, PendingResponses, SharedSink};

use super::local_task::spawn_local;
use crate::avatars::avatar::emote_load::async_play_emote_or_stash;
use crate::godot_classes::dcl_avatar::DclAvatar;
use crate::godot_classes::dcl_global::DclGlobal;

/// Owner of the engine-side handles. `Global.engine` exposes it.
#[derive(GodotClass)]
#[class(base=Node)]
pub struct DclEngine {
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
            pending: PendingResponses::default(),
            sink: Rc::new(EmulatorSink),
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

    /// A setting changed, whoever changed it (the shell through the core, or the engine itself).
    #[signal]
    pub fn setting_changed(name: GString);

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
        if let Some(core) = dcl_core::Core::try_get() {
            core.hub().set_report_tag("realm", &realm.to_string());
        }
        self.report(EngineEvent::LoadingFinished {
            realm: realm.to_string(),
            parcel: Parcel::new(x, y),
        });
    }

    /// Something the player has to hear about: `entry_private`, `entry_password` and
    /// `entry_not_found` (detail: the realm), `disconnected_duplicate`, `disconnected_room_closed`,
    /// `disconnected_other`, `disconnected_platform_ban`, `disconnect_recovered` (the connection came back
    /// while its notice was up), `banned` and `low_memory`.
    #[func]
    pub fn report_notice(&self, kind: GString, detail: GString) {
        let realm = detail.to_string();
        let notice = match kind.to_string().as_str() {
            "entry_private" => EngineNotice::EntryRefused {
                reason: EntryRefusal::PrivateWorld,
                realm,
            },
            "entry_password" => EngineNotice::EntryRefused {
                reason: EntryRefusal::NeedsPassword,
                realm,
            },
            "entry_not_found" => EngineNotice::EntryRefused {
                reason: EntryRefusal::NotFound,
                realm,
            },
            "disconnected_duplicate" => EngineNotice::Disconnected {
                reason: DisconnectReason::DuplicateSession,
            },
            "disconnected_room_closed" => EngineNotice::Disconnected {
                reason: DisconnectReason::RoomClosed,
            },
            "disconnected_other" => EngineNotice::Disconnected {
                reason: DisconnectReason::Other,
            },
            "disconnected_platform_ban" => EngineNotice::Disconnected {
                reason: DisconnectReason::PlatformBan,
            },
            "disconnect_recovered" => EngineNotice::DisconnectRecovered,
            "banned" => EngineNotice::Banned,
            "low_memory" => EngineNotice::LowMemory,
            other => {
                godot_error!("report_notice: unknown kind {other}");
                return;
            }
        };
        self.report(EngineEvent::Notice(notice));
    }

    #[func]
    pub fn report_scene_crashed(&self) {
        self.report(EngineEvent::SceneCrashed);
    }

    /// The player's whereabouts, a few times a second while in a world.
    pub fn report_hud(&self, snapshot: HudSnapshot) {
        self.report(EngineEvent::Hud(snapshot));
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

    /// A player setting or engine state read from the core's store; nil for an unknown name.
    #[func]
    pub fn setting(&self, name: GString) -> Variant {
        super::settings_link::get_variant(&name.to_string())
    }

    /// Stores a setting, applies it in the engine and, with `commit`, saves it.
    #[func]
    pub fn set_setting(&self, name: GString, value: Variant, commit: bool) {
        super::settings_link::set_variant(&name.to_string(), &value, commit, false);
    }

    /// Stores a setting the engine itself changed (a graphic profile, a benchmark result): it is
    /// announced to the shell but neither applied again nor saved; `save_settings` saves.
    #[func]
    pub fn adopt_setting(&self, name: GString, value: Variant) {
        super::settings_link::set_variant(&name.to_string(), &value, false, true);
    }

    #[func]
    pub fn save_settings(&self) {
        super::settings_link::save();
    }

    /// Whether the player accepted this build's terms.
    #[func]
    pub fn is_eula_accepted(&self) -> bool {
        dcl_core::ffi::eula_accepted()
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
        self.signals().text_input_focus().emit(focused, height);
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
            self.signals().text_input_focus().emit(true, height);
            self.keep_focused_input_above_keyboard();
        }
    }

    /// Slides the scene UI up by the part of the focused input the keyboard would cover, and
    /// back down when the keyboard goes away. The field never moves past the top of the screen.
    fn keep_focused_input_above_keyboard(&self) {
        let Some(world) = crate::world::state::current() else {
            return;
        };
        let mut container = world.scene_ui.clone();
        let mut offset = 0.0;
        if self.text_input_focused && self.keyboard_height > 0 {
            if let Some(viewport) = world.explorer.get_viewport() {
                let screen_height = viewport.get_visible_rect().size.y;
                let keyboard_top = screen_height
                    - crate::world::api::device_pixels_to_viewport_units(
                        self.keyboard_height as f32,
                    );
                let rect = self.focused_input_rect;
                let overlap = rect.end().y + KEYBOARD_MARGIN - keyboard_top;
                offset = overlap.clamp(0.0, rect.position.y.max(0.0));
            }
        }
        let x = container.get_position().x;
        container.set_position(Vector2::new(x, -offset));
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

    #[func]
    pub fn has_world(&self) -> bool {
        crate::world::state::current().is_some()
    }

    fn avatar(&self) -> Option<Gd<DclAvatar>> {
        crate::world::state::current()?.player.bind().avatar.clone()
    }

    pub fn touch_camera_input(&self) -> Option<Gd<crate::player::mobile_input::MobileCameraInput>> {
        crate::world::state::current().map(|w| w.camera_input.clone())
    }

    /// Whether the explorer UI has focus; `true` when there is no world to ask.
    pub fn ui_has_focus(&self) -> bool {
        crate::world::api::ui_has_focus()
    }

    // ---- facade for the host (the JSON bridge today, the shell later)

    /// Moves the local player to the centre of `parcel` in the current realm.
    #[func]
    pub fn teleport(&self, parcel: Vector2i) {
        crate::world::api::teleport_to(parcel);
    }

    /// Plays an emote on the local avatar.
    #[func]
    pub fn play_local_emote(&self, urn: GString) {
        if let Some(avatar) = self.avatar() {
            spawn_local(async_play_emote_or_stash(avatar, urn.to_string(), -1, -1));
        }
    }

    /// Where the shell's native controls sit, in device pixels: the HUD and scene UI keep clear
    /// of these margins.
    #[func]
    pub fn set_hud_insets(&self, top: f32, right: f32, bottom: f32, left: f32) {
        crate::world::api::set_hud_insets(top, right, bottom, left);
    }

    // ---- local player

    pub fn move_player_to(&self, position: Vector3, skip_loading: bool) {
        crate::world::api::move_to(position, skip_loading, true);
    }

    pub fn player_look_at(&self, target: Vector3) {
        crate::world::api::player_look_at(target);
    }

    pub fn camera_look_at(&self, target: Vector3) {
        crate::world::api::camera_look_at(target);
    }

    pub fn avatar_look_at_independent(&self, target: Vector3) {
        crate::world::api::avatar_look_at_independent(target);
    }

    // ---- local avatar emotes

    pub fn play_emote(&self, emote_id: &str, mask: i64, scene_id: i32) {
        if let Some(avatar) = self.avatar() {
            spawn_local(async_play_emote_or_stash(
                avatar,
                emote_id.to_string(),
                mask,
                scene_id,
            ));
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
            avatar.bind_mut().register_scene_emote_content(
                scene_entity_id,
                base_url,
                glb_hash,
                audio_hash,
            );
        }
    }

    pub fn stop_emote_from_scene(&self, scene_id: i32) {
        if let Some(mut avatar) = self.avatar() {
            avatar.bind_mut().stop_emote_from_scene(scene_id);
        }
    }
}
