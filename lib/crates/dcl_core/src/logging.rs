//! The process-wide `tracing` subscriber and panic hook, installed by `Core::install` so logs exist
//! before any engine. The engine adds its own sinks later with [`attach_engine_sinks`].

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Once, OnceLock, RwLock};

use tracing::Subscriber;
use tracing_subscriber::filter::{filter_fn, EnvFilter};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{reload, Layer, Registry};

use crate::Platform;

/// Logcat tag and os_log subsystem 1.x used (the old crate name); dashboards filter on it.
#[cfg(any(target_os = "android", target_os = "ios"))]
const LOG_NAME: &str = "dclgodot";

/// Receives every event that passes the filter, after the platform sink.
pub trait EventSink: Send + Sync {
    fn on_event(&self, event: &tracing::Event<'_>);
}

static FILTER: OnceLock<reload::Handle<EnvFilter, Registry>> = OnceLock::new();
static ENGINE_SINKS: RwLock<Vec<Box<dyn EventSink>>> = RwLock::new(Vec::new());
static PLATFORM_ENABLED: AtomicBool = AtomicBool::new(true);

/// Extracts the `message` field of an event.
#[derive(Default)]
pub struct MessageVisitor {
    pub message: String,
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

struct EngineSinkLayer;

impl<S: Subscriber> Layer<S> for EngineSinkLayer {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let sinks = ENGINE_SINKS.read().unwrap_or_else(|e| e.into_inner());
        for sink in sinks.iter() {
            sink.on_event(event);
        }
    }
}

fn default_filter(platform: Platform) -> &'static str {
    match platform {
        Platform::Desktop => "warn",
        Platform::Android | Platform::Ios => "info",
    }
}

#[cfg(target_os = "android")]
fn platform_layer<S>() -> impl Layer<S>
where
    S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    use tracing_subscriber::fmt::format::FmtSpan;
    paranoid_android::layer(LOG_NAME)
        .with_span_events(FmtSpan::CLOSE)
        .with_thread_names(true)
        .with_ansi(false)
}

/// Writes to Apple's Unified Logging. The `oslog` crate (unlike `tracing-oslog`) uses `%{public}s`,
/// so messages are visible in Console.app instead of `<private>`.
#[cfg(target_os = "ios")]
struct IosOsLogLayer {
    logger: oslog::OsLog,
}

#[cfg(target_os = "ios")]
impl<S: Subscriber> Layer<S> for IosOsLogLayer {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let metadata = event.metadata();
        let level = metadata.level();
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);
        let formatted = format!(
            "[{}] [{}] {} ({}:{})",
            level,
            metadata.target(),
            visitor.message,
            metadata.file().unwrap_or("unknown"),
            metadata.line().unwrap_or(0)
        );
        let oslog_level = match *level {
            tracing::Level::ERROR | tracing::Level::WARN => oslog::Level::Error,
            tracing::Level::INFO => oslog::Level::Info,
            tracing::Level::DEBUG | tracing::Level::TRACE => oslog::Level::Debug,
        };
        self.logger.with_level(oslog_level, &formatted);
    }
}

#[cfg(target_os = "ios")]
fn platform_layer<S: Subscriber>() -> impl Layer<S> {
    IosOsLogLayer {
        logger: oslog::OsLog::new(LOG_NAME, "default"),
    }
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn platform_layer<S>() -> impl Layer<S>
where
    S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    tracing_subscriber::fmt::layer()
}

fn build_subscriber(platform: Platform) -> impl Subscriber + Send + Sync {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(default_filter(platform)));
    let (filter_layer, handle) = reload::Layer::new(filter);
    let _ = FILTER.set(handle);

    let platform_layer =
        platform_layer().with_filter(filter_fn(|_| PLATFORM_ENABLED.load(Ordering::Relaxed)));

    Registry::default()
        .with(filter_layer)
        .with(platform_layer)
        .with(EngineSinkLayer)
}

fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let backtrace = std::backtrace::Backtrace::force_capture();
        tracing::error!(target: "panic", "{info}\n{backtrace}");
        previous(info);
    }));
}

/// Installs the global subscriber and the chaining panic hook. Idempotent. A no-op in this
/// crate's own tests, where `tracing-test` owns the global subscriber.
pub fn init(platform: Platform) {
    if cfg!(test) {
        return;
    }
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = tracing::subscriber::set_global_default(build_subscriber(platform));
        install_panic_hook();
    });
}

/// Replaces the engine sinks. With `mute_platform` the platform sink stops, for when the engine
/// routes the same events into Godot's own output (which already reaches logcat).
pub fn attach_engine_sinks(sinks: Vec<Box<dyn EventSink>>, mute_platform: bool) {
    *ENGINE_SINKS.write().unwrap_or_else(|e| e.into_inner()) = sinks;
    PLATFORM_ENABLED.store(!mute_platform, Ordering::Relaxed);
}

/// Changes the log filter at runtime; accepts any `EnvFilter` string, e.g. `dclgodot::comms=debug,warn`.
pub fn set_filter(filter: &str) -> Result<(), String> {
    let handle = FILTER
        .get()
        .ok_or_else(|| "logger not initialized".to_string())?;
    let parsed =
        EnvFilter::try_new(filter).map_err(|e| format!("invalid filter \"{filter}\": {e}"))?;
    handle
        .reload(parsed)
        .map_err(|e| format!("failed to reload the filter: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use std::sync::{Arc, Mutex};

    struct Counter(Arc<AtomicUsize>);

    impl EventSink for Counter {
        fn on_event(&self, _event: &tracing::Event<'_>) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    // One test: the sinks and the filter handle are process-wide.
    #[test]
    fn sinks_receive_events_and_filter_reloads() {
        let subscriber = build_subscriber(Platform::Desktop);
        tracing::subscriber::with_default(subscriber, || {
            let count = Arc::new(AtomicUsize::new(0));
            attach_engine_sinks(vec![Box::new(Counter(count.clone()))], true);
            set_filter("warn").unwrap();

            tracing::warn!("seen");
            tracing::debug!("filtered out");
            assert_eq!(count.load(Ordering::SeqCst), 1);

            set_filter("debug").unwrap();
            tracing::debug!("now seen");
            assert_eq!(count.load(Ordering::SeqCst), 2);

            assert!(set_filter("=not a filter=[").is_err());
            assert!(!PLATFORM_ENABLED.load(Ordering::Relaxed));
        });
        attach_engine_sinks(Vec::new(), false);
    }

    #[test]
    fn panic_hook_chains_to_the_previous_hook() {
        let called = Arc::new(Mutex::new(false));
        let seen = called.clone();
        let original = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |_| *seen.lock().unwrap() = true));

        install_panic_hook();
        let _ = std::panic::catch_unwind(|| panic!("boom"));

        std::panic::set_hook(original);
        assert!(*called.lock().unwrap());
    }
}
