//! Non-blocking shader/pipeline compilation (engine opt-in, Mobile renderer, Android/iOS only).
//! The remote flag is fail-closed and cannot be turned off once on. CLI `--async-shaders` /
//! `--no-async-shaders` and the deeplink `async-shaders=true/false` win over the flag.

use std::cell::Cell;

use dcl_core::Core;
use godot::classes::RenderingServer;
use godot::prelude::*;

pub const FLAG_ASYNC_SHADER_COMPILATION: &str = "async-shader-compilation";

thread_local! {
    static ENABLED: Cell<bool> = const { Cell::new(false) };
    /// The flag fetch ended, loaded or not (fail-closed either way).
    static FLAGS_SETTLED: Cell<bool> = const { Cell::new(false) };
}

pub fn is_enabled() -> bool {
    ENABLED.get()
}

pub fn on_flags_applied() {
    FLAGS_SETTLED.set(true);
    update();
}

/// Re-evaluated at boot, when the flags settle and on every deeplink.
pub fn update() {
    if ENABLED.get() {
        return;
    }
    let enabled = match super::feature_flags::local_override(
        "--async-shaders",
        "--no-async-shaders",
        "async-shaders",
    ) {
        Some(local) => local,
        None if FLAGS_SETTLED.get() => {
            dcl_core::flags::is_enabled(FLAG_ASYNC_SHADER_COMPILATION, false)
        }
        None => return,
    };
    let enabled = enabled && enable();
    if !enabled {
        godot_print!("[FeatureFlags] async shader compilation: disabled");
    }
    if let Some(core) = Core::try_get() {
        core.hub().set_report_tag(
            "async_shader_compilation",
            if enabled { "on" } else { "off" },
        );
    }
}

fn enable() -> bool {
    let threads = match std::env::consts::OS {
        "android" => 3,
        "ios" => 2,
        _ => return false,
    };
    RenderingServer::singleton()
        .enable_async_shader_compilation_ex()
        .shader_compile_threads(threads)
        .done();
    ENABLED.set(true);
    godot_print!("[FeatureFlags] async shader compilation: enabled ({threads} threads)");
    true
}
