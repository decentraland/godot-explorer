use std::{cell::RefCell, collections::HashMap, rc::Rc};

use deno_core::{error::AnyError, op, Op, OpDecl, OpState};
use futures_util::{SinkExt, StreamExt};
use http::{HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::{client::IntoClientRequest, protocol::CloseFrame};

pub fn ops() -> Vec<OpDecl> {
    vec![
        op_ws_create::DECL,
        op_ws_cleanup::DECL,
        op_ws_send::DECL,
        op_ws_poll::DECL,
    ]
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
    Closed,
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

#[op]
fn op_ws_create(op_state: Rc<RefCell<OpState>>, url: String, protocols: Vec<String>) -> u32 {
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

#[op]
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
            if let Some(_data) = data {
                Ok(WsPoll::Closed)
            } else {
                Ok(WsPoll::Closed)
            }
        }
        None => Err(anyhow::Error::msg("none")),
    };

    let mut state = op_state.borrow_mut();
    let ws_state = state.borrow_mut::<WsState>();
    ws_state.ws_receiver.insert(res_id, receiver);

    data
}

#[op]
async fn op_ws_send(
    op_state: Rc<RefCell<OpState>>,
    res_id: u32,
    event: WsSendData,
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

#[op]
fn op_ws_cleanup(state: &mut OpState, res_id: u32) -> Result<(), AnyError> {
    tracing::debug!("cleanup {:?}", res_id);

    let ws_state = state.borrow_mut::<WsState>();

    if let Some(mut receiver) = ws_state.ws_receiver.remove(&res_id) {
        receiver.close();
    }

    ws_state.ws_sender.remove(&res_id);

    Ok(())
}
