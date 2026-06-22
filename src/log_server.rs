//! Desktop-side collector for the `--log-stream` debug feature.
//!
//! The mobile app (see `lib/src/tools/log_stream.rs`) dials out as a WebSocket
//! client and streams its captured stdout/stderr here. This server accepts those
//! connections and prints every received line to stdout, so it can be piped
//! (`cargo run -- log-server | grep -i error`). Mirrors the connect-out model the
//! Scene Inspector already uses, which avoids iOS local-network listener friction.

use std::net::{IpAddr, SocketAddr};

use anyhow::Result;
use colored::Colorize;
use futures_util::StreamExt;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;

use crate::ui::{print_message, MessageType};

/// Best-effort primary LAN IPv4 of this machine. Uses the classic "connect a UDP
/// socket to a public address and read back the local address" trick — no packets
/// are actually sent, it just resolves which interface would be used.
pub fn lan_ip() -> Option<IpAddr> {
    let sock = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.connect("8.8.8.8:80").ok()?;
    sock.local_addr().ok().map(|a| a.ip())
}

/// Build the `ws://host:port` the app should dial back to. `prefer_lan` resolves
/// the machine's LAN IP (for on-device builds); otherwise loopback for desktop.
pub fn ws_url(port: u16, prefer_lan: bool) -> String {
    let host = if prefer_lan {
        lan_ip()
            .map(|ip| ip.to_string())
            .unwrap_or_else(|| "127.0.0.1".to_string())
    } else {
        "127.0.0.1".to_string()
    };
    format!("ws://{host}:{port}")
}

/// Print the ready banner: the LAN IP and a paste-ready `--log-stream=...` arg.
pub fn print_banner(port: u16) {
    let host = lan_ip()
        .map(|ip| ip.to_string())
        .unwrap_or_else(|| "<this-machine-ip>".to_string());
    print_message(
        MessageType::Info,
        &format!(
            "Log collector listening on 0.0.0.0:{port}\n\
             From a mobile build, launch with:\n    \
             --log-stream=ws://{host}:{port}\n\
             Or via deeplink:  decentraland://open?log-stream=ws://{host}:{port}"
        ),
    );
}

/// Run the collector, blocking on its own tokio runtime. Used by the standalone
/// `log-server` subcommand.
pub fn run_blocking(bind: &str, port: u16) -> Result<()> {
    print_banner(port);
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    rt.block_on(serve(bind.to_string(), port))
}

/// Start the collector on a background thread with its own runtime. Used when
/// embedded into `cargo run -- run --log-stream`, so it lives alongside the
/// device console stream in the same terminal.
pub fn spawn_background(bind: String, port: u16) {
    std::thread::Builder::new()
        .name("dcl-log-server".into())
        .spawn(move || {
            let rt = match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    print_message(MessageType::Error, &format!("log-server runtime: {e}"));
                    return;
                }
            };
            if let Err(e) = rt.block_on(serve(bind, port)) {
                print_message(MessageType::Error, &format!("log-server: {e}"));
            }
        })
        .ok();
}

async fn serve(bind: String, port: u16) -> Result<()> {
    let addr: SocketAddr = format!("{bind}:{port}").parse()?;
    let listener = TcpListener::bind(addr).await?;
    loop {
        let (stream, peer) = listener.accept().await?;
        tokio::spawn(async move {
            if let Err(e) = handle_client(stream, peer).await {
                print_message(
                    MessageType::Warning,
                    &format!("log-server client {peer} ended: {e}"),
                );
            }
        });
    }
}

async fn handle_client(stream: TcpStream, peer: SocketAddr) -> Result<()> {
    let ws = tokio_tungstenite::accept_async(stream).await?;
    print_message(MessageType::Success, &format!("device connected ({peer})"));
    let (_write, mut read) = ws.split();
    while let Some(msg) = read.next().await {
        match msg? {
            Message::Text(line) => println!("{}", colorize(&line)),
            Message::Close(_) => break,
            _ => {}
        }
    }
    print_message(MessageType::Step, &format!("device disconnected ({peer})"));
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
