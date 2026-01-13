//! Sentry integration for Rust-side error tracking.
//!
//! This module provides:
//! - Forwarding of Rust tracing logs to Godot's Sentry SDK as breadcrumbs
//! - Custom panic handler that captures stack traces and sends them to Sentry
//! - Session and user ID synchronization with the Godot SDK

use std::sync::atomic::{AtomicBool, Ordering};
use tracing::Subscriber;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

/// Flag to track if the panic handler has been installed
static PANIC_HANDLER_INSTALLED: AtomicBool = AtomicBool::new(false);

/// Check for a CLI flag in the raw command line arguments.
/// This is used before DclCli is initialized.
fn check_cli_flag(flag: &str) -> bool {
    use godot::classes::Os;
    use godot::obj::Singleton;

    let args = Os::singleton().get_cmdline_args();
    for arg in args.as_slice() {
        if arg.to_string() == flag {
            return true;
        }
    }
    false
}

/// Returns true if Sentry debug mode is enabled (via CLI flag or env var).
pub fn is_sentry_debug_mode() -> bool {
    std::env::var("SENTRY_FORCE_ENABLE").is_ok() || check_cli_flag("--sentry-debug")
}

/// Checks if Sentry should be enabled.
/// Returns true always - Sentry is enabled for all builds including dev.
pub fn is_sentry_enabled() -> bool {
    true
}

/// Installs the custom panic handler that sends panics to Godot's Sentry SDK.
/// This should be called once during application startup.
pub fn install_panic_handler() {
    // Only install once
    if PANIC_HANDLER_INSTALLED.swap(true, Ordering::SeqCst) {
        return;
    }

    let default_hook = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |panic_info| {
        // Capture the backtrace
        let backtrace = std::backtrace::Backtrace::force_capture();

        // Format the panic message
        let message = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic".to_string()
        };

        // Get location info
        let location = if let Some(loc) = panic_info.location() {
            format!("{}:{}:{}", loc.file(), loc.line(), loc.column())
        } else {
            "unknown location".to_string()
        };

        // Format the full error message with backtrace
        let full_message = format!(
            "Rust panic at {}: {}\n\nBacktrace:\n{}",
            location, message, backtrace
        );

        // Send to Godot's Sentry SDK
        send_panic_to_sentry(&full_message);

        // Call the default hook (which will print to stderr)
        default_hook(panic_info);
    }));
}

/// Sends a panic message to Godot's Sentry SDK as a fatal error.
fn send_panic_to_sentry(message: &str) {
    use godot::classes::Engine;
    use godot::prelude::*;

    // Get SentrySDK singleton
    let Some(mut sentry_sdk) = Engine::singleton().get_singleton("SentrySDK") else {
        eprintln!("[Rust Panic Handler] SentrySDK singleton not found, cannot report panic");
        return;
    };

    // SentrySDK.LEVEL_FATAL = 4
    let level_fatal: i64 = 4;

    // Capture the message as a fatal error
    sentry_sdk.call(
        "capture_message",
        &[message.to_variant(), level_fatal.to_variant()],
    );

    // Try to flush - give Sentry time to send before potential crash
    // Note: This may not work if the SDK doesn't expose a flush method
    if sentry_sdk.has_method("flush") {
        sentry_sdk.call("flush", &[]);
    }
}

/// Creates a tracing layer that forwards events to Godot's Sentry SDK as breadcrumbs.
/// This ensures all Rust logs appear as breadcrumbs in Godot Sentry events.
pub fn godot_sentry_layer() -> Option<GodotSentryLayer> {
    if is_sentry_enabled() {
        Some(GodotSentryLayer)
    } else {
        None
    }
}

/// A tracing layer that forwards log events to Godot's Sentry SDK as breadcrumbs.
pub struct GodotSentryLayer;

impl<S> Layer<S> for GodotSentryLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        use tracing::Level;

        let level = *event.metadata().level();

        // Map tracing level to Sentry level string
        // Skip DEBUG and TRACE to reduce noise in Sentry breadcrumbs
        let sentry_level = match level {
            Level::ERROR => "error",
            Level::WARN => "warning",
            Level::INFO => "info",
            Level::DEBUG | Level::TRACE => return, // Skip debug and trace levels
        };

        // Extract message from event
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);

        let target = event.metadata().target();
        let message = format!("[Rust:{}] {}", target, visitor.message);

        // Forward to Godot Sentry SDK
        add_breadcrumb_to_godot(&message, sentry_level);
    }
}

