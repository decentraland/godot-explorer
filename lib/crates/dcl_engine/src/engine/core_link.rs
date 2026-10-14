//! The engine's end of the app state machine (`dcl_core::app`): it attaches a command queue to the
//! core, runs what the core sends on Godot's main thread, and reports loading back.

use std::sync::Arc;

use dcl_core::app::{CommandQueue, Destination, EngineCommand, EngineEvent, State};
use dcl_core::Core;
use godot::classes::RenderingServer;
use godot::prelude::*;

use super::DclEngine;

fn destination_args(destination: &Destination) -> VarDictionary {
    let (x, y) = destination.parcel.unwrap_or((0, 0));
    vdict! {
        "realm": destination.realm.clone().unwrap_or_default(),
        "x": x,
        "y": y,
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
    }
}

impl DclEngine {
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
