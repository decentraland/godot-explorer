use tracing::{Level, Subscriber};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

/// A tracing Layer that forwards logs to Godot's Sentry SDK as breadcrumbs
pub struct SentryTracingLayer;

impl<S> Layer<S> for SentryTracingLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let level = *event.metadata().level();

        // Map tracing level to Sentry level string
        // Skip DEBUG and TRACE to reduce noise in Sentry breadcrumbs
        let sentry_level = match level {
            Level::ERROR => "error",
            Level::WARN => "warning",
            Level::INFO => "info",
            Level::DEBUG | Level::TRACE => return,
        };

        // Extract the message from the event
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);

        let target = event.metadata().target();
        let message = format!("[Rust:{}] {}", target, visitor.message);

        // Forward directly to Godot Sentry SDK as breadcrumb
        add_breadcrumb_to_godot(&message, sentry_level);
    }
}

/// Adds a breadcrumb to the Godot Sentry SDK.
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

/// Emits test messages at various log levels to verify Sentry integration.
pub fn emit_sentry_test_messages() {
    tracing::trace!("[Sentry Test] Rust: tracing::trace() - ignored");
    tracing::debug!("[Sentry Test] Rust: tracing::debug() - ignored");
    tracing::info!("[Sentry Test] Rust: tracing::info() - breadcrumb");
    tracing::warn!("[Sentry Test] Rust: tracing::warn() - breadcrumb");
    tracing::error!("[Sentry Test] Rust: tracing::error() - breadcrumb");
}
