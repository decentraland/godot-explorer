//! Unified log streaming for mobile debugging (primarily iOS).
//!
//! Problem: on iOS there is no convenient `adb logcat` equivalent. Logs live in
//! several channels — Rust (`tracing`), GDScript (`print`/`push_error`), the Godot
//! engine, and native Swift/ObjC (`NSLog`/`printf`). Crucially, on iOS Godot does
//! NOT route its own output (GDScript `print`, `godot_print!`) to the process
//! stdout/stderr — so a plain fd capture misses the most important logs.
//!
//! Capture strategy (two complementary sinks feeding one hub):
//!  1. A custom Godot `Logger` registered via `OS.add_logger`. Godot calls it for
//!     every `print` / `push_error` / engine message, so this captures GDScript,
//!     `godot_print!` (Rust routes through Godot's logger), engine logs and errors
//!     on every platform — independent of where stdout actually goes.
//!  2. On iOS only, an stdout/stderr fd redirect to catch native `printf` / Swift
//!     `print` / `fprintf(stderr)` (which bypass Godot's logger). Captured bytes
//!     are teed back to the real fds so `--console` keeps working.
//!
//! The app acts as a WebSocket **client** that dials out to a desktop collector
//! (`cargo run -- log-server`), mirroring the Scene Inspector connect-out model.
//! Activation is opt-in via `--log-stream=ws://host:port` or a baked deeplink,
//! off by default.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use futures_util::{SinkExt, StreamExt};
use godot::classes::{ILogger, Logger, Os};
use godot::prelude::*;
use tokio::sync::{broadcast, Notify};
use tokio_tungstenite::tungstenite::Message;

use crate::scene_runner::tokio_runtime::TokioRuntime;

/// Max lines replayed to a freshly connected client (recent history).
const RING_CAP: usize = 4096;
/// Broadcast backlog; under extreme volume the oldest live lines are dropped
/// (the connected client lags) rather than blocking the producers.
const BROADCAST_CAP: usize = 8192;

struct Hub {
    bcast: broadcast::Sender<String>,
    ring: Mutex<VecDeque<String>>,
}

fn hub() -> &'static Hub {
    static HUB: OnceLock<Hub> = OnceLock::new();
    HUB.get_or_init(|| Hub {
        bcast: broadcast::channel(BROADCAST_CAP).0,
        ring: Mutex::new(VecDeque::with_capacity(RING_CAP)),
    })
}

/// Push a captured log line into the hub (ring buffer + live broadcast).
/// MUST NOT log via Godot/tracing — it is called from inside the Godot logger and
/// would recurse.
fn push(line: String) {
    if line.is_empty() {
        return;
    }
    let hub = hub();
    {
        let mut ring = hub.ring.lock().unwrap();
        if ring.len() >= RING_CAP {
            ring.pop_front();
        }
        ring.push_back(line.clone());
    }
    let _ = hub.bcast.send(line);
}

// ---------------------------------------------------------------------------
// Godot logger sink: captures GDScript + godot_print! + engine + errors.
// ---------------------------------------------------------------------------

/// A Godot `Logger` that forwards every engine/GDScript/Rust message into the hub.
#[derive(GodotClass)]
#[class(init, base = Logger)]
struct DclLogHubLogger {
    base: Base<Logger>,
}

#[godot_api]
impl ILogger for DclLogHubLogger {
    fn log_message(&mut self, message: GString, _error: bool) {
        // A single message may contain several lines (and trailing newline).
        for line in message.to_string().lines() {
            // When the tracing LogHubLayer is active (iOS), Rust logs reach the
            // hub directly with a "[Rust:" prefix; skip the godot_print! copy that
            // also lands here via GodotTracingLayer to avoid duplicating them.
            if RUST_VIA_LAYER.load(Ordering::Relaxed) && line.starts_with("[Rust:") {
                continue;
            }
            push(line.to_string());
        }
    }

    // NOTE: `log_error` is intentionally NOT overridden. Its `Array<Gd<ScriptBacktrace>>`
    // parameter triggers a gdext class-id panic on iOS. Rust errors/warnings are
    // captured via the tracing `LogHubLayer` instead; GDScript `print` still flows
    // through `log_message`.
}

