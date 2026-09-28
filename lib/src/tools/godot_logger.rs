use std::sync::OnceLock;

use godot::classes::Os;
use godot::prelude::*;
use tracing::{Level, Subscriber};
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::{reload, Layer, Registry};

/// Global handle to swap the log filter at runtime.
static RELOAD_HANDLE: OnceLock<reload::Handle<EnvFilter, Registry>> = OnceLock::new();

/// Visitor to extract the message field from a tracing event.
/// Shared between GodotTracingLayer and IosOsLogLayer.
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

/// Log targets whose ERROR/WARN output is demoted to a plain print.
///
/// `dcl_rpc` is a git dependency we cannot patch. Its `StreamProtocol` and
/// `ClientMessagesHandler` errors fire once per inbound server message on a
/// stream whose consumer was already dropped, which is ~19% of all Sentry
/// volume and not actionable from here. The root cause (subscription streams
/// orphaned by the 30s resubscribe) is tracked separately in #2591.
const DEMOTED_TARGETS: [&str; 1] = ["dcl_rpc"];

fn is_demoted_target(target: &str) -> bool {
    DEMOTED_TARGETS
        .iter()
        .any(|demoted| target.starts_with(demoted))
}

/// A tracing Layer that forwards all log messages to Godot's print functions.
/// This unifies Rust logging output into the Godot console across all platforms.
pub struct GodotTracingLayer;

impl<S> Layer<S> for GodotTracingLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let metadata = event.metadata();
        let target = metadata.target();

        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);

        let file = metadata.file().unwrap_or("unknown");
        let line = metadata.line().unwrap_or(0);
        let msg = format!("[Rust:{}] {} ({}:{})", target, visitor.message, file, line);

        #[cfg(target_os = "android")]
        if !godot::sys::is_main_thread() {
            android_background::queue(msg, metadata);
            return;
        }

        emit_to_godot(&msg, metadata);
    }
}

fn emit_to_godot(msg: &str, metadata: &tracing::Metadata<'_>) {
    match *metadata.level() {
        // Demoted targets never reach Godot's error stream, which is what
        // the Sentry SDK captures. They stay in the logs and stay out of
        // the quota; `--rust-log` still shows them.
        Level::ERROR | Level::WARN if is_demoted_target(metadata.target()) => {
            godot_print!("{}", msg);
        }
        Level::ERROR => {
            print_error_with_source(msg, metadata);
        }
        Level::WARN => {
            print_warning_with_source(msg, metadata);
        }
        Level::INFO => godot_print!("{}", msg),
        Level::DEBUG => godot_print!("[DEBUG] {}", msg),
        Level::TRACE => godot_print!("[TRACE] {}", msg),
    }
}

/// Android: sentry-godot turns every Godot print into a breadcrumb through its
/// JNI plugin, and off the main thread that lookup can come back null and
/// crash in `SentryBreadcrumb::create` (GODOT-EXPLORER-2W5, 2CX, 2RP, 2WX,
/// 2P6, 2PW: livekit, tokio, memory-monitor threads). Events from other Rust
/// threads wait here and are printed on the main thread by
/// [`flush_background_logs`], which `ContentProvider::process` calls every
/// frame (it is in the tree from launch).
#[cfg(target_os = "android")]
mod android_background {
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    /// Bounds memory if the main thread stalls; the oldest lines go first.
    const MAX_PENDING: usize = 2048;

    static PENDING: Mutex<VecDeque<(String, &'static tracing::Metadata<'static>)>> =
        Mutex::new(VecDeque::new());
    static DROPPED: AtomicUsize = AtomicUsize::new(0);

    pub fn queue(msg: String, metadata: &'static tracing::Metadata<'static>) {
        let mut pending = PENDING.lock().unwrap_or_else(|e| e.into_inner());
        if pending.len() >= MAX_PENDING {
            pending.pop_front();
            DROPPED.fetch_add(1, Ordering::Relaxed);
        }
        pending.push_back((msg, metadata));
    }

