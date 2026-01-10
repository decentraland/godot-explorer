use std::sync::Mutex;

use once_cell::sync::Lazy;
use tracing::{Level, Subscriber};
use tracing_subscriber::Layer;

/// Represents a log entry captured from tracing
#[derive(Clone, Debug)]
pub struct SentryLogEntry {
    pub level: SentryLogLevel,
    pub message: String,
    pub target: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SentryLogLevel {
    Error,
    Warn,
}

impl SentryLogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            SentryLogLevel::Error => "error",
            SentryLogLevel::Warn => "warning",
        }
    }
}

/// Global queue for Sentry log entries
static SENTRY_LOG_QUEUE: Lazy<Mutex<Vec<SentryLogEntry>>> = Lazy::new(|| Mutex::new(Vec::new()));

/// Maximum number of entries to keep in the queue to prevent unbounded growth
const MAX_QUEUE_SIZE: usize = 100;

/// A tracing Layer that captures error and warning events for Sentry
pub struct SentryTracingLayer;

impl<S> Layer<S> for SentryTracingLayer
where
    S: Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let level = *event.metadata().level();

        // Only capture error and warn levels
        let sentry_level = match level {
            Level::ERROR => SentryLogLevel::Error,
            Level::WARN => SentryLogLevel::Warn,
            _ => return,
        };

        // Extract the message from the event
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);

        let entry = SentryLogEntry {
            level: sentry_level,
            message: visitor.message,
            target: event.metadata().target().to_string(),
        };

        // Add to queue
        if let Ok(mut queue) = SENTRY_LOG_QUEUE.lock() {
            // Prevent unbounded growth by removing oldest entries
            if queue.len() >= MAX_QUEUE_SIZE {
                queue.remove(0);
            }
            queue.push(entry);
        }
    }
}

/// Drains and returns all pending log entries from the queue
pub fn drain_sentry_logs() -> Vec<SentryLogEntry> {
    if let Ok(mut queue) = SENTRY_LOG_QUEUE.lock() {
        std::mem::take(&mut *queue)
    } else {
        Vec::new()
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
            // Fallback: if no "message" field, use the first field
            if self.message.is_empty() {
                self.message = format!("{:?}", value);
            } else {
                self.message
                    .push_str(&format!(", {}={:?}", field.name(), value));
            }
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" || self.message.is_empty() {
            self.message = value.to_string();
        }
    }
}
