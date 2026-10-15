use std::sync::{
    atomic::{AtomicU8, Ordering},
    Arc,
};

use dcl_core::Core;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::{client::IntoClientRequest, http::HeaderValue, Message};

const CLOSED: u8 = 0;
const CONNECTING: u8 = 1;
const OPEN: u8 = 2;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WsState {
    Closed,
    Connecting,
    Open,
}

pub enum Frame {
    Binary(Vec<u8>),
    Text(String),
}

enum Outgoing {
    Frame(Message),
    Close,
}

/// A polled websocket: a tokio task owns the socket, the owner reads `state()` and
/// drains `next_packet()` from its own thread.
pub struct WsClient {
    state: Arc<AtomicU8>,
    incoming: Option<mpsc::UnboundedReceiver<Frame>>,
    outgoing: Option<mpsc::UnboundedSender<Outgoing>>,
}

impl WsClient {
    pub fn new() -> Self {
        Self {
            state: Arc::new(AtomicU8::new(CLOSED)),
            incoming: None,
            outgoing: None,
        }
    }

    pub fn state(&self) -> WsState {
        match self.state.load(Ordering::Acquire) {
            OPEN => WsState::Open,
            CONNECTING => WsState::Connecting,
            _ => WsState::Closed,
        }
    }

    pub fn connect(&mut self, url: &str, protocol: &str) {
        self.close();
        let (in_tx, in_rx) = mpsc::unbounded_channel();
        let (out_tx, out_rx) = mpsc::unbounded_channel();
        self.incoming = Some(in_rx);
        self.outgoing = Some(out_tx);
        // A fresh state cell per connection, so a stale task cannot overwrite the new one.
        self.state = Arc::new(AtomicU8::new(CONNECTING));
        let task = run(
            url.to_string(),
            protocol.to_string(),
            self.state.clone(),
            in_tx,
            out_rx,
        );
        // An ambient runtime (unit tests) wins; the engine main thread has none.
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(task);
        } else {
            Core::spawn(task);
        }
    }

    /// Queues a binary frame; false when the socket is not open.
    pub fn send(&self, data: Vec<u8>) -> bool {
        self.queue(Message::Binary(data))
    }

    /// Queues a text frame; false when the socket is not open.
    pub fn send_text(&self, text: String) -> bool {
        self.queue(Message::Text(text))
    }

    fn queue(&self, message: Message) -> bool {
        self.state() == WsState::Open
            && self
                .outgoing
                .as_ref()
                .is_some_and(|tx| tx.send(Outgoing::Frame(message)).is_ok())
    }

    pub fn next_frame(&mut self) -> Option<Frame> {
        self.incoming.as_mut()?.try_recv().ok()
    }

    /// The next binary frame; text frames are dropped.
    pub fn next_packet(&mut self) -> Option<Vec<u8>> {
        loop {
            match self.next_frame()? {
                Frame::Binary(data) => return Some(data),
                Frame::Text(_) => {}
            }
        }
    }

    pub fn close(&mut self) {
        if let Some(tx) = self.outgoing.take() {
            let _ = tx.send(Outgoing::Close);
        }
        self.incoming = None;
        self.state.store(CLOSED, Ordering::Release);
    }
}

impl Default for WsClient {
    fn default() -> Self {
        Self::new()
    }
}

async fn run(
    url: String,
    protocol: String,
    state: Arc<AtomicU8>,
    incoming: mpsc::UnboundedSender<Frame>,
    mut outgoing: mpsc::UnboundedReceiver<Outgoing>,
) {
    let Ok(mut request) = url.as_str().into_client_request() else {
        tracing::warn!("comms websocket: invalid url {url}");
        state.store(CLOSED, Ordering::Release);
        return;
    };
    if let Some(value) = (!protocol.is_empty())
        .then(|| HeaderValue::from_str(&protocol).ok())
        .flatten()
    {
        request
            .headers_mut()
            .insert("Sec-WebSocket-Protocol", value);
    }
    let mut stream = match tokio_tungstenite::connect_async(request).await {
        Ok((stream, _)) => stream,
        Err(err) => {
            tracing::debug!("comms websocket: connect to {url} failed: {err}");
            state.store(CLOSED, Ordering::Release);
            return;
        }
    };
    // `close()` may have run while connecting; it already set CLOSED on this cell.
    if state
        .compare_exchange(CONNECTING, OPEN, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        let _ = stream.close(None).await;
        return;
    }
    loop {
        tokio::select! {
            msg = stream.next() => match msg {
                Some(Ok(Message::Binary(data))) => {
                    if incoming.send(Frame::Binary(data)).is_err() {
                        break;
                    }
                }
                Some(Ok(Message::Text(text))) => {
                    if incoming.send(Frame::Text(text)).is_err() {
                        break;
                    }
                }
                Some(Ok(Message::Ping(data))) => {
                    let _ = stream.send(Message::Pong(data)).await;
                }
                Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                Some(Ok(_)) => {}
            },
            out = outgoing.recv() => match out {
                Some(Outgoing::Frame(message)) => {
                    if stream.send(message).await.is_err() {
                        break;
                    }
                }
                Some(Outgoing::Close) | None => {
                    let _ = stream.close(None).await;
                    break;
                }
            },
        }
    }
    state.store(CLOSED, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::net::TcpListener;

    async fn wait_for(mut cond: impl FnMut() -> bool) {
        for _ in 0..100 {
            if cond() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("timed out");
    }

    #[tokio::test]
    async fn echoes_binary_frames_and_reports_state() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.unwrap();
            let mut ws = tokio_tungstenite::accept_async(tcp).await.unwrap();
            while let Some(Ok(msg)) = ws.next().await {
                if msg.is_binary() {
                    ws.send(msg).await.unwrap();
                }
            }
        });

        let mut client = WsClient::new();
        assert_eq!(client.state(), WsState::Closed);
        assert!(!client.send(vec![1]));
        client.connect(&format!("ws://{addr}"), "rfc5");
        wait_for(|| client.state() == WsState::Open).await;

        assert!(client.send(vec![1, 2, 3]));
        let mut got = None;
        wait_for(|| {
            got = client.next_packet();
            got.is_some()
        })
        .await;
        assert_eq!(got.unwrap(), vec![1, 2, 3]);

        client.close();
        assert_eq!(client.state(), WsState::Closed);
    }

    #[tokio::test]
    async fn refused_connection_ends_closed() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        let mut client = WsClient::new();
        client.connect(&format!("ws://{addr}"), "rfc5");
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert_eq!(client.state(), WsState::Closed);
    }
}
