//! `cargo run -- debug-hub` — the desktop rendezvous broker for the unified
//! scene-inspector debug channel.
//!
//! A device dials OUT to the hub (it can't be dialed into, esp. on iOS); local
//! tools (AI / MCP / websocat / the external inspector app) connect to the
//! loopback consumer port. The hub fans the device's frames to every consumer
//! and relays each consumer's commands back to the device.

use std::net::{IpAddr, SocketAddr};

use anyhow::Result;
use colored::Colorize;
use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::Message;

use crate::ui::{print_message, MessageType};

/// Broadcast backlog for device→consumer frames. Large because entries (CRDT /
/// logs) can burst; a lagging consumer drops oldest rather than stalling others.
const HUB_FRAME_CAP: usize = 8192;
/// Backlog for consumer→device commands. Commands are infrequent.
const HUB_CMD_CAP: usize = 256;

/// What the hub prints for the device it serves (it always fans every frame out
/// to all consumers regardless — this only affects the hub's own stdout). The
/// `run --target ios` log viewer is NOT here: it's a separate *consumer* (see
/// `spawn_log_viewer`), so it works whether this process owns the hub or reuses
/// an existing one, and coexists with other consumers (inspector app, websocat).
#[derive(Clone, Copy, PartialEq)]
pub enum HubOutput {
    /// Relay only, print nothing. The auto-hub started by `run --target …` (the
    /// viewer/consumer and `adb logcat` handle on-screen logs).
    Quiet,
    /// Echo every device frame verbatim. The standalone `debug-hub` viewer.
    RawFrames,
}

/// Best-effort primary LAN IPv4 of this machine. Uses the classic "connect a UDP
/// socket to a public address and read back the local address" trick — no packets
/// are actually sent, it just resolves which interface would be used.
pub fn lan_ip() -> Option<IpAddr> {
    let sock = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.connect("8.8.8.8:80").ok()?;
    sock.local_addr().ok().map(|a| a.ip())
}

// ---------------------------------------------------------------------------
// debug-hub: rendezvous broker for the unified scene-inspector channel.
//
// One device dials out (it can't be dialed into, esp. on iOS) to the
// device-facing listener; many consumers (AI / MCP / websocat / the external
// inspector app) connect to the loopback consumer-facing listener. The hub fans
// the device's outbound frames to every consumer and relays each consumer's
// commands back to the device — so a single connect-out device is reachable by
// any number of local tools, on any platform.
// ---------------------------------------------------------------------------

/// Print the hub banner: how the device and the consumers each connect.
pub fn print_hub_banner(device_port: u16, consumer_port: u16) {
    let host = lan_ip()
        .map(|ip| ip.to_string())
        .unwrap_or_else(|| "<this-machine-ip>".to_string());
    print_message(
        MessageType::Info,
        &format!(
            "Debug hub ready.\n\
             Device dials out to:   ws://{host}:{device_port}\n    \
             launch with:  --scene-inspector=ws://{host}:{device_port}\n    \
             (or deeplink:  decentraland://open?scene-inspector=ws://{host}:{device_port})\n\
             Consumers connect to:  ws://127.0.0.1:{consumer_port}\n    \
             (websocat / MCP / inspector app — fan-out + command relay)"
        ),
    );
}

/// Run the rendezvous hub, blocking on its own tokio runtime. The standalone
/// `debug-hub` subcommand is also the full viewer, so it echoes device frames to
/// stdout verbatim (`HubOutput::RawFrames`).
pub fn run_hub_blocking(bind: &str, device_port: u16, consumer_port: u16) -> Result<()> {
    print_hub_banner(device_port, consumer_port);
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    rt.block_on(serve_hub(
        bind.to_string(),
        device_port,
        consumer_port,
        HubOutput::RawFrames,
    ))
}

/// Start the hub on a background thread (its own tokio runtime) and return
/// immediately, so a single `run --target ios/android` can do build + deploy +
/// hub. Non-fatal: if the consumer port is already bound we assume a hub is
/// already up and reuse it rather than failing the run. `output` controls only
/// the hub's own stdout — frames always fan out to consumers (websocat / MCP /
/// inspector app) regardless.
pub fn spawn_hub_background(bind: &str, device_port: u16, consumer_port: u16, output: HubOutput) {
    // Probe the loopback consumer port: a running hub holds it. This is a
    // best-effort check (a tiny TOCTOU window remains; a lost race just logs a
    // warning from the thread below and is otherwise harmless).
    if std::net::TcpListener::bind(("127.0.0.1", consumer_port)).is_err() {
        print_message(
            MessageType::Info,
            &format!("Debug hub already running (consumer port {consumer_port}) — reusing it."),
        );
        return;
    }
    print_hub_banner(device_port, consumer_port);
    let bind = bind.to_string();
    let spawned = std::thread::Builder::new()
        .name("debug-hub".into())
        .spawn(move || {
            let rt = match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    print_message(
                        MessageType::Warning,
                        &format!("debug hub: runtime init failed: {e}"),
                    );
                    return;
                }
            };
            if let Err(e) = rt.block_on(serve_hub(bind, device_port, consumer_port, output)) {
                print_message(MessageType::Warning, &format!("debug hub stopped: {e}"));
            }
        });
    if let Err(e) = spawned {
        print_message(
            MessageType::Warning,
            &format!("could not start debug hub thread: {e}"),
        );
    }
}

