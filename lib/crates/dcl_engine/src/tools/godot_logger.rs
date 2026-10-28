use dcl_core::logging::EventSink;
pub use dcl_core::logging::MessageVisitor;
use godot::classes::Os;
use godot::global::{print_custom, PrintLevel, PrintRecord, PrintSource};
use godot::prelude::*;
use tracing::Level;

/// A core log sink that forwards all log messages to Godot's print functions.
/// This unifies Rust logging output into the Godot console across all platforms.
pub struct GodotTracingLayer;

impl EventSink for GodotTracingLayer {
    fn on_event(&self, event: &tracing::Event<'_>) {
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
        Level::ERROR => {
            print_with_source(PrintLevel::Error, msg, metadata);
        }
        Level::WARN => {
            print_with_source(PrintLevel::Warn, msg, metadata);
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
/// frame (it is in the tree from launch). They land up to a frame late, after
/// main-thread lines printed in between, so a logcat capture is not a strict
/// timeline across threads.
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

/// Like `godot_error!`/`godot_warn!` but with the real source location from tracing metadata,
/// so Sentry and the Godot debugger show the actual Rust call site.
fn print_with_source(level: PrintLevel, msg: &str, metadata: &tracing::Metadata<'_>) {
    print_custom(PrintRecord {
        level,
        message: msg,
        rationale: None,
        source: Some(PrintSource {
            function: metadata.target(),
            file: metadata.file().unwrap_or("unknown"),
            line: metadata.line().unwrap_or(0),
        }),
        editor_notify: false,
    });
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

/// Applies `--rust-log <filter>` over the filter the core installed (its default, or `RUST_LOG`).
/// On Android/iOS `RUST_LOG` is unavailable, so the argument (cmdline or deeplink) is the way in.
pub fn apply_cmdline_log_filter() {
    let args = get_godot_args();
    godot_print!(
        "[RustLogger] Godot cmdline args ({} total): {:?}",
        args.len(),
        args
    );
    if let Some(filter) = find_arg_value(&args, "--rust-log") {
        if let Err(e) = set_log_filter(&filter) {
            godot_error!("[RustLogger] {}", e);
        }
    }
}

/// Change the log filter at runtime. Accepts any valid `EnvFilter` string.
///
/// Examples: `"debug"`, `"info"`, `"dclgodot::comms=debug,warn"`.
pub fn set_log_filter(filter_str: &str) -> Result<(), String> {
    dcl_core::logging::set_filter(filter_str).inspect(|_| {
        godot_print!("[RustLogger] Filter set to: \"{}\"", filter_str);
    })
}
