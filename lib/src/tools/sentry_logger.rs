//! Sentry integration for Rust-side error tracking and crash reporting.
//!
//! This module initializes the Sentry SDK in Rust, providing:
//! - Panic capture with full Rust stack traces
//! - Tracing integration for error/warning events
//! - Session and user ID synchronization with the Godot SDK

use std::sync::OnceLock;

use sentry::ClientInitGuard;
use tracing::Subscriber;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

/// The Sentry DSN for the Decentraland Godot Explorer project.
/// This is the same DSN used by the Godot Sentry SDK.
const SENTRY_DSN: &str =
    "https://03559fa545b3fa2bc9e876a41d6aab2f@o4510187684298752.ingest.us.sentry.io/4510187688361984";

/// Global guard to keep Sentry initialized for the lifetime of the application.
static SENTRY_GUARD: OnceLock<ClientInitGuard> = OnceLock::new();

/// Determines the environment based on the version string.
fn get_environment() -> &'static str {
    let version = env!("GODOT_EXPLORER_VERSION");
    if version.contains("-prod") {
        "production"
    } else if version.contains("-staging") {
        "staging"
    } else {
        "development"
    }
}

/// Checks if Sentry should be enabled (only for production and staging builds).
/// Use `--sentry-debug` CLI flag or `SENTRY_FORCE_ENABLE=1` env var to enable in dev builds.
pub fn is_sentry_enabled() -> bool {
    // Check if force-enabled via environment variable (useful for local testing)
    if std::env::var("SENTRY_FORCE_ENABLE").is_ok() {
        return true;
    }

    // Check if force-enabled via CLI flag (check raw args since DclCli isn't initialized yet)
    if check_cli_flag("--sentry-debug") {
        return true;
    }

    let version = env!("GODOT_EXPLORER_VERSION");
    // Dev builds should not send to Sentry by default
    !version.contains("-dev")
}

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

/// Initializes the Sentry SDK with the appropriate configuration.
/// This should be called once during application startup, before setting up the tracing subscriber.
pub fn init_sentry() {
    if !is_sentry_enabled() {
        return;
    }

    let release = format!(
        "org.decentraland.godotexplorer@{}",
        env!("GODOT_EXPLORER_VERSION")
    );

    let guard = sentry::init((
        SENTRY_DSN,
        sentry::ClientOptions {
            release: Some(release.into()),
            environment: Some(get_environment().into()),
            // Capture 100% of errors
            sample_rate: 1.0,
            // Enable session tracking
            auto_session_tracking: true,
            // Attach stack traces to all events
            attach_stacktrace: true,
            ..Default::default()
        },
    ));

    // Store the guard globally to prevent it from being dropped
    let _ = SENTRY_GUARD.set(guard);
}

/// Creates a tracing layer that forwards events to Godot's Sentry SDK as breadcrumbs.
/// This ensures all Rust logs appear as breadcrumbs in Godot Sentry events.
/// The Rust Sentry SDK is kept only for panic capture (better stack traces).
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
        let sentry_level = match level {
            Level::ERROR => "error",
            Level::WARN => "warning",
            Level::INFO => "info",
            Level::DEBUG => "debug",
            Level::TRACE => return, // Skip trace level
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
        if field.name() == "message" {
            self.message = format!("{:?}", value);
        } else if self.message.is_empty() {
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
    let breadcrumb_variant =
        ClassDb::singleton().class_call_static("SentryBreadcrumb", "create", &[message.to_variant()]);

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
/// Called automatically when --sentry-debug is enabled.
pub fn emit_sentry_test_messages() {
    tracing::trace!("[Sentry Test] This is a TRACE message - ignored");
    tracing::debug!("[Sentry Test] This is a DEBUG message - breadcrumb");
    tracing::info!("[Sentry Test] This is an INFO message - breadcrumb");
    tracing::warn!("[Sentry Test] This is a WARN message - breadcrumb");
    tracing::error!("[Sentry Test] This is an ERROR message - event");
}

/// Sets the Sentry user ID. Call this after the user is authenticated.
pub fn set_sentry_user(user_id: &str) {
    sentry::configure_scope(|scope| {
        scope.set_user(Some(sentry::User {
            id: Some(user_id.to_string()),
            ..Default::default()
        }));
    });
}

/// Sets the Sentry session ID tag. Call this when the session is created.
pub fn set_sentry_session_id(session_id: &str) {
    sentry::configure_scope(|scope| {
        scope.set_tag("dcl_session_id", session_id);
    });
}

/// Sets a custom tag on the Sentry scope.
pub fn set_sentry_tag(key: &str, value: &str) {
    sentry::configure_scope(|scope| {
        scope.set_tag(key, value);
    });
}