/// Visitor to extract the message field from a tracing event
#[derive(Default)]
struct MessageVisitor {
    message: String,
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" || self.message.is_empty() {
            self.message = format!("{:?}", value);
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" || self.message.is_empty() {
            self.message = value.to_string();
        }
    }
}

/// Adds a breadcrumb to the Godot Sentry SDK.
/// `level` should be one of: "error", "warning", "info", "debug"
fn add_breadcrumb_to_godot(message: &str, level: &str) {
    use godot::classes::{ClassDb, Engine};
    use godot::prelude::*;

    // Get SentrySDK singleton
    let Some(mut sentry_sdk) = Engine::singleton().get_singleton("SentrySDK") else {
        return;
    };

    // Map level string to SentrySDK level constants
    // SentrySDK.LEVEL_DEBUG = 0, LEVEL_INFO = 1, LEVEL_WARNING = 2, LEVEL_ERROR = 3, LEVEL_FATAL = 4
    let level_int: i64 = match level {
        "debug" => 0,
        "info" => 1,
        "warning" => 2,
        "error" => 3,
        _ => 1, // Default to INFO
    };

    // Call static method SentryBreadcrumb.create(message)
    let breadcrumb_variant = ClassDb::singleton().class_call_static(
        "SentryBreadcrumb",
        "create",
        &[message.to_variant()],
    );

    if breadcrumb_variant.is_nil() {
        return;
    }

    // Set properties and add to SentrySDK
    if let Ok(mut breadcrumb) = breadcrumb_variant.try_to::<Gd<Object>>() {
        breadcrumb.set("category", &"rust".to_variant());
        breadcrumb.set("level", &level_int.to_variant());
        breadcrumb.set("type", &"default".to_variant());
        sentry_sdk.call("add_breadcrumb", &[breadcrumb.to_variant()]);
    }
}

/// Emits test messages at various log levels to verify Sentry integration.
pub fn emit_sentry_test_messages() {
    tracing::trace!("[Sentry Test] This is a TRACE message - ignored");
    tracing::debug!("[Sentry Test] This is a DEBUG message - ignored");
    tracing::info!("[Sentry Test] This is an INFO message - breadcrumb");
    tracing::warn!("[Sentry Test] This is a WARN message - breadcrumb");
    tracing::error!("[Sentry Test] This is an ERROR message - breadcrumb");
}

/// Triggers a test panic to verify the panic handler.
pub fn trigger_test_panic() {
    panic!("Test panic triggered via trigger_test_panic()");
}

/// Sets the Sentry user ID by adding a breadcrumb (syncs with Godot SDK scope).
pub fn set_sentry_user(user_id: &str) {
    use godot::classes::Engine;
    use godot::prelude::*;

    let Some(mut sentry_sdk) = Engine::singleton().get_singleton("SentrySDK") else {
        return;
    };

    // Create a SentryUser and set it
    let user_variant = godot::classes::ClassDb::singleton().instantiate("SentryUser");
    if !user_variant.is_nil() {
        if let Ok(mut user) = user_variant.try_to::<Gd<RefCounted>>() {
            user.set("id", &user_id.to_variant());
            sentry_sdk.call("set_user", &[user.to_variant()]);
        }
    }
}

/// Sets a Sentry tag (syncs with Godot SDK scope).
pub fn set_sentry_tag(key: &str, value: &str) {
    use godot::classes::Engine;
    use godot::prelude::*;

    let Some(mut sentry_sdk) = Engine::singleton().get_singleton("SentrySDK") else {
        return;
    };

    sentry_sdk.call("set_tag", &[key.to_variant(), value.to_variant()]);
}

/// Sets the Sentry session ID tag.
pub fn set_sentry_session_id(session_id: &str) {
    set_sentry_tag("dcl_session_id", session_id);
}