/// Spawn a background **log viewer**: connect to the hub's loopback consumer port
/// as one consumer among many, subscribe to the `log` stream, and pretty-print
/// log entries to stdout (skipping `native`, already shown verbatim in the device
/// `--console`). Used by `run --target ios` so the dev sees the GDScript/Rust logs
/// os_log hides — in the same terminal, whether this process started the hub or
/// reused one. Retries/reconnects forever (daemon thread; dies with the process).
pub fn spawn_log_viewer(consumer_port: u16) {
    std::thread::Builder::new()
        .name("hub-log-viewer".into())
        .spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(_) => return,
            };
            rt.block_on(log_viewer_loop(consumer_port));
        })
        .ok();
}

async fn log_viewer_loop(consumer_port: u16) {
    let url = format!("ws://127.0.0.1:{consumer_port}");
    let mut announced = false;
    loop {
        if let Ok((ws, _)) = tokio_tungstenite::connect_async(&url).await {
            let (mut write, mut read) = ws.split();
            // Subscribe as a consumer (relayed to the device → flips its log
            // stream on and flushes the boot ring). Opt-in stays intact: nothing
            // streams until this subscribe lands.
            let sub = concat!(
                r#"{"type":"SCENE_INSPECTOR_CMD","cmd":"subscribe","#,
                r#""args":{"streams":["log"]},"id":"hub-log-viewer"}"#
            );
            // Initial subscribe covers a device that's ALREADY connected when we
            // attach (e.g. reusing a hub). For a device that connects LATER, our
            // subscribe was lost (no device to relay to), so we re-subscribe when
            // its `session_start` arrives — idempotent, and re-flushes its boot
            // ring. Together these cover every connect ordering.
            if write.send(Message::Text(sub.to_string())).await.is_ok() {
                if !announced {
                    announced = true;
                    print_message(
                        MessageType::Info,
                        "Log viewer attached (GDScript/Rust logs below; native via --console).",
                    );
                }
                while let Some(Ok(msg)) = read.next().await {
                    if let Message::Text(line) = msg {
                        if line.contains("\"type\":\"session_start\"") {
                            let _ = write.send(Message::Text(sub.to_string())).await;
                        }
                        print_log_entries(&line);
                    }
                }
            }
        }
        // Hub not up yet, or the connection dropped — back off and retry.
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
    }
}

async fn serve_hub(
    bind: String,
    device_port: u16,
    consumer_port: u16,
    output: HubOutput,
) -> Result<()> {
    // device → consumers (frames) and consumers → device (commands).
    let to_consumers = broadcast::channel::<String>(HUB_FRAME_CAP).0;
    let to_device = broadcast::channel::<String>(HUB_CMD_CAP).0;

    let device_listener = TcpListener::bind(format!("{bind}:{device_port}")).await?;
    // Consumers are local tools only — never expose the command channel on the LAN.
    let consumer_listener = TcpListener::bind(format!("127.0.0.1:{consumer_port}")).await?;

    let dev = accept_devices(
        device_listener,
        to_consumers.clone(),
        to_device.clone(),
        output,
    );
    let cons = accept_consumers(consumer_listener, to_consumers, to_device);
    tokio::try_join!(dev, cons)?;
    Ok(())
}

async fn accept_devices(
    listener: TcpListener,
    to_consumers: broadcast::Sender<String>,
    to_device: broadcast::Sender<String>,
    output: HubOutput,
) -> Result<()> {
    loop {
        let (stream, peer) = listener.accept().await?;
        let (tc, td) = (to_consumers.clone(), to_device.clone());
        tokio::spawn(async move {
            if let Err(e) = handle_device(stream, peer, tc, td, output).await {
                print_message(
                    MessageType::Warning,
                    &format!("hub device {peer} ended: {e}"),
                );
            }
        });
    }
}

async fn accept_consumers(
    listener: TcpListener,
    to_consumers: broadcast::Sender<String>,
    to_device: broadcast::Sender<String>,
) -> Result<()> {
    loop {
        let (stream, peer) = listener.accept().await?;
        let (tc, td) = (to_consumers.clone(), to_device.clone());
        tokio::spawn(async move {
            if let Err(e) = handle_consumer(stream, peer, tc, td).await {
                print_message(
                    MessageType::Warning,
                    &format!("hub consumer {peer} ended: {e}"),
                );
            }
        });
    }
}

