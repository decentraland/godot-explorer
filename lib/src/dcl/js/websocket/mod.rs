use std::time::Duration;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use deno_core::{error::AnyError, op2, OpDecl, OpState};
use futures_util::{SinkExt, StreamExt};
use http::{HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::protocol::{CloseFrame, WebSocketConfig};
use tokio_tungstenite::tungstenite::{client::IntoClientRequest, Message};

pub fn ops() -> Vec<OpDecl> {
    vec![op_ws_create(), op_ws_cleanup(), op_ws_send(), op_ws_poll()]
}

/// Give up on the opening handshake after this long.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
/// Give up on a single frame write after this long (wedged/adversarial peer).
const WRITE_TIMEOUT: Duration = Duration::from_secs(30);
/// After we initiate close, wait at most this long for the peer's close frame
/// before tearing the socket down so it can never wedge in CLOSING.
const CLOSE_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
/// Reject inbound messages/frames larger than this.
const MAX_MESSAGE_SIZE: usize = 16 << 20; // 16 MiB
const MAX_FRAME_SIZE: usize = 16 << 20; // 16 MiB
/// Backstop against a scene opening sockets in a loop and leaking them.
const MAX_CONNECTIONS_PER_SCENE: usize = 50;

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
enum WsSendData {
    Binary {
        data: Vec<u8>,
    },
    Text {
        data: String,
    },
    Close {
        code: Option<u16>,
        reason: Option<String>,
    },
}

enum WsReceiveData {
    BinaryData(Vec<u8>),
    TextData(String),
    Error(AnyError),
    Close(Option<CloseFrame<'static>>),
    Connected { protocol: Option<String> },
}

struct WsState {
    counter: u32,
    ws_receiver: HashMap<u32, tokio::sync::mpsc::Receiver<WsReceiveData>>,
    ws_sender: HashMap<u32, tokio::sync::mpsc::Sender<WsSendData>>,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum WsPoll {
    Connected { protocol: Option<String> },
    Closed { code: Option<u16>, reason: String },
    BinaryData { data: Vec<u8> },
    TextData { data: String },
}

/// Write a frame with a bounded timeout so a wedged peer cannot pin us forever.
async fn write<S>(ws_send: &mut S, message: Message) -> Result<(), ()>
where
    S: futures_util::Sink<Message> + Unpin,
{
    match tokio::time::timeout(WRITE_TIMEOUT, ws_send.send(message)).await {
        Ok(Ok(())) => Ok(()),
        _ => Err(()),
    }
}

async fn ws_poll(
    url: String,
    protocols: Vec<String>,
    mut receiver: tokio::sync::mpsc::Receiver<WsSendData>,
    sender: tokio::sync::mpsc::Sender<WsReceiveData>,
) -> Result<(), AnyError> {
    tracing::debug!("connecting to {:?}", url);

    let mut http_request = url.clone().into_client_request()?;

    if !protocols.is_empty() {
        let protocols = protocols.join(",");
        http_request.headers_mut().insert(
            HeaderName::from_static("sec-websocket-protocol"),
            HeaderValue::from_str(&protocols)?,
        );
    }

    http_request.headers_mut().insert(
        HeaderName::from_static("user-agent"),
        HeaderValue::from_static("DCLExplorer/0.1"),
    );

    http_request.headers_mut().insert(
        HeaderName::from_static("origin"),
        HeaderValue::from_static("https://decentraland.org"),
    );

    http_request.headers_mut().insert(
        HeaderName::from_static("accept"),
        HeaderValue::from_static("*/*"),
    );

    let config = WebSocketConfig {
        max_message_size: Some(MAX_MESSAGE_SIZE),
        max_frame_size: Some(MAX_FRAME_SIZE),
        ..Default::default()
    };

    let connect = tokio_tungstenite::connect_async_with_config(http_request, Some(config), false);
    let (ws_stream, response) = match tokio::time::timeout(CONNECT_TIMEOUT, connect).await {
        Err(_) => return Err(anyhow::Error::msg("connection timed out")),
        Ok(Err(err)) => match err {
            tokio_tungstenite::tungstenite::Error::Http(http_err) => {
                let body_error = http_err
                    .body()
                    .as_ref()
                    .map(|body| String::from_utf8_lossy(body));
                return Err(anyhow::Error::msg(format!("http error: {body_error:?}")));
            }
            err => {
                tracing::error!("error connecting to {:?}: {:?}", url, err);
                return Err(err.into());
            }
        },
        Ok(Ok(value)) => value,
    };
    tracing::debug!("connected to {:?}", url);

    // Report the subprotocol the server actually selected (if any).
    let protocol = response
        .headers()
        .get("sec-websocket-protocol")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    sender.send(WsReceiveData::Connected { protocol }).await?;

    let (mut ws_send, mut read) = ws_stream.split();

    // Close-handshake deadline: starts effectively-never and is armed the moment
    // we initiate a close, so a peer that ignores our close cannot wedge us.
    let close_deadline = tokio::time::sleep(Duration::from_secs(60 * 60 * 24 * 365));
    tokio::pin!(close_deadline);
    let mut closing = false;

    loop {
        tokio::select! {
            // App -> socket. Stop accepting new app data once we are closing.
            to_send = receiver.recv(), if !closing => {
                match to_send {
                    Some(WsSendData::Binary { data }) => {
                        if write(&mut ws_send, Message::Binary(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(WsSendData::Text { data }) => {
                        if write(&mut ws_send, Message::Text(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(WsSendData::Close { code, reason }) => {
                        let frame = code.map(|code| CloseFrame {
                            code: code.into(),
                            reason: reason.unwrap_or_default().into(),
                        });
                        let _ = write(&mut ws_send, Message::Close(frame)).await;
                        closing = true;
                        close_deadline
                            .as_mut()
                            .reset(tokio::time::Instant::now() + CLOSE_HANDSHAKE_TIMEOUT);
                    }
                    None => {
                        // JS side went away without close(): shut the socket down.
                        let _ = ws_send.close().await;
                        break;
                    }
                }
            }

            // Socket -> app.
            data_received = read.next() => {
                match data_received {
                    Some(Ok(Message::Binary(data))) => {
                        if sender.send(WsReceiveData::BinaryData(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Text(data))) => {
                        if sender.send(WsReceiveData::TextData(data)).await.is_err() {
                            break;
                        }
                    }
                    // tokio-tungstenite auto-responds to Ping (and flushes the Pong
                    // while reading), so no manual Pong is needed here.
                    Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) => {}
                    Some(Ok(Message::Close(frame))) => {
                        let _ = sender.send(WsReceiveData::Close(frame)).await;
                        let _ = ws_send.close().await;
                        break;
                    }
                    // Raw frames are not produced by the Stream API; ignore defensively.
                    Some(Ok(Message::Frame(_))) => {}
                    Some(Err(err)) => {
                        let _ = sender.send(WsReceiveData::Error(err.into())).await;
                        break;
                    }
                    None => {
                        // Stream ended. Expected if we asked to close; otherwise
                        // it is an abnormal drop the scene should hear about.
                        if closing {
                            let _ = sender.send(WsReceiveData::Close(None)).await;
                        } else {
                            let _ = sender
                                .send(WsReceiveData::Error(anyhow::Error::msg(
                                    "connection closed",
                                )))
                                .await;
                        }
                        break;
                    }
                }
            }

            // The peer never completed the close handshake in time.
            _ = &mut close_deadline, if closing => {
                let _ = sender.send(WsReceiveData::Close(None)).await;
                break;
            }
        }
    }
    Ok(())
}

#[op2]
fn op_ws_create(
    op_state: Rc<RefCell<OpState>>,
    #[string] url: String,
    #[serde] protocols: Vec<String>,
) -> u32 {
    let has_ws_state = op_state.borrow().has::<WsState>();
    if !has_ws_state {
        op_state.borrow_mut().put::<WsState>(WsState {
            counter: 0,
            ws_receiver: HashMap::new(),
            ws_sender: HashMap::new(),
        });
    }

    let (ws_resource_id, recv_send_data, send_ondata, over_cap) = {
        let mut state = op_state.borrow_mut();
        let ws_state = state.borrow_mut::<WsState>();
        // Count live connections before allocating a new id.
        let over_cap = ws_state.ws_sender.len() >= MAX_CONNECTIONS_PER_SCENE;
        ws_state.counter += 1;

        let id = ws_state.counter;
        let (sender, recv_send_data) = tokio::sync::mpsc::channel(100);
        let (send_ondata, receiver) = tokio::sync::mpsc::channel(100);

        ws_state.ws_receiver.insert(id, receiver);
        ws_state.ws_sender.insert(id, sender);

        (id, recv_send_data, send_ondata, over_cap)
    };

    tokio::spawn(async move {
        if over_cap {
            let _ = send_ondata
                .send(WsReceiveData::Error(anyhow::Error::msg(
                    "too many WebSocket connections open for this scene",
                )))
                .await;
        } else {
            let result = ws_poll(url, protocols, recv_send_data, send_ondata.clone()).await;
            tracing::info!("websocket task finished with result: {:?}", result);
            // A connection failure returns Err without ever emitting Connected;
            // surface it so the JS side can fire `onerror` before `onclose`.
            if let Err(err) = result {
                let _ = send_ondata.send(WsReceiveData::Error(err)).await;
            }
        }
        let _ = send_ondata.send(WsReceiveData::Close(None)).await;
    });

    ws_resource_id
}

#[op2(async)]
#[serde]
async fn op_ws_poll(op_state: Rc<RefCell<OpState>>, res_id: u32) -> Result<WsPoll, AnyError> {
    let mut receiver = {
        let mut state = op_state.borrow_mut();
        let ws_state = state.borrow_mut::<WsState>();
        let receiver = ws_state.ws_receiver.remove(&res_id);

        if receiver.is_none() {
            return Err(anyhow::Error::msg("invalid resource id"));
        }

        receiver.unwrap()
    };

    let data = match receiver.recv().await {
        Some(WsReceiveData::BinaryData(data)) => Ok(WsPoll::BinaryData { data }),
        Some(WsReceiveData::TextData(data)) => Ok(WsPoll::TextData { data }),
        Some(WsReceiveData::Connected { protocol }) => Ok(WsPoll::Connected { protocol }),
        Some(WsReceiveData::Error(err)) => Err(err),
        Some(WsReceiveData::Close(data)) => {
            let (code, reason) = match data {
                Some(frame) => (Some(u16::from(frame.code)), frame.reason.into_owned()),
                None => (None, String::new()),
            };
            Ok(WsPoll::Closed { code, reason })
        }
        None => Err(anyhow::Error::msg("none")),
    };

    let mut state = op_state.borrow_mut();
    let ws_state = state.borrow_mut::<WsState>();
    ws_state.ws_receiver.insert(res_id, receiver);

    data
}

#[op2(async)]
async fn op_ws_send(
    op_state: Rc<RefCell<OpState>>,
    res_id: u32,
    #[serde] event: WsSendData,
) -> Result<(), AnyError> {
    let sender = {
        let state = op_state.borrow_mut();
        let sender = state.borrow::<WsState>().ws_sender.get(&res_id);
        if sender.is_none() {
            return Err(anyhow::Error::msg("invalid resource id"));
        }
        sender.unwrap().clone()
    };

    sender.send(event).await.map_err(anyhow::Error::from)
}

#[op2(fast)]
fn op_ws_cleanup(state: &mut OpState, res_id: u32) -> Result<(), AnyError> {
    tracing::debug!("cleanup {:?}", res_id);

    let ws_state = state.borrow_mut::<WsState>();

    if let Some(mut receiver) = ws_state.ws_receiver.remove(&res_id) {
        receiver.close();
    }

    // Dropping the sender lets the native `ws_poll` task observe the closed
    // channel and terminate, releasing the socket instead of leaking it.
    ws_state.ws_sender.remove(&res_id);

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Integration tests for the native `ws_poll` transport task, exercised
    //! against a local `tokio-tungstenite` server (no new dependencies). These
    //! lock down connect, text/binary round-trip, ordering, single-pong,
    //! ping-does-not-block-reads, close-code propagation (both directions),
    //! the close-handshake timeout, subprotocol negotiation and connection
    //! failure — and guard against regressions of the Colyseus bug (#2430).
    //!
    //! Run with: `cargo test -p dclgodot websocket`

    use std::future::Future;
    use std::time::{Duration, Instant};

    use futures_util::{SinkExt, StreamExt};
    use tokio::net::TcpListener;
    use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
    use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
    use tokio_tungstenite::tungstenite::protocol::CloseFrame;
    use tokio_tungstenite::tungstenite::Message;
    use tokio_tungstenite::WebSocketStream;

    use super::{ws_poll, WsReceiveData, WsSendData};

    type ServerWs = WebSocketStream<tokio::net::TcpStream>;

    async fn serve<F, Fut>(handler: F) -> String
    where
        F: FnOnce(ServerWs) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let ws = tokio_tungstenite::accept_async(stream).await.unwrap();
            handler(ws).await;
        });
        format!("ws://{addr}/")
    }

    struct Client {
        to_native: tokio::sync::mpsc::Sender<WsSendData>,
        from_native: tokio::sync::mpsc::Receiver<WsReceiveData>,
    }

    fn connect(url: String) -> Client {
        connect_with(url, vec![])
    }

    fn connect_with(url: String, protocols: Vec<String>) -> Client {
        let (to_native, rx_send) = tokio::sync::mpsc::channel(100);
        let (tx_recv, from_native) = tokio::sync::mpsc::channel(100);
        tokio::spawn(ws_poll(url, protocols, rx_send, tx_recv));
        Client {
            to_native,
            from_native,
        }
    }

    async fn next(c: &mut Client) -> WsReceiveData {
        tokio::time::timeout(Duration::from_secs(8), c.from_native.recv())
            .await
            .expect("timed out waiting for a native event")
            .expect("native->JS channel closed unexpectedly")
    }

    fn close(code: Option<u16>, reason: Option<&str>) -> WsSendData {
        WsSendData::Close {
            code,
            reason: reason.map(str::to_string),
        }
    }

    async fn echo_server(mut ws: ServerWs) {
        while let Some(Ok(m)) = ws.next().await {
            match m {
                Message::Text(_) | Message::Binary(_) => {
                    if ws.send(m).await.is_err() {
                        break;
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    }

    #[tokio::test]
    async fn reports_connected_first() {
        let url = serve(echo_server).await;
        let mut c = connect(url);
        assert!(
            matches!(next(&mut c).await, WsReceiveData::Connected { .. }),
            "the first native event must be Connected"
        );
        c.to_native.send(close(None, None)).await.unwrap();
    }

    #[tokio::test]
    async fn echoes_text() {
        let url = serve(echo_server).await;
        let mut c = connect(url);
        assert!(matches!(
            next(&mut c).await,
            WsReceiveData::Connected { .. }
        ));

        c.to_native
            .send(WsSendData::Text {
                data: "hello world".into(),
            })
            .await
            .unwrap();

        match next(&mut c).await {
            WsReceiveData::TextData(s) => assert_eq!(s, "hello world"),
            _ => panic!("expected TextData echo"),
        }
    }

    #[tokio::test]
    async fn echoes_binary() {
        let url = serve(echo_server).await;
        let mut c = connect(url);
        assert!(matches!(
            next(&mut c).await,
            WsReceiveData::Connected { .. }
        ));

        let payload = vec![0u8, 1, 2, 3, 255, 254, 42];
        c.to_native
            .send(WsSendData::Binary {
                data: payload.clone(),
            })
            .await
            .unwrap();

        match next(&mut c).await {
            WsReceiveData::BinaryData(b) => assert_eq!(b, payload),
            _ => panic!("expected BinaryData echo"),
        }
    }

    #[tokio::test]
    async fn preserves_order_under_burst() {
        let url = serve(echo_server).await;
        let mut c = connect(url);
        assert!(matches!(
            next(&mut c).await,
            WsReceiveData::Connected { .. }
        ));

        for i in 0..50 {
            c.to_native
                .send(WsSendData::Text {
                    data: format!("msg-{i}"),
                })
                .await
                .unwrap();
        }
        for i in 0..50 {
            match next(&mut c).await {
                WsReceiveData::TextData(s) => assert_eq!(s, format!("msg-{i}")),
                _ => panic!("expected in-order TextData"),
            }
        }
    }

    #[tokio::test]
    async fn round_trips_large_binary() {
        let url = serve(echo_server).await;
        let mut c = connect(url);
        assert!(matches!(
            next(&mut c).await,
            WsReceiveData::Connected { .. }
        ));

        let payload: Vec<u8> = (0..1_000_000).map(|i| (i % 251) as u8).collect();
        c.to_native
            .send(WsSendData::Binary {
                data: payload.clone(),
            })
            .await
            .unwrap();

        match next(&mut c).await {
            WsReceiveData::BinaryData(b) => assert_eq!(b, payload),
            _ => panic!("expected large BinaryData echo"),
        }
    }

    #[tokio::test]
    async fn responds_to_server_ping_with_a_single_pong() {
        let (tx, rx) = tokio::sync::oneshot::channel::<usize>();
        let url = serve(|mut ws: ServerWs| async move {
            ws.send(Message::Ping(vec![9, 8, 7])).await.unwrap();
            let mut pongs = 0usize;
            let deadline = tokio::time::sleep(Duration::from_millis(500));
            tokio::pin!(deadline);
            loop {
                tokio::select! {
                    _ = &mut deadline => break,
                    m = ws.next() => match m {
                        Some(Ok(Message::Pong(_))) => pongs += 1,
                        Some(Ok(Message::Close(_))) | None => break,
                        _ => {}
                    }
                }
            }
            let _ = tx.send(pongs);
        })
        .await;

        let mut c = connect(url);
        assert!(matches!(
            next(&mut c).await,
            WsReceiveData::Connected { .. }
        ));

        let pongs = tokio::time::timeout(Duration::from_secs(5), rx)
            .await
            .expect("server never finished counting pongs")
            .expect("pong channel dropped");
        assert_eq!(
            pongs, 1,
            "exactly one pong must be sent (no manual duplicate)"
        );
    }

    // Regression guard for the old join!/int-channel deadlock: pings must never
    // stall the read path.
    #[tokio::test]
    async fn pings_do_not_block_inbound_data() {
        let url = serve(|mut ws: ServerWs| async move {
            for _ in 0..10 {
                ws.send(Message::Ping(vec![0])).await.unwrap();
            }
            ws.send(Message::Text("after-pings".into())).await.unwrap();
            while let Some(Ok(m)) = ws.next().await {
                if m.is_close() {
                    break;
                }
            }
        })
        .await;

        let mut c = connect(url);
        assert!(matches!(
            next(&mut c).await,
            WsReceiveData::Connected { .. }
        ));
        match next(&mut c).await {
            WsReceiveData::TextData(s) => assert_eq!(s, "after-pings"),
            _ => panic!("expected the Text that follows the pings"),
        }
    }

    // Regression guard for the receive half of #2430.
    #[tokio::test]
    async fn server_close_propagates_code_and_reason() {
        let url = serve(|mut ws: ServerWs| async move {
            ws.send(Message::Close(Some(CloseFrame {
                code: CloseCode::Away, // 1001
                reason: "going away".into(),
            })))
            .await
            .unwrap();
            while let Some(Ok(_)) = ws.next().await {}
        })
        .await;

        let mut c = connect(url);
        loop {
            match next(&mut c).await {
                WsReceiveData::Connected { .. } => continue,
                WsReceiveData::Close(Some(frame)) => {
                    assert_eq!(u16::from(frame.code), 1001, "close code must be preserved");
                    assert_eq!(frame.reason, "going away", "close reason must be preserved");
                    break;
                }
                WsReceiveData::Close(None) => panic!("close frame lost its code/reason"),
                _ => panic!("unexpected event before close"),
            }
        }
    }

    // The client's close code + reason must reach the peer (send half of #2430).
    #[tokio::test]
    async fn client_close_sends_code_and_reason() {
        let (tx, rx) = tokio::sync::oneshot::channel::<Option<(u16, String)>>();
        let url = serve(|mut ws: ServerWs| async move {
            let mut tx = Some(tx);
            while let Some(msg) = ws.next().await {
                if let Ok(Message::Close(frame)) = msg {
                    let got = frame.map(|f| (u16::from(f.code), f.reason.into_owned()));
                    if let Some(tx) = tx.take() {
                        let _ = tx.send(got);
                    }
                    break;
                }
            }
        })
        .await;

        let mut c = connect(url);
        assert!(matches!(
            next(&mut c).await,
            WsReceiveData::Connected { .. }
        ));
        c.to_native
            .send(close(Some(4001), Some("kick")))
            .await
            .unwrap();

        let got = tokio::time::timeout(Duration::from_secs(5), rx)
            .await
            .expect("server never saw the close")
            .expect("close channel dropped");
        assert_eq!(got, Some((4001, "kick".to_string())));
    }

    // A peer that ignores our close must not wedge us: ws_poll terminates and
    // the JS side receives a close within the handshake timeout.
    #[tokio::test]
    async fn close_handshake_times_out_when_peer_is_silent() {
        let url = serve(|ws: ServerWs| async move {
            // Never read our close frame (so tungstenite can't auto-echo it) and
            // hold the TCP connection open -> forces our close-handshake timeout.
            let _held_open = ws;
            futures_util::future::pending::<()>().await;
        })
        .await;

        let mut c = connect(url);
        assert!(matches!(
            next(&mut c).await,
            WsReceiveData::Connected { .. }
        ));
        let started = Instant::now();
        c.to_native.send(close(Some(1000), None)).await.unwrap();

        loop {
            match tokio::time::timeout(Duration::from_secs(15), c.from_native.recv()).await {
                Ok(Some(WsReceiveData::Close(_))) | Ok(None) => break,
                Ok(Some(_)) => continue,
                Err(_) => panic!("ws_poll wedged: no close within timeout"),
            }
        }
        let elapsed = started.elapsed();
        assert!(
            elapsed >= Duration::from_secs(4) && elapsed < Duration::from_secs(12),
            "close should resolve via the ~5s handshake timeout, took {elapsed:?}"
        );
    }

    #[tokio::test]
    async fn reports_negotiated_subprotocol() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws = tokio_tungstenite::accept_hdr_async(
                stream,
                |_req: &Request, mut resp: Response| {
                    resp.headers_mut()
                        .insert("sec-websocket-protocol", "json".parse().unwrap());
                    Ok(resp)
                },
            )
            .await
            .unwrap();
            let _ = ws.next().await;
        });

        let mut c = connect_with(
            format!("ws://{addr}/"),
            vec!["json".into(), "msgpack".into()],
        );
        match next(&mut c).await {
            WsReceiveData::Connected { protocol } => {
                assert_eq!(protocol.as_deref(), Some("json"));
            }
            _ => panic!("expected Connected with the negotiated subprotocol"),
        }
    }

    #[tokio::test]
    async fn connect_failure_returns_err_without_connected() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            if let Ok((stream, _)) = listener.accept().await {
                drop(stream);
            }
        });

        let (_to_native, rx_send) = tokio::sync::mpsc::channel(100);
        let (tx_recv, mut from_native) = tokio::sync::mpsc::channel(100);

        let res = ws_poll(format!("ws://{addr}/"), vec![], rx_send, tx_recv).await;
        assert!(res.is_err(), "a failed handshake must return Err");
        assert!(
            from_native.try_recv().is_err(),
            "no Connected event may be emitted on connection failure"
        );
    }
}
