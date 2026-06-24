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

/// Run the rendezvous hub, blocking on its own tokio runtime.
pub fn run_hub_blocking(bind: &str, device_port: u16, consumer_port: u16) -> Result<()> {
    print_hub_banner(device_port, consumer_port);
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    rt.block_on(serve_hub(bind.to_string(), device_port, consumer_port))
}

async fn serve_hub(bind: String, device_port: u16, consumer_port: u16) -> Result<()> {
    // device → consumers (frames) and consumers → device (commands).
    let to_consumers = broadcast::channel::<String>(HUB_FRAME_CAP).0;
    let to_device = broadcast::channel::<String>(HUB_CMD_CAP).0;

    let device_listener = TcpListener::bind(format!("{bind}:{device_port}")).await?;
    // Consumers are local tools only — never expose the command channel on the LAN.
    let consumer_listener = TcpListener::bind(format!("127.0.0.1:{consumer_port}")).await?;

    let dev = accept_devices(device_listener, to_consumers.clone(), to_device.clone());
    let cons = accept_consumers(consumer_listener, to_consumers, to_device);
    tokio::try_join!(dev, cons)?;
    Ok(())
}

async fn accept_devices(
    listener: TcpListener,
    to_consumers: broadcast::Sender<String>,
    to_device: broadcast::Sender<String>,
) -> Result<()> {
    loop {
        let (stream, peer) = listener.accept().await?;
        let (tc, td) = (to_consumers.clone(), to_device.clone());
        tokio::spawn(async move {
            if let Err(e) = handle_device(stream, peer, tc, td).await {
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
) -> Result<()> {
    let ws = tokio_tungstenite::accept_async(stream).await?;
    print_message(MessageType::Success, &format!("device connected ({peer})"));
    let (mut write, mut read) = ws.split();
    let mut cmd_rx = to_device.subscribe();
    loop {
        tokio::select! {
            msg = read.next() => match msg {
                Some(Ok(Message::Text(line))) => {
                    println!("{}", colorize(&line));
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