    pub fn flush() {
        let pending = std::mem::take(&mut *PENDING.lock().unwrap_or_else(|e| e.into_inner()));
        let dropped = DROPPED.swap(0, Ordering::Relaxed);
        if dropped > 0 {
            godot::global::godot_print!(
                "[RustLogger] {} background-thread log lines dropped while the main thread was busy",
                dropped
            );
        }
        for (msg, metadata) in pending {
            super::emit_to_godot(&msg, metadata);
        }
    }
}

/// Prints the log lines other Rust threads queued (Android only; a no-op
/// elsewhere). Must be called on the main thread.
pub fn flush_background_logs() {
    #[cfg(target_os = "android")]
    android_background::flush();
}

/// Like `godot_error!` but with the real source location from tracing metadata,
/// so Sentry and the Godot debugger show the actual Rust call site.
fn print_error_with_source(msg: &str, metadata: &tracing::Metadata<'_>) {
    let file = metadata.file().unwrap_or("unknown");
    let line = metadata.line().unwrap_or(0) as i32;
    let function = metadata.target();

    let msg_c = format!("{}\0", msg);
    let func_c = format!("{}\0", function);
    let file_c = format!("{}\0", file);

    unsafe {
        godot::sys::interface_fn!(print_error)(
            godot::sys::c_str_from_str(&msg_c),
            godot::sys::c_str_from_str(&func_c),
            godot::sys::c_str_from_str(&file_c),
            line,
            false as godot::sys::GDExtensionBool,
        );
    }
}

/// Like `godot_warn!` but with the real source location from tracing metadata.
fn print_warning_with_source(msg: &str, metadata: &tracing::Metadata<'_>) {
    let file = metadata.file().unwrap_or("unknown");
    let line = metadata.line().unwrap_or(0) as i32;
    let function = metadata.target();

    let msg_c = format!("{}\0", msg);
    let func_c = format!("{}\0", function);
    let file_c = format!("{}\0", file);

    unsafe {
        godot::sys::interface_fn!(print_warning)(
            godot::sys::c_str_from_str(&msg_c),
            godot::sys::c_str_from_str(&func_c),
            godot::sys::c_str_from_str(&file_c),
            line,
            false as godot::sys::GDExtensionBool,
        );
    }
}

/// Collects Godot cmdline args (both regular and user args after `--`).
/// Works on all platforms including Android deeplinks and iOS.
pub fn get_godot_args() -> Vec<String> {
    let cmdline_args = Os::singleton().get_cmdline_args();
    let user_args = Os::singleton().get_cmdline_user_args();

    let mut args: Vec<String> = cmdline_args
        .to_vec()
        .iter()
        .map(|a| a.to_string())
        .collect();
    for arg in user_args.to_vec() {
        let s = arg.to_string();
        if !args.contains(&s) {
            args.push(s);
        }
    }
    args
}

/// Find the value for a `--key value` or `--key=value` arg from Godot cmdline args.
pub fn find_arg_value(args: &[String], name: &str) -> Option<String> {
    let prefix = format!("{}=", name);
    for (i, arg) in args.iter().enumerate() {
        if let Some(val) = arg.strip_prefix(&prefix) {
            return Some(val.to_string());
        }
        if arg == name {
            if let Some(next) = args.get(i + 1) {
                if !next.starts_with("--") {
                    return Some(next.clone());
                }
            }
        }
    }
    None
}

/// Check if `--no-pipe-logging` was passed via Godot cmdline args.
/// Returns `true` if logs should be piped to Godot (default behavior).
pub fn should_pipe_to_godot() -> bool {
    let args = get_godot_args();
    let should_pipe = !args.iter().any(|arg| arg == "--no-pipe-logging");
    godot_print!(
        "[RustLogger] should_pipe_to_godot={} (--no-pipe-logging {}found)",
        should_pipe,
        if should_pipe { "not " } else { "" }
    );
    should_pipe
}

