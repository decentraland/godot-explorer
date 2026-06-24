//! Log capture for the unified scene-inspector channel.
//!
//! Captures logs from every source and folds them into the scene-inspector
//! stream as `"log"` entries (`crate::tools::scene_inspector::emit_log`). There
//! is no separate transport here: the scene-inspector bridge connects out to the
//! debug-hub, and `emit_log` is connection-gated, so nothing is captured until a
//! consumer subscribes.
//!
//! Logs live in several channels — Rust (`tracing`), GDScript (`print` /
//! `push_error`), the Godot engine, and native Swift/ObjC (`NSLog` / `printf`).
//! Sinks:
//!  1. `DclLogHubLogger` — a Godot `Logger` (`OS.add_logger`). Godot calls it for
//!     every `print` / `push_error` / engine message, capturing GDScript,
//!     `godot_print!` (Rust routed through Godot's logger) and engine logs on
//!     every platform.
//!  2. `LogHubLayer` — a `tracing` layer for structured Rust events (all levels,
//!     with file/line); the Rust path on iOS, where `godot_print` never reaches
//!     stdout.
//!  3. iOS-only fd capture — an stdout/stderr redirect for native `printf` /
//!     Swift `print` (which bypass Godot's logger), teed back to the real fds so
//!     `--console` keeps working.

use std::sync::atomic::{AtomicBool, Ordering};

use godot::classes::{ILogger, Logger, Os};
use godot::prelude::*;

// ---------------------------------------------------------------------------
// Godot logger sink: captures GDScript + godot_print! + engine + errors.
// ---------------------------------------------------------------------------

/// A Godot `Logger` that folds every engine/GDScript/Rust message into the
/// scene-inspector stream.
#[derive(GodotClass)]
#[class(init, base = Logger)]
struct DclLogHubLogger {
    base: Base<Logger>,
}

#[godot_api]
impl ILogger for DclLogHubLogger {
    fn log_message(&mut self, message: GString, error: bool) {
        // Cross-platform tap: sees GDScript/engine messages and (on desktop /
        // Android, where LogHubLayer is absent) Rust logs routed via godot_print!.
        let level = if error { Some("error") } else { None };
        // A single message may contain several lines (and a trailing newline).
        for line in message.to_string().lines() {
            // On iOS the LogHubLayer already emits Rust events (prefixed "[Rust:");
            // skip the godot_print! copy here to avoid duplicating them.
            if RUST_VIA_LAYER.load(Ordering::Relaxed) && line.starts_with("[Rust:") {
                continue;
            }
            crate::tools::scene_inspector::emit_log(
                "godot",
                level,
                None,
                None,
                None,
                line.to_string(),
            );
        }
    }

    // NOTE: `log_error` is intentionally NOT overridden. Its
    // `Array<Gd<ScriptBacktrace>>` parameter triggers a gdext class-id panic on
    // iOS. Rust errors/warnings are captured via `LogHubLayer`; GDScript `print`
    // still flows through `log_message`.
}

/// True once `LogHubLayer` is feeding Rust events, so `log_message` can drop the
/// duplicate `godot_print!` copies on iOS.
static RUST_VIA_LAYER: AtomicBool = AtomicBool::new(false);

/// A `tracing` layer that folds every Rust event into the scene-inspector stream
/// as a structured `"rust"` log entry. Added to the subscriber in
/// `dcl_global::*::init_logger`; on iOS it's the Rust path since `godot_print`
/// doesn't reach stdout there.
pub struct LogHubLayer;

impl LogHubLayer {
    pub fn new() -> Self {
        RUST_VIA_LAYER.store(true, Ordering::Relaxed);
        LogHubLayer
    }
}

impl Default for LogHubLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> tracing_subscriber::Layer<S> for LogHubLayer
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let metadata = event.metadata();
        let level = *metadata.level();
        let target = metadata.target();
        let file = metadata.file().unwrap_or("unknown");
        let line = metadata.line().unwrap_or(0);

        let mut visitor = crate::tools::godot_logger::MessageVisitor::default();
        event.record(&mut visitor);

        let level_str = match level {
            tracing::Level::ERROR => "error",
            tracing::Level::WARN => "warn",
            tracing::Level::INFO => "info",
            tracing::Level::DEBUG => "debug",
            tracing::Level::TRACE => "trace",
        };
        // No-op unless a consumer subscribed to logs (connection-gated).
        crate::tools::scene_inspector::emit_log(
            "rust",
            Some(level_str),
            Some(target),
            Some(file),
            Some(line),
            visitor.message,
        );
    }
}

static LOGGER_REGISTERED: AtomicBool = AtomicBool::new(false);