/// True once the tracing `LogHubLayer` is feeding Rust logs into the hub, so
/// `log_message` can drop the duplicate `godot_print!` copies.
static RUST_VIA_LAYER: AtomicBool = AtomicBool::new(false);

/// A `tracing` Layer that pushes every Rust log event straight into the hub —
/// independent of where Godot routes its console output (it doesn't reach stdout
/// on iOS), and covering all levels including WARN/ERROR. Add it to the subscriber
/// alongside the existing layers (see `dcl_global::*::init_logger`).
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

        let prefix = match level {
            tracing::Level::ERROR => "ERROR ",
            tracing::Level::WARN => "WARN ",
            _ => "",
        };
        push(format!(
            "{prefix}[Rust:{target}] {} ({file}:{line})",
            visitor.message
        ));
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

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Install all capture sinks (idempotent, cheap, no tokio). Safe to call early.
pub fn install_capture() {
    register_godot_logger();
    #[cfg(target_os = "ios")]
    fd_capture::install();
}

/// Start (or re-target) the outbound WebSocket client that streams captured logs
/// to `url` (e.g. `ws://192.168.1.5:9231`). Ensures capture is installed first.
/// Safe to call multiple times — the first call spawns the client; later calls
/// update the target and trigger a hot-reconnect.
pub fn start_client(url: String) {
    install_capture();

    let slot = target_url();
    {
        let mut guard = slot.lock().unwrap();
        *guard = url;
    }
    url_changed().notify_waiters();

    if CLIENT_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    TokioRuntime::spawn(async move {
        client_loop().await;
    });
}

// ---------------------------------------------------------------------------
// WebSocket client (connect-out + backoff + ring replay)
// ---------------------------------------------------------------------------

static CLIENT_STARTED: AtomicBool = AtomicBool::new(false);

fn target_url() -> &'static Mutex<String> {
    static T: OnceLock<Mutex<String>> = OnceLock::new();
    T.get_or_init(|| Mutex::new(String::new()))
}

fn url_changed() -> &'static Notify {
    static N: OnceLock<Notify> = OnceLock::new();
    N.get_or_init(Notify::new)
}

fn current_url() -> String {
    target_url().lock().unwrap().clone()
}

async fn client_loop() {
    use std::time::Duration;
    let mut backoff = 1.0f64;
    loop {
        let url = current_url();
        if url.is_empty() {
            url_changed().notified().await;
            continue;
        }

        match tokio_tungstenite::connect_async(&url).await {
            Ok((ws, _)) => {
                backoff = 1.0;
                tracing::info!("[log-stream] connected to {}", url);
                serve_connection(ws).await;
                tracing::debug!("[log-stream] disconnected from {}", url);
            }
            Err(e) => {
                tracing::debug!("[log-stream] connect to {} failed: {}", url, e);
            }
        }

        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs_f64(backoff)) => {}
            _ = url_changed().notified() => { backoff = 1.0; continue; }
        }
        backoff = (backoff * 2.0).min(30.0);
    }
}

async fn serve_connection<S>(ws: tokio_tungstenite::WebSocketStream<S>)
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let hub = hub();
    // Subscribe BEFORE replaying history so no live line slips through the gap.
    let mut rx = hub.bcast.subscribe();
    let (mut write, mut read) = ws.split();

    let backlog: Vec<String> = {
        let ring = hub.ring.lock().unwrap();
        ring.iter().cloned().collect()
    };
    for line in backlog {
        if write.send(Message::Text(line)).await.is_err() {
            return;
        }
    }

    loop {
        tokio::select! {
            msg = rx.recv() => match msg {
                Ok(line) => {
                    if write.send(Message::Text(line)).await.is_err() {
                        return;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return,
            },
            incoming = read.next() => match incoming {
                Some(Ok(Message::Close(_))) | None => return,
                Some(Err(_)) => return,
                Some(Ok(_)) => {}
            },
            _ = url_changed().notified() => return,
        }
    }
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
                super::push(String::from_utf8_lossy(&line[..end]).into_owned());
            } else if acc.len() > MAX_LINE {
                let line: Vec<u8> = std::mem::take(acc);
                super::push(String::from_utf8_lossy(&line).into_owned());
                break;
            } else {
                break;
            }
        }
    }
}
