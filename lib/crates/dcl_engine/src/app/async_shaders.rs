//! Non-blocking shader/pipeline compilation (engine opt-in, Mobile renderer, Android/iOS only).
//! The remote flag is fail-closed and cannot be turned off once on. CLI `--async-shaders` /
//! `--no-async-shaders` and the deeplink `async-shaders=true/false` win over the flag.

use std::cell::Cell;

use dcl_core::Core;
use godot::prelude::*;

use super::deep_link::truthy;
use crate::engine::engine_fork;
use crate::fetcher::globals;
use crate::godot_classes::dcl_global::DclGlobal;

pub const FLAG_ASYNC_SHADER_COMPILATION: &str = "async-shader-compilation";

thread_local! {
    static ENABLED: Cell<bool> = const { Cell::new(false) };
    /// The flag fetch ended, loaded or not (fail-closed either way).
    static FLAGS_SETTLED: Cell<bool> = const { Cell::new(false) };
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
    let enabled = match local_override() {
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

fn local_override() -> Option<bool> {
    {
        let cli = globals::cli();
        let cli = cli.bind();
        if cli.has_arg("--no-async-shaders".into()) {
            return Some(false);
        }
        if cli.has_arg("--async-shaders".into()) {
            return Some(true);
        }
    }
    let link = DclGlobal::try_singleton()?.bind().deep_link_obj.clone();
    let value = link
        .bind()
        .get_params()
        .get("async-shaders")
        .map(|v| v.stringify().to_string())
        .unwrap_or_default();
    (!value.is_empty()).then(|| truthy(&value))
}

fn enable() -> bool {
    let threads = match std::env::consts::OS {
        "android" => 3,
        "ios" => 2,
        _ => return false,
    };
    if !engine_fork::enable_async_shader_compilation(threads) {
        return false;
    }
    ENABLED.set(true);
    godot_print!("[FeatureFlags] async shader compilation: enabled ({threads} threads)");
    true
}
