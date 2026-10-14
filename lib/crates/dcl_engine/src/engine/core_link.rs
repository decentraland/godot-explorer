//! The engine's end of the app state machine (`dcl_core::app`): it attaches a command queue to the
//! core, runs what the core sends on Godot's main thread, and reports loading back.

use std::sync::Arc;

use dcl_core::app::{
    CameraFocus, ColorTarget, CommandQueue, Control, Destination, EngineCommand, EngineEvent,
    Parcel, State,
};
use dcl_core::Core;
use godot::classes::{Os, RenderingServer};
use godot::prelude::*;

use super::{scene_link, screenshot_link, settings_link, social_link, DclEngine};

fn destination_args(destination: &Destination) -> VarDictionary {
    let parcel = destination.parcel.unwrap_or(Parcel::new(0, 0));
    vdict! {
        "realm": destination.realm.clone().unwrap_or_default(),
        "x": parcel.x,
        "y": parcel.y,
    }
}

/// `(kind, args)` of the `core_command` signal for a command, `None` for the ones Rust runs itself.
fn signal_payload(command: &EngineCommand) -> Option<(&'static str, VarDictionary)> {
    match command {
        EngineCommand::EnterWorld(destination) => {
            Some(("enter_world", destination_args(destination)))
        }
        EngineCommand::ExitWorld => Some(("exit_world", VarDictionary::new())),
        EngineCommand::ShowAvatarPreview(rect) => Some((
            "show_avatar_preview",
            vdict! {
                "x": rect.x,
                "y": rect.y,
                "w": rect.width,
                "h": rect.height,
            },
        )),
        EngineCommand::HideAvatarPreview => Some(("hide_avatar_preview", VarDictionary::new())),
        EngineCommand::SetRendering(_) => None,
        EngineCommand::Control(control) => Some(control_payload(control)),
    }
}

fn focus_name(focus: CameraFocus) -> &'static str {
    match focus {
        CameraFocus::Overall => "overall",
        CameraFocus::Head => "head",
        CameraFocus::Torso => "torso",
        CameraFocus::Legs => "legs",
        CameraFocus::Feet => "feet",
        CameraFocus::Hands => "hands",
    }
}

fn color_target_name(target: ColorTarget) -> &'static str {
    match target {
        ColorTarget::Skin => "skin",
        ColorTarget::Hair => "hair",
        ColorTarget::Eyes => "eyes",
    }
}

/// `(kind, args)` of the `core_command` signal for a shell control.
pub(super) fn control_payload(control: &Control) -> (&'static str, VarDictionary) {
    match control {
        Control::Teleport { parcel } => ("teleport", vdict! { "x": parcel.x, "y": parcel.y }),
        Control::SendChat { text } => ("send_chat", vdict! { "text": text.clone() }),
        Control::Emote { urn } => ("emote", vdict! { "urn": urn.clone() }),
        Control::SetHudInsets { insets } => (
            "set_hud_insets",
            vdict! {
                "top": insets.top,
                "right": insets.right,
                "bottom": insets.bottom,
                "left": insets.left,
            },
        ),
        Control::SetChatWidth { width } => ("set_chat_width", vdict! { "width": *width }),
        Control::ChatOpen { open } => ("chat_open", vdict! { "open": *open }),
        Control::HideUi { hidden } => ("hide_ui", vdict! { "hidden": *hidden }),
        Control::FocusCamera { focus } => ("focus_camera", vdict! { "focus": focus_name(*focus) }),
        Control::Equip { urn } => ("equip", vdict! { "urn": urn.clone() }),
        Control::Unequip { urn } => ("unequip", vdict! { "urn": urn.clone() }),
        Control::SetBodyShape { urn } => ("set_body_shape", vdict! { "urn": urn.clone() }),
        Control::SetColor { target, color } => (
            "set_color",
            vdict! { "target": color_target_name(*target), "color": color.clone() },
        ),
        Control::SetPreviewUrl { url } => ("set_preview_url", vdict! { "url": url.clone() }),
        Control::PlayPreviewEmote { urn } => ("play_preview_emote", vdict! { "urn": urn.clone() }),
        Control::SetEmoteSlot { slot, urn } => (
            "set_emote_slot",
            vdict! { "slot": *slot, "urn": urn.clone() },
        ),
        Control::RandomizeAvatar => ("randomize_avatar", VarDictionary::new()),
        Control::SetName { name } => ("set_name", vdict! { "name": name.clone() }),
    }
}

