//! Exercises every logging stack (Rust, Swift, Kotlin, Objective-C) so each can be confirmed in the
//! unified channel; grep `[LOGTEST]`. The error and warning forms also run the Sentry pipeline, so
//! this never runs in production.

use godot::classes::Engine;
use godot::prelude::*;

use crate::godot_classes::dcl_android_plugin::DclAndroidPlugin;
use crate::godot_classes::dcl_global::DclGlobal;
use crate::godot_classes::dcl_swift_lib::DclSwiftLibPlugin;

/// Runs at most once, when `--test-logging` or `?test-logging=true` asked for it.
#[derive(Default)]
pub struct LoggingSelftest {
    done: bool,
}

/// Whether a deeplink param value asks for the test.
pub fn param_requests(value: &str) -> bool {
    value.eq_ignore_ascii_case("true")
}

impl LoggingSelftest {
    /// Called at boot (covers the CLI flag, as on iOS where the arg is baked) and on every deeplink
    /// arrival (Android delivers it late and its launcher strips command-line args).
    pub fn maybe_run(&mut self, cli_flag: bool, deeplink_param: &str) {
        if self.done || DclGlobal::is_production() {
            return;
        }
        if cli_flag || param_requests(deeplink_param) {
            self.done = true;
            run();
        }
    }
}

fn run() {
    godot_print!("[LOGTEST] ===== logging self-test start =====");
    DclGlobal::singleton().bind().test_logging();
    if DclSwiftLibPlugin::is_available() {
        godot_print!(
            "[LOGTEST][swift] DclSwiftLib.test_logging() -> {}",
            DclSwiftLibPlugin::test_logging()
        );
    }
    if DclAndroidPlugin::is_available() {
        DclAndroidPlugin::test_logging();
        godot_print!("[LOGTEST][kotlin] invoked DclAndroidPlugin.test_logging()");
    }
    // `has_singleton` first: asking for an absent one logs an error.
    if Engine::singleton().has_singleton("DclGodotiOS") {
        if let Some(mut ios) = Engine::singleton().get_singleton("DclGodotiOS") {
            if ios.has_method("test_logging") {
                ios.call("test_logging", &[]);
                godot_print!("[LOGTEST][objc] invoked DclGodotiOS.test_logging()");
            }
        }
    }
    godot_print!("[LOGTEST] ===== logging self-test end =====");
}

#[cfg(test)]
mod tests {
    use super::param_requests;

    #[test]
    fn only_true_requests_the_test() {
        assert!(param_requests("true"));
        assert!(param_requests("TRUE"));
        assert!(!param_requests("false"));
        assert!(!param_requests(""));
        assert!(!param_requests("1"));
    }
}
