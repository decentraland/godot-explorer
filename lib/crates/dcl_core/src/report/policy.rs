use std::sync::Arc;

use super::event::{BuildEnvironment, Decision, ReportEvent, ReportLevel, ReportSource};

/// Share of sessions that upload the engine log.
pub const ATTACH_LOG_SAMPLE_RATE: f64 = 0.01;
/// Remote rate until the `sentry_sample_rate` flag loads, or when it is absent.
pub const DEFAULT_REMOTE_RATE: f64 = 1.0;

const RUST_SCENE_PREFIX: &str = "[Rust:dclgodot::dcl::js]";
const RUST_APP_PREFIX: &str = "[Rust:dclgodot";
const RUST_PREFIX: &str = "[Rust:";

/// Uniform draws in `[0, 1)`; injectable so tests are deterministic.
pub trait RandomSource: Send + Sync {
    fn next_f64(&self) -> f64;
}

pub struct ThreadRandom;

impl RandomSource for ThreadRandom {
    fn next_f64(&self) -> f64 {
        rand::random::<f64>()
    }
}

pub struct ReportPolicy {
    environment: BuildEnvironment,
    remote_rate: f64,
    error_events_enabled: bool,
    attach_log: bool,
    random: Arc<dyn RandomSource>,
}

impl ReportPolicy {
    /// Rolls the per-session log-upload sample once.
    pub fn new(environment: BuildEnvironment, random: Arc<dyn RandomSource>) -> Self {
        let attach_log = random.next_f64() < ATTACH_LOG_SAMPLE_RATE;
        Self {
            environment,
            remote_rate: DEFAULT_REMOTE_RATE,
            error_events_enabled: false,
            attach_log,
            random,
        }
    }

    pub fn with_thread_random(environment: BuildEnvironment) -> Self {
        Self::new(environment, Arc::new(ThreadRandom))
    }

    pub fn attach_log(&self) -> bool {
        self.attach_log
    }

    pub fn remote_rate(&self) -> f64 {
        self.remote_rate
    }

    /// From the `sentry_sample_rate` flag: clamped to `[0, 1]`, NaN falls back to the default.
    pub fn set_remote_rate(&mut self, rate: f64) {
        self.remote_rate = if rate.is_nan() {
            DEFAULT_REMOTE_RATE
        } else {
            rate.clamp(0.0, 1.0)
        };
    }

    /// From the `sentry_error_events` flag: lets non-fatal exception events through.
    pub fn set_error_events_enabled(&mut self, enabled: bool) {
        self.error_events_enabled = enabled;
    }

    pub fn decide(&self, event: &ReportEvent) -> Decision {
        if self.environment == BuildEnvironment::Development {
            return Decision::Drop;
        }
        match self.source_of(event) {
            Some(source) => self.sample(source),
            None => Decision::Drop,
        }
    }

    fn source_of(&self, event: &ReportEvent) -> Option<ReportSource> {
        if let Some(source) = event.source {
            return Some(source);
        }
        if event.is_crash || event.level == ReportLevel::Fatal {
            return Some(ReportSource::Crash);
        }
        match event.event_kind.as_deref() {
            Some("scene_crash") => return Some(ReportSource::SceneCrash),
            Some("exit_reason") => return Some(ReportSource::ExitReason),
            _ => {}
        }
        if event.category.as_deref() == Some("FEEDBACK") {
            return Some(ReportSource::Feedback);
        }
        let Some(value) = &event.exception else {
            return Some(ReportSource::Capture);
        };
        let text = if value.is_empty() {
            &event.message
        } else {
            value
        };
        firehose_source(text, self.error_events_enabled)
    }

    fn sample(&self, source: ReportSource) -> Decision {
        if self.remote_rate <= 0.0 {
            return Decision::Drop;
        }
        if !source.remote_rate_exempt() && self.random.next_f64() >= self.remote_rate {
            return Decision::Drop;
        }
        let keep_rate = source.keep_rate();
        if keep_rate < 1.0 && self.random.next_f64() >= keep_rate {
            return Decision::Drop;
        }
        Decision::Keep {
            source,
            attach_log: self.attach_log,
        }
    }
}

/// Source for a logged error with this text, or `None` to drop it. A use of a freed object is kept
/// even with the error-events flag off.
fn firehose_source(text: &str, error_events_enabled: bool) -> Option<ReportSource> {
    if is_freed_instance_error(text) {
        return Some(ReportSource::FreedInstance);
    }
    error_events_enabled.then(|| classify(text))
}

/// Rust-prefixed text is excluded: a scene's `console.error` arrives with that prefix.
fn is_freed_instance_error(text: &str) -> bool {
    !text.starts_with(RUST_PREFIX)
        && (text.contains("previously freed") || text.contains("a freed object"))
}

/// Most specific prefix first: scene, then our crate, then any dependency; the rest is the engine.
fn classify(text: &str) -> ReportSource {
    if text.starts_with(RUST_SCENE_PREFIX) {
        ReportSource::Scene
    } else if text.starts_with(RUST_APP_PREFIX) {
        ReportSource::RustApp
    } else if text.starts_with(RUST_PREFIX) {
        ReportSource::RustDep
    } else {
        ReportSource::Engine
    }
}