/// Build an `EnvFilter` from `--rust-log` arg, `RUST_LOG` env var, or the given default.
///
/// Priority: `--rust-log <filter>` > `RUST_LOG` env var > `default_filter`.
/// On Android/iOS, `RUST_LOG` is typically unavailable so `--rust-log` via
/// deeplink is the primary way to override the default.
fn build_log_filter(default_filter: &str) -> EnvFilter {
    let args = get_godot_args();

    godot_print!(
        "[RustLogger] build_log_filter called with default_filter=\"{}\"",
        default_filter
    );
    godot_print!(
        "[RustLogger] Godot cmdline args ({} total): {:?}",
        args.len(),
        args
    );

    // 1. --rust-log flag (works on all platforms via deeplink or cmdline)
    if let Some(filter) = find_arg_value(&args, "--rust-log") {
        godot_print!(
            "[RustLogger] Found --rust-log arg, using filter: \"{}\"",
            filter
        );
        return EnvFilter::new(filter);
    }
    godot_print!("[RustLogger] No --rust-log arg found in cmdline args");

    // 2. RUST_LOG env var (mostly useful on desktop)
    match std::env::var("RUST_LOG") {
        Ok(val) => godot_print!("[RustLogger] RUST_LOG env var found: \"{}\"", val),
        Err(_) => godot_print!("[RustLogger] RUST_LOG env var not set"),
    }
    if let Ok(filter) = EnvFilter::try_from_default_env() {
        godot_print!("[RustLogger] Using RUST_LOG env var as filter");
        return filter;
    }

    // 3. Default
    godot_print!("[RustLogger] Using default filter: \"{}\"", default_filter);
    EnvFilter::new(default_filter)
}

/// Create a reloadable filter layer and store the handle for runtime updates.
///
/// The returned layer acts as a global filter applied to all layers in the subscriber.
/// Use [`set_log_filter`] to change the filter at runtime.
pub fn create_reload_filter(default_filter: &str) -> reload::Layer<EnvFilter, Registry> {
    godot_print!(
        "[RustLogger] create_reload_filter called with default_filter=\"{}\"",
        default_filter
    );
    let filter = build_log_filter(default_filter);
    godot_print!("[RustLogger] EnvFilter created: {:?}", filter);
    let (layer, handle) = reload::Layer::<EnvFilter, Registry>::new(filter);
    match RELOAD_HANDLE.set(handle) {
        Ok(()) => godot_print!("[RustLogger] Reload handle stored successfully"),
        Err(_) => godot_print!("[RustLogger] Reload handle was already set (duplicate init?)"),
    }
    layer
}

/// Change the log filter at runtime. Accepts any valid `EnvFilter` string.
///
/// Examples: `"debug"`, `"info"`, `"dclgodot::comms=debug,warn"`.
pub fn set_log_filter(filter_str: &str) -> Result<(), String> {
    godot_print!(
        "[RustLogger] set_log_filter called with: \"{}\"",
        filter_str
    );
    let handle = RELOAD_HANDLE.get().ok_or_else(|| {
        let msg = "Logger not initialized (RELOAD_HANDLE is empty)".to_string();
        godot_error!("[RustLogger] {}", msg);
        msg
    })?;
    let new_filter = EnvFilter::try_new(filter_str).map_err(|e| {
        let msg = format!("Invalid filter string \"{}\": {}", filter_str, e);
        godot_error!("[RustLogger] {}", msg);
        msg
    })?;
    godot_print!("[RustLogger] New EnvFilter parsed: {:?}", new_filter);
    handle.reload(new_filter).map_err(|e| {
        let msg = format!("Failed to reload filter: {}", e);
        godot_error!("[RustLogger] {}", msg);
        msg
    })?;
    godot_print!(
        "[RustLogger] Filter reloaded successfully to: \"{}\"",
        filter_str
    );
    Ok(())
}
