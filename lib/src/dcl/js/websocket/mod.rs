use std::{cell::RefCell, collections::HashMap, rc::Rc};

use deno_core::{error::AnyError, op2, OpDecl, OpState};
use futures_util::{SinkExt, StreamExt};
use http::{HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::{client::IntoClientRequest, protocol::CloseFrame};

pub fn ops() -> Vec<OpDecl> {
    vec![op_ws_create(), op_ws_cleanup(), op_ws_send(), op_ws_poll()]
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
enum WsSendData {
    Binary { data: Vec<u8> },
    Text { data: String },
    Close,
}

enum WsReceiveData {
    BinaryData(Vec<u8>),
    TextData(String),
    Error(AnyError),
    Close(Option<CloseFrame<'static>>),
    Connected,
}

struct WsState {
    counter: u32,
    ws_receiver: HashMap<u32, tokio::sync::mpsc::Receiver<WsReceiveData>>,
    ws_sender: HashMap<u32, tokio::sync::mpsc::Sender<WsSendData>>,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum WsPoll {
    Connected,
    Closed { code: Option<u16>, reason: String },
    BinaryData { data: Vec<u8> },
    TextData { data: String },
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

    tracing::debug!("request to {:?}", http_request);

    let connection_result = tokio_tungstenite::connect_async(http_request).await;

    let (ws_stream, _) = match connection_result {
        Ok(connection_result) => connection_result,
        Err(err) => match err {
            tokio_tungstenite::tungstenite::Error::Http(http_err) => {
                let body_error = http_err
                    .body()
                    .as_ref()
                    .map(|body| String::from_utf8_lossy(body));
                return Err(anyhow::Error::msg(format!("http error: {:?}", body_error)));
            }
            err => {
                tracing::error!("error connecting to {:?}: {:?}", url, err);
                return Err(err.into());
            }
        },
    };
    tracing::debug!("connected to {:?}", url);
    sender.send(WsReceiveData::Connected).await?;

    tracing::debug!("status sent");
    let (mut ws_send, mut read) = ws_stream.split();

    let sender_a = sender.clone();

    // make local channel
    let (int_sender, mut int_receiver) = tokio::sync::mpsc::channel(5);

    tokio::join!(
        async move {
            loop {
                // With select approach
                let final_data: Option<tokio_tungstenite::tungstenite::Message>;
                let mut critical_cond = false;

                tokio::select! {
                    to_send = receiver.recv() => {
                        final_data = match to_send {
                            Some(WsSendData::Binary { data }) => Some(tokio_tungstenite::tungstenite::Message::Binary(data)),
                            Some(WsSendData::Text { data }) => Some(tokio_tungstenite::tungstenite::Message::Text(data)),
                            Some(WsSendData::Close) => None,
                            None => {
                                critical_cond = true;
                                None
                            }
                        };

                    },
                    to_send = int_receiver.recv() => {
                        final_data = to_send;
                    }
                };

                if let Some(data) = final_data {
                    if ws_send.send(data).await.is_err() {
                        break;
                    }
                } else {
                    if critical_cond {
                        let _ = sender_a
                            .send(WsReceiveData::Error(anyhow::Error::msg("none from sender")))
                            .await;
                    }
                    let _ = ws_send.close().await;
                    break;
                }
            }
        },
        async move {
            loop {
                let data_received = read.next().await;
                tracing::debug!("receiving {:?}", data_received);
                let result = match data_received {
                    Some(Ok(tokio_tungstenite::tungstenite::Message::Frame(_data))) => {
                        tracing::error!("unsupported frame type");
                        Some(())
                    }
                    Some(Ok(tokio_tungstenite::tungstenite::Message::Binary(data))) => {
                        sender.send(WsReceiveData::BinaryData(data)).await.ok()
                    }
                    Some(Ok(tokio_tungstenite::tungstenite::Message::Text(data))) => {
                        sender.send(WsReceiveData::TextData(data)).await.ok()
                    }
                    Some(Ok(tokio_tungstenite::tungstenite::Message::Ping(data))) => int_sender
                        .send(tokio_tungstenite::tungstenite::Message::Pong(data))
                        .await
                        .ok(),
                    Some(Ok(tokio_tungstenite::tungstenite::Message::Pong(_data))) => Some(()),
                    Some(Ok(tokio_tungstenite::tungstenite::Message::Close(data))) => {
                        let _ = sender.send(WsReceiveData::Close(data)).await;
                        None
                    }
                    Some(Err(err)) => {
                        let _ = sender.send(WsReceiveData::Error(err.into())).await;
                        None
                    }
                    None => {
                        let _ = sender
                            .send(WsReceiveData::Error(anyhow::Error::msg(
                                "data receiver closed",
                            )))
                            .await;
                        None
                    }
                };
                if result.is_none() {
                    break;
                }
            }
        }
    );
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

    let (ws_resource_id, recv_send_data, send_ondata) = {
        let mut state = op_state.borrow_mut();
        let ws_state = state.borrow_mut::<WsState>();
        ws_state.counter += 1;

        let id = ws_state.counter;
        let (sender, recv_send_data) = tokio::sync::mpsc::channel(100);
        let (send_ondata, receiver) = tokio::sync::mpsc::channel(100);

        ws_state.ws_receiver.insert(id, receiver);
        ws_state.ws_sender.insert(id, sender);

        (id, recv_send_data, send_ondata)
    };

    tokio::spawn(async move {
        let result = ws_poll(url, protocols, recv_send_data, send_ondata.clone()).await;
        tracing::info!("websocket task finished with result: {:?}", result);
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
        Some(WsReceiveData::Connected) => Ok(WsPoll::Connected),
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

    ws_state.ws_sender.remove(&res_id);

    Ok(())
}

#[cfg(test)]
mod tests {
    //! Integration tests for the native `ws_poll` transport task, exercised
    //! against a local `tokio-tungstenite` server. These lock down the
    //! transport behaviour scenes rely on — connect, text/binary round-trip,
    //! message ordering, ping/pong, close-code propagation and connection
    //! failure — and guard against regressions of the Colyseus WebSocket bug
    //! (#2430), whose sibling defect was the close code/reason being dropped.
    //!
    //! Run with: `cargo test -p dclgodot websocket`

    use std::future::Future;
    use std::time::Duration;

    use futures_util::{SinkExt, StreamExt};
    use tokio::net::TcpListener;
    use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
    use tokio_tungstenite::tungstenite::protocol::CloseFrame;
    use tokio_tungstenite::tungstenite::Message;
    use tokio_tungstenite::WebSocketStream;

    use super::{ws_poll, WsReceiveData, WsSendData};

    type ServerWs = WebSocketStream<tokio::net::TcpStream>;

    /// Bind an ephemeral local server, accept ONE connection, run `handler`,
    /// and return the `ws://` URL to dial.
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

    /// Wire up the JS<->native channels the same way `op_ws_create` does and
    /// spawn the real `ws_poll` task.
    fn connect(url: String) -> Client {
        let (to_native, rx_send) = tokio::sync::mpsc::channel(100);
        let (tx_recv, from_native) = tokio::sync::mpsc::channel(100);
        tokio::spawn(ws_poll(url, vec![], rx_send, tx_recv));
        Client {
            to_native,
            from_native,
        }
    }

    /// Await the next native->JS event, failing the test on timeout/close.
    async fn next(c: &mut Client) -> WsReceiveData {
        tokio::time::timeout(Duration::from_secs(5), c.from_native.recv())
            .await
            .expect("timed out waiting for a native event")
            .expect("native->JS channel closed unexpectedly")
    }

    /// A server that echoes text/binary frames and stops on close.
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
            matches!(next(&mut c).await, WsReceiveData::Connected),
            "the first native event must be Connected"
        );
        c.to_native.send(WsSendData::Close).await.unwrap();
    }

    #[tokio::test]
    async fn echoes_text() {
        let url = serve(echo_server).await;
        let mut c = connect(url);
        assert!(matches!(next(&mut c).await, WsReceiveData::Connected));

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
        assert!(matches!(next(&mut c).await, WsReceiveData::Connected));

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
        assert!(matches!(next(&mut c).await, WsReceiveData::Connected));

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
        assert!(matches!(next(&mut c).await, WsReceiveData::Connected));

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
    async fn responds_to_server_ping_with_pong() {
        let (tx, rx) = tokio::sync::oneshot::channel::<Vec<u8>>();
        let url = serve(|mut ws: ServerWs| async move {
            ws.send(Message::Ping(vec![9, 8, 7])).await.unwrap();
            let mut tx = Some(tx);
            while let Some(Ok(m)) = ws.next().await {
                if let Message::Pong(p) = m {
                    if let Some(tx) = tx.take() {
                        let _ = tx.send(p);
                    }
                    break;
                }
            }
        })
        .await;

        let mut c = connect(url);
        assert!(matches!(next(&mut c).await, WsReceiveData::Connected));

        // The client must auto-pong; the ping must not surface to the JS side.
        let pong = tokio::time::timeout(Duration::from_secs(5), rx)
            .await
            .expect("server never received a pong")
            .expect("pong channel dropped");
        assert_eq!(
            pong,
            vec![9, 8, 7],
            "pong payload must echo the ping payload"
        );
    }

    /// Regression test for #2430's sibling defect: the server's close code and
    /// reason must survive all the way to the native->JS boundary.
    #[tokio::test]
    async fn server_close_propagates_code_and_reason() {
        let url = serve(|mut ws: ServerWs| async move {
            ws.send(Message::Close(Some(CloseFrame {
                code: CloseCode::Away, // 1001
                reason: "going away".into(),
            })))
            .await
            .unwrap();
            // Drive the close handshake to completion.
            while let Some(Ok(_)) = ws.next().await {}
        })
        .await;

        let mut c = connect(url);
        loop {
            match next(&mut c).await {
                WsReceiveData::Connected => continue,
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

    #[tokio::test]
    async fn client_close_is_seen_by_server() {
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let url = serve(|mut ws: ServerWs| async move {
            loop {
                match ws.next().await {
                    Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                    Some(Ok(_)) => {}
                }
            }
            let _ = tx.send(());
        })
        .await;

        let mut c = connect(url);
        assert!(matches!(next(&mut c).await, WsReceiveData::Connected));
        c.to_native.send(WsSendData::Close).await.unwrap();

        tokio::time::timeout(Duration::from_secs(5), rx)
            .await
            .expect("server never observed the client close")
            .expect("close-signal channel dropped");
    }

    #[tokio::test]
    async fn connect_failure_returns_err_without_connected() {
        // Accept the TCP connection then drop it before completing the
        // WebSocket handshake -> deterministic handshake failure.
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