impl DclEngine {
    /// The device and build facts every crash report is tagged with, now that the GPU is known.
    fn set_report_context() {
        use crate::godot_classes::dcl_global::DclGlobal;
        use dcl_core::report::{BuildEnvironment, ReportContext};
        let Some(core) = Core::try_get() else {
            return;
        };
        let rendering = RenderingServer::singleton();
        let os = Os::singleton();
        let environment = if DclGlobal::is_production() {
            BuildEnvironment::Production
        } else if DclGlobal::is_staging() {
            BuildEnvironment::Staging
        } else {
            BuildEnvironment::Development
        };
        let physical_ram_bytes = os
            .get_memory_info()
            .get("physical")
            .and_then(|v| v.try_to::<i64>().ok());
        core.hub().set_report_context(
            ReportContext {
                platform: os.get_name().to_string(),
                debug_build: os.is_debug_build(),
                gpu_name: rendering.get_video_adapter_name().to_string(),
                gpu_vendor: rendering.get_video_adapter_vendor().to_string(),
                gpu_api_version: rendering.get_video_adapter_api_version().to_string(),
                locale: os.get_locale().to_string(),
                commit_hash: DclGlobal::get_commit_hash().to_string(),
                physical_ram_bytes,
                attach_branch_info: !DclGlobal::is_production(),
                branch_name: DclGlobal::get_branch_name().to_string(),
                commit_message: DclGlobal::get_commit_message().to_string(),
            },
            environment,
        );
    }

    /// Hands the core the engine's side of each service contract (settings applier, nearby
    /// avatars, scene facts, screenshots).
    pub(super) fn register_core_services() {
        Self::set_report_context();
        settings_link::register();
        social_link::register();
        scene_link::register();
        screenshot_link::register();
    }

    /// Registers this engine with the core's app. Called once the host-driven flow is running.
    pub(super) fn attach_to_core(&mut self) {
        let Some(core) = Core::try_get() else {
            godot_error!("DclEngine: no core to attach to");
            return;
        };
        let queue = Arc::new(CommandQueue::default());
        self.core_commands = Some(queue.clone());
        core.with_app(|app| app.engine_attach(queue));
    }

    pub(super) fn detach_from_core(&mut self) {
        if self.core_commands.take().is_some() {
            if let Some(core) = Core::try_get() {
                core.with_app(|app| app.engine_detach());
            }
        }
    }

    /// Runs the commands the core queued since the last frame.
    pub(super) fn run_core_commands(&mut self) {
        let Some(queue) = &self.core_commands else {
            return;
        };
        for command in queue.drain() {
            if let EngineCommand::SetRendering(on) = command {
                RenderingServer::singleton().set_render_loop_enabled(on);
            }
            if let Some((kind, args)) = signal_payload(&command) {
                self.base_mut()
                    .emit_signal("core_command", &[kind.to_variant(), args.to_variant()]);
            }
        }
    }

    pub(super) fn report(&self, event: EngineEvent) {
        if self.core_commands.is_some() {
            if let Some(core) = Core::try_get() {
                core.with_app(|app| app.engine_event(event));
            }
        }
    }

    /// Whether the host-driven flow is running: the host finished the core's boot before the engine
    /// started. Emulator and test runs never do, and keep the engine to themselves.
    pub(super) fn core_is_driving() -> bool {
        Core::try_get().is_some_and(|core| core.with_app(|app| *app.state() != State::Booting))
    }
}