/// The connect-out device: its frames are printed and fanned to consumers; it
/// receives relayed consumer commands on the same socket.
async fn handle_device(
    stream: TcpStream,
    peer: SocketAddr,
    to_consumers: broadcast::Sender<String>,
    to_device: broadcast::Sender<String>,
    output: HubOutput,
) -> Result<()> {
    let ws = tokio_tungstenite::accept_async(stream).await?;
    print_message(MessageType::Success, &format!("device connected ({peer})"));
    let (mut write, mut read) = ws.split();
    let mut cmd_rx = to_device.subscribe();
    loop {
        tokio::select! {
            msg = read.next() => match msg {
                Some(Ok(Message::Text(line))) => {
                    if output == HubOutput::RawFrames {
                        println!("{}", colorize(&line));
                    }
                    let _ = to_consumers.send(line);
                }
                Some(Ok(Message::Close(_))) | None => break,
                Some(Ok(_)) => {}
                Some(Err(e)) => return Err(e.into()),
            },
            cmd = cmd_rx.recv() => match cmd {
                Ok(c) => {
                    if write.send(Message::Text(c)).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {}
                Err(broadcast::error::RecvError::Closed) => break,
            },
        }
    }
    print_message(MessageType::Step, &format!("device disconnected ({peer})"));
    Ok(())
}

/// A local consumer: receives every device frame; its commands are relayed to
/// the device.
async fn handle_consumer(
    stream: TcpStream,
    peer: SocketAddr,
    to_consumers: broadcast::Sender<String>,
    to_device: broadcast::Sender<String>,
) -> Result<()> {
    let ws = tokio_tungstenite::accept_async(stream).await?;
    print_message(
        MessageType::Success,
        &format!("consumer connected ({peer})"),
    );
    let (mut write, mut read) = ws.split();
    let mut frame_rx = to_consumers.subscribe();
    loop {
        tokio::select! {
            msg = read.next() => match msg {
                Some(Ok(Message::Text(cmd))) => {
                    let _ = to_device.send(cmd);
                }
                Some(Ok(Message::Close(_))) | None => break,
                Some(Ok(_)) => {}
                Some(Err(e)) => return Err(e.into()),
            },
            frame = frame_rx.recv() => match frame {
                Ok(f) => {
                    if write.send(Message::Text(f)).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {}
                Err(broadcast::error::RecvError::Closed) => break,
            },
        }
    }
    print_message(
        MessageType::Step,
        &format!("consumer disconnected ({peer})"),
    );
    Ok(())
}

/// Light severity coloring. `colored` is tty-aware (and honors NO_COLOR), so this
/// stays pipe-friendly: when stdout is redirected the lines are emitted plain.
fn colorize(line: &str) -> String {
    if line.contains("ERROR") || line.contains("SCRIPT ERROR") {
        line.red().to_string()
    } else if line.contains("WARN") {
        line.yellow().to_string()
    } else {
        line.to_string()
    }
}

/// LogViewer: pull the `log` entries out of a SCENE_INSPECTOR frame and print
/// them readably (`[source/level] msg`), skipping `native` (already shown
/// verbatim in the device `--console`) and the CRDT/perf/lifecycle firehose.
fn print_log_entries(line: &str) {
    // Cheap pre-filter: most frames (pure CRDT/perf) carry no log entry, so skip
    // the JSON parse entirely unless a log entry is present.
    if !line.contains("\"type\":\"log\"") {
        return;
    }
    let frame: serde_json::Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(_) => return,
    };
    let entries = match frame
        .get("payload")
        .and_then(|p| p.get("entries"))
        .and_then(|e| e.as_array())
    {
        Some(e) => e,
        None => return,
    };
    for entry in entries {
        if entry.get("type").and_then(|t| t.as_str()) != Some("log") {
            continue;
        }
        let source = entry.get("source").and_then(|s| s.as_str()).unwrap_or("?");
        if source == "native" {
            continue;
        }
        let level = entry.get("level").and_then(|l| l.as_str());
        let msg = entry.get("msg").and_then(|m| m.as_str()).unwrap_or("");
        // `target` is the Rust module path (e.g. `dclgodot::comms`) for `rust`
        // entries — show it so the lib/sublib is visible. GDScript logs have none.
        let target = entry.get("target").and_then(|t| t.as_str());
        let tag = match (level, target) {
            (Some(l), Some(t)) => format!("[{source}/{l} {t}]"),
            (Some(l), None) => format!("[{source}/{l}]"),
            (None, Some(t)) => format!("[{source} {t}]"),
            (None, None) => format!("[{source}]"),
        };
        let formatted = format!("{tag} {msg}");
        let colored = match level {
            Some(l) if l.eq_ignore_ascii_case("error") => formatted.red().to_string(),
            Some(l) if l.eq_ignore_ascii_case("warn") => formatted.yellow().to_string(),
            _ => formatted,
        };
        println!("{colored}");
    }
}
