//! Client for the social service real-time stream (`/v1/stream`): a WebSocket with JSON text
//! frames. One connection carries every subscription. See
//! `docs/websocket-stream-protocol.md` in social-service-ea for the protocol.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::{HeaderName, HeaderValue};
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;

use crate::auth::ephemeral_auth_chain::EphemeralAuthChain;
use crate::auth::wallet::sign_request;

const CONNECTION_TIMEOUT_SECS: u64 = 10;
const SUBSCRIBE_TIMEOUT_SECS: u64 = 10;
/// A re-subscribe right after an unsubscribe can race the server tearing the old one down.
const SUBSCRIBE_ATTEMPTS: usize = 5;
const SUBSCRIBE_RETRY_DELAY_MS: u64 = 300;
const PING_INTERVAL_SECS: u64 = 30;
/// Nothing received for this long (not even the pong of our ping) means the socket is dead.
const STALE_CONNECTION_SECS: u64 = 75;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ConnectionState {
    Disconnected = 0,
    Connecting = 1,
    Connected = 2,
    Reconnecting = 3,
}

impl ConnectionState {
    fn from_u8(value: u8) -> Self {
        match value {
            1 => ConnectionState::Connecting,
            2 => ConnectionState::Connected,
            3 => ConnectionState::Reconnecting,
            _ => ConnectionState::Disconnected,
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum ServerMessage {
    Ready,
    Subscribed {
        id: String,
    },
    Event {
        id: String,
        stream: String,
        data: Value,
    },
    End {
        id: String,
    },
    Error {
        #[serde(default)]
        id: Option<String>,
        code: String,
        message: String,
    },
    Pong,
}

struct SubscribeError {
    code: String,
    message: String,
}

struct PendingSubscription {
    stream: String,
    sender: mpsc::UnboundedSender<Value>,
    done: oneshot::Sender<Result<(), SubscribeError>>,
}

struct ActiveSubscription {
    id: String,
    sender: mpsc::UnboundedSender<Value>,
}

#[derive(Default)]
struct Subscriptions {
    /// Keyed by stream name: the server allows one subscription per stream and connection.
    active: HashMap<String, ActiveSubscription>,
    /// Keyed by subscription id, until the server confirms or rejects it.
    pending: HashMap<String, PendingSubscription>,
}

struct Connection {
    outgoing: mpsc::UnboundedSender<Message>,
    subscriptions: Arc<Mutex<Subscriptions>>,
    closed: CancellationToken,
    next_id: AtomicU64,
}

impl Connection {
    fn send_json(&self, value: Value) -> Result<()> {
        self.outgoing
            .send(Message::Text(value.to_string()))
            .map_err(|_| anyhow!("Social stream connection is closed"))
    }
}

pub struct SocialStream {
    url: String,
    wallet: Arc<EphemeralAuthChain>,
    connection: tokio::sync::Mutex<Option<Arc<Connection>>>,
    state: Arc<AtomicU8>,
}

impl SocialStream {
    pub fn new(url: String, wallet: Arc<EphemeralAuthChain>) -> Self {
        Self {
            url,
            wallet,
            connection: tokio::sync::Mutex::new(None),
            state: Arc::new(AtomicU8::new(ConnectionState::Disconnected as u8)),
        }
    }

    pub fn connection_state(&self) -> ConnectionState {
        ConnectionState::from_u8(self.state.load(Ordering::Relaxed))
    }

    /// Subscribes to `stream` and returns its events. The receiver ends when the subscription
    /// ends or the connection drops; subscribing again then reconnects.
    ///
    /// Subscribing to a stream that is already live on this connection hands its events to the
    /// new receiver instead of creating a second server-side subscription.
    pub async fn subscribe(&self, stream: &str) -> Result<mpsc::UnboundedReceiver<Value>> {
        let mut last_error = anyhow!("Could not subscribe to {stream}");

        for _ in 0..SUBSCRIBE_ATTEMPTS {
            let connection = self.connection().await?;

            let (sender, receiver) = mpsc::unbounded_channel();
            let (done, done_receiver) = oneshot::channel();
            let id = format!(
                "{stream}-{}",
                connection.next_id.fetch_add(1, Ordering::Relaxed)
            );

            {
                let mut subscriptions = connection.subscriptions.lock().unwrap();
                if let Some(active) = subscriptions.active.get_mut(stream) {
                    active.sender = sender;
                    return Ok(receiver);
                }
                subscriptions.pending.insert(
                    id.clone(),
                    PendingSubscription {
                        stream: stream.to_string(),
                        sender,
                        done,
                    },
                );
            }

            connection.send_json(json!({ "type": "subscribe", "id": id, "stream": stream }))?;

            match tokio::time::timeout(Duration::from_secs(SUBSCRIBE_TIMEOUT_SECS), done_receiver)
                .await
            {
                Ok(Ok(Ok(()))) => return Ok(receiver),
                Ok(Ok(Err(error))) if error.code == "duplicate_subscription" => {
                    last_error = anyhow!("{stream}: {}", error.message);
                    tokio::time::sleep(Duration::from_millis(SUBSCRIBE_RETRY_DELAY_MS)).await;
                }
                Ok(Ok(Err(error))) => {
                    return Err(anyhow!(
                        "Subscribing to {stream} failed ({}): {}",
                        error.code,
                        error.message
                    ))
                }
                Ok(Err(_)) => {
                    last_error = anyhow!("The connection closed while subscribing to {stream}");
                }
                Err(_) => {
                    connection.subscriptions.lock().unwrap().pending.remove(&id);
                    return Err(anyhow!(
                        "Timeout subscribing to {stream} after {SUBSCRIBE_TIMEOUT_SECS}s"
                    ));
                }
            }
        }

        Err(last_error)
    }

    /// Closes the connection, ending every subscription.
    pub fn close(&self) {
        if let Ok(guard) = self.connection.try_lock() {
            if let Some(connection) = guard.as_ref() {
                connection.closed.cancel();
            }
        }
    }

    async fn connection(&self) -> Result<Arc<Connection>> {
        let mut guard = self.connection.lock().await;
        if let Some(connection) = guard.as_ref() {
            if !connection.closed.is_cancelled() {
                return Ok(connection.clone());
            }
        }

        let previous_state = if guard.is_some() {
            ConnectionState::Reconnecting
        } else {
            ConnectionState::Connecting
        };
        self.state.store(previous_state as u8, Ordering::Relaxed);

        match connect(&self.url, &self.wallet, self.state.clone()).await {
            Ok(connection) => {
                self.state
                    .store(ConnectionState::Connected as u8, Ordering::Relaxed);
                *guard = Some(connection.clone());
                Ok(connection)
            }
            Err(error) => {
                self.state
                    .store(ConnectionState::Disconnected as u8, Ordering::Relaxed);
                tracing::warn!("Social stream connection failed: {error}");
                Err(error)
            }
        }
    }
}

impl Drop for SocialStream {
    fn drop(&mut self) {
        self.close();
    }
}

async fn connect(
    url: &str,
    wallet: &EphemeralAuthChain,
    state: Arc<AtomicU8>,
) -> Result<Arc<Connection>> {
    let timeout = Duration::from_secs(CONNECTION_TIMEOUT_SECS);

    // The upgrade request carries the Signed Fetch headers for `get:/v1/stream`, so the server
    // verifies them before accepting the socket (401 when rejected) and answers `ready` at once.
    let mut request = url
        .into_client_request()
        .map_err(|e| anyhow!("Invalid social stream URL {url}: {e}"))?;
    for (key, value) in sign_request("get", request.uri(), wallet, json!({})).await {
        request.headers_mut().insert(
            HeaderName::from_bytes(key.as_bytes())?,
            HeaderValue::from_str(&value)?,
        );
    }

    let (mut socket, _) = tokio::time::timeout(timeout, tokio_tungstenite::connect_async(request))
        .await
        .map_err(|_| {
            anyhow!("Timeout connecting to the social stream after {CONNECTION_TIMEOUT_SECS}s")
        })?
        .map_err(|e| anyhow!("Failed to connect to the social stream: {e}"))?;

    tokio::time::timeout(timeout, async {
        while let Some(message) = socket.next().await {
            match message.map_err(|e| anyhow!("Social stream error during auth: {e}"))? {
                Message::Text(text) => {
                    if let Ok(ServerMessage::Ready) = serde_json::from_str(&text) {
                        return Ok(());
                    }
                }
                Message::Close(frame) => {
                    return Err(anyhow!(
                        "The social stream rejected the connection: {}",
                        frame
                            .map(|f| format!("{} {}", f.code, f.reason))
                            .unwrap_or_default()
                    ))
                }
                _ => {}
            }
        }
        Err(anyhow!("The social stream closed during auth"))
    })
    .await
    .map_err(|_| anyhow!("Timeout authenticating on the social stream"))??;

    let (outgoing, outgoing_receiver) = mpsc::unbounded_channel();
    let connection = Arc::new(Connection {
        outgoing,
        subscriptions: Arc::new(Mutex::new(Subscriptions::default())),
        closed: CancellationToken::new(),
        next_id: AtomicU64::new(1),
    });

    tokio::spawn(run_connection(
        socket,
        outgoing_receiver,
        connection.subscriptions.clone(),
        connection.closed.clone(),
        connection.outgoing.clone(),
        state,
    ));

    Ok(connection)
}

type Socket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

async fn run_connection(
    socket: Socket,
    mut outgoing_receiver: mpsc::UnboundedReceiver<Message>,
    subscriptions: Arc<Mutex<Subscriptions>>,
    closed: CancellationToken,
    outgoing: mpsc::UnboundedSender<Message>,
    state: Arc<AtomicU8>,
) {
    let (mut writer, mut reader) = socket.split();
    let mut ping_interval = tokio::time::interval(Duration::from_secs(PING_INTERVAL_SECS));
    ping_interval.tick().await;
    let mut last_received = Instant::now();

    loop {
        tokio::select! {
            _ = closed.cancelled() => break,
            message = outgoing_receiver.recv() => {
                let Some(message) = message else { break };
                if let Err(e) = writer.send(message).await {
                    tracing::warn!("Social stream write failed: {e}");
                    break;
                }
            }
            _ = ping_interval.tick() => {
                if last_received.elapsed() > Duration::from_secs(STALE_CONNECTION_SECS) {
                    tracing::warn!("Social stream went silent, closing it");
                    break;
                }
                if writer.send(Message::Text(json!({ "type": "ping" }).to_string())).await.is_err() {
                    break;
                }
            }
            message = reader.next() => {
                match message {
                    Some(Ok(Message::Text(text))) => {
                        last_received = Instant::now();
                        handle_server_message(&text, &subscriptions, &outgoing);
                    }
                    Some(Ok(Message::Close(frame))) => {
                        tracing::info!("Social stream closed by the server: {:?}", frame);
                        break;
                    }
                    Some(Ok(_)) => last_received = Instant::now(),
                    Some(Err(e)) => {
                        tracing::warn!("Social stream read failed: {e}");
                        break;
                    }
                    None => break,
                }
            }
        }
    }

    // In this order so a reconnect, which only starts once `closed` is cancelled, is never
    // reported as disconnected by the connection it replaces.
    state.store(ConnectionState::Disconnected as u8, Ordering::Relaxed);
    closed.cancel();
    {
        // Dropping the senders ends every receiver, which is how consumers learn the
        // subscription dropped. Dropping `done` fails the pending subscribes.
        let mut subscriptions = subscriptions.lock().unwrap();
        subscriptions.active.clear();
        subscriptions.pending.clear();
    }
    let _ = writer.close().await;
}

fn handle_server_message(
    text: &str,
    subscriptions: &Mutex<Subscriptions>,
    outgoing: &mpsc::UnboundedSender<Message>,
) {
    let message: ServerMessage = match serde_json::from_str(text) {
        Ok(message) => message,
        Err(e) => {
            tracing::warn!("Unexpected social stream message ({e}): {text}");
            return;
        }
    };

    let mut subscriptions = subscriptions.lock().unwrap();
    match message {
        ServerMessage::Subscribed { id } => {
            if let Some(pending) = subscriptions.pending.remove(&id) {
                let _ = pending.done.send(Ok(()));
                subscriptions.active.insert(
                    pending.stream,
                    ActiveSubscription {
                        id,
                        sender: pending.sender,
                    },
                );
            }
        }
        ServerMessage::Event { id, stream, data } => {
            let Some(active) = subscriptions.active.get(&stream) else {
                return;
            };
            if active.id != id {
                return;
            }
            if active.sender.send(data).is_err() {
                // Nobody listens anymore: stop the server-side subscription too.
                subscriptions.active.remove(&stream);
                let _ = outgoing.send(Message::Text(
                    json!({ "type": "unsubscribe", "id": id }).to_string(),
                ));
            }
        }
        ServerMessage::End { id } => {
            subscriptions.active.retain(|_, active| active.id != id);
        }
        ServerMessage::Error { id, code, message } => {
            let Some(id) = id else {
                tracing::warn!("Social stream error {code}: {message}");
                return;
            };
            if let Some(pending) = subscriptions.pending.remove(&id) {
                let _ = pending.done.send(Err(SubscribeError { code, message }));
            } else {
                tracing::warn!("Social stream subscription {id} failed ({code}): {message}");
                subscriptions.active.retain(|_, active| active.id != id);
            }
        }
        ServerMessage::Ready | ServerMessage::Pong => {}
    }
}