fn register_godot_logger() {
    if LOGGER_REGISTERED.swap(true, Ordering::SeqCst) {
        return;
    }
    // Os holds a Ref to the logger, keeping it alive — no need to store it.
    let logger = DclLogHubLogger::new_gd();
    Os::singleton().add_logger(&logger.upcast::<Logger>());
}

/// Install the capture sinks (idempotent, cheap, no tokio). Called when a
/// consumer subscribes to logs (`SceneInspectorDispatcher::set_stream_logs`), so
/// nothing is installed until logs are actually wanted.
pub fn install_capture() {
    register_godot_logger();
    #[cfg(target_os = "ios")]
    fd_capture::install();
}

// ---------------------------------------------------------------------------
// iOS-only: stdout/stderr fd capture for native printf / Swift print / NSLog(stderr)
// ---------------------------------------------------------------------------

#[cfg(target_os = "ios")]
mod fd_capture {
    use std::sync::atomic::{AtomicBool, Ordering};

    /// Read chunk size for the capture pipes.
    const READ_BUF: usize = 4096;
    /// Force-flush a partial line if it grows past this without a newline.
    const MAX_LINE: usize = 64 * 1024;

    static INSTALLED: AtomicBool = AtomicBool::new(false);

    pub fn install() {
        if INSTALLED.swap(true, Ordering::SeqCst) {
            return;
        }
        // SAFETY: standard fd plumbing. Failures leave the original fds untouched.
        unsafe {
            if let Some((read_fd, saved_fd)) = redirect_fd(libc::STDOUT_FILENO) {
                spawn_reader(read_fd, saved_fd);
            }
            if let Some((read_fd, saved_fd)) = redirect_fd(libc::STDERR_FILENO) {
                spawn_reader(read_fd, saved_fd);
            }
            // Line-buffer C stdio so native logs arrive promptly. Apple symbols.
            extern "C" {
                static mut __stdoutp: *mut libc::FILE;
                static mut __stderrp: *mut libc::FILE;
            }
            libc::setvbuf(__stdoutp, std::ptr::null_mut(), libc::_IOLBF, 0);
            libc::setvbuf(__stderrp, std::ptr::null_mut(), libc::_IOLBF, 0);
        }
    }

    unsafe fn redirect_fd(target_fd: libc::c_int) -> Option<(libc::c_int, libc::c_int)> {
        let saved = libc::dup(target_fd);
        if saved < 0 {
            return None;
        }
        let mut fds = [0 as libc::c_int; 2];
        if libc::pipe(fds.as_mut_ptr()) != 0 {
            libc::close(saved);
            return None;
        }
        let (read_fd, write_fd) = (fds[0], fds[1]);
        if libc::dup2(write_fd, target_fd) < 0 {
            libc::close(saved);
            libc::close(read_fd);
            libc::close(write_fd);
            return None;
        }
        libc::close(write_fd);
        Some((read_fd, saved))
    }

    fn spawn_reader(read_fd: libc::c_int, saved_fd: libc::c_int) {
        std::thread::Builder::new()
            .name("dcl-log-capture".into())
            .spawn(move || {
                let mut buf = [0u8; READ_BUF];
                let mut acc: Vec<u8> = Vec::with_capacity(READ_BUF);
                loop {
                    let n = unsafe {
                        libc::read(read_fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len())
                    };
                    if n <= 0 {
                        break;
                    }
                    let chunk = &buf[..n as usize];
                    // Tee verbatim to the original console fd (keep --console intact).
                    unsafe {
                        let _ = libc::write(
                            saved_fd,
                            chunk.as_ptr() as *const libc::c_void,
                            n as usize,
                        );
                    }
                    acc.extend_from_slice(chunk);
                    flush_lines(&mut acc);
                }
            })
            .ok();
    }

    fn flush_lines(acc: &mut Vec<u8>) {
        loop {
            if let Some(pos) = acc.iter().position(|&b| b == b'\n') {
                let line: Vec<u8> = acc.drain(..=pos).collect();
                let mut end = line.len();
                while end > 0 && (line[end - 1] == b'\n' || line[end - 1] == b'\r') {
                    end -= 1;
                }
                let s = String::from_utf8_lossy(&line[..end]).into_owned();
                crate::tools::scene_inspector::emit_log("native", None, None, None, None, s);
            } else if acc.len() > MAX_LINE {
                let line: Vec<u8> = std::mem::take(acc);
                let s = String::from_utf8_lossy(&line).into_owned();
                crate::tools::scene_inspector::emit_log("native", None, None, None, None, s);
                break;
            } else {
                break;
            }
        }
    }
}
