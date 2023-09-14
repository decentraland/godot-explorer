use std::{cell::RefCell, collections::HashMap, rc::Rc};

use deno_core::{
    error::AnyError,
    futures::{SinkExt, StreamExt},
    op, Op, OpDecl, OpState,
};
use serde::Serialize;

pub fn ops() -> Vec<OpDecl> {
    vec![
        op_ws_create::DECL,
        op_ws_cleanup::DECL,
        op_ws_close::DECL,
        op_ws_poll::DECL,
        op_ws_send_bin::DECL,
        op_ws_send_text::DECL,
    ]
}

#[derive(Debug)]
enum WsSendData {
    Binary(Vec<u8>),
    Text(String),
}

enum WsReceiveData {
    BinaryData(Vec<u8>),
    TextData(String),
    Error,
    Close,
    Connected,
}

struct WsState {
    counter: u32,
    ws_receiver: HashMap<u32, tokio::sync::mpsc::Receiver<WsReceiveData>>,
    ws_sender: HashMap<u32, tokio::sync::mpsc::Sender<WsSendData>>,
}

#[derive(Serialize)]
struct WsPoll {
    connected: bool,
    binary_data: Option<Vec<u8>>,
    text_data: Option<String>,
    closed: bool,
}

async fn ws_poll(
    url: String,
    _protocols: Vec<String>, // TODO: implement
    mut receiver: tokio::sync::mpsc::Receiver<WsSendData>,
    sender: tokio::sync::mpsc::Sender<WsReceiveData>,
) -> Result<(), AnyError> {
    tracing::info!("connecting to {:?}", url);
    let (ws_stream, _) = tokio_tungstenite::connect_async(&url).await?;

    tracing::info!("connected to {:?}", url);
    sender.send(WsReceiveData::Connected).await?;

    tracing::info!("status sent");
    let (mut write, mut read) = ws_stream.split();

    loop {
        tokio::select! {
            to_send = receiver.recv() => {
                tracing::info!("to send {:?}", to_send);
                match to_send {
                    Some(WsSendData::Binary(data)) => {
                        write.send(tokio_tungstenite::tungstenite::Message::Binary(data)).await?;
                    }
                    Some(WsSendData::Text(data)) => {
                        write.send(tokio_tungstenite::tungstenite::Message::Text(data)).await?;
                    }
                    None => {
                        write.close().await?;
                        break;
                    }
                }
            }
            data_received = read.next() => {
                tracing::info!("receiving {:?}", data_received);
                match data_received {
                    Some(Ok(tokio_tungstenite::tungstenite::Message::Frame(_data))) => {
                        todo!("unsupported")
                    }
                    Some(Ok(tokio_tungstenite::tungstenite::Message::Binary(data))) => {
                        sender.send(WsReceiveData::BinaryData(data)).await?;
                    }
                    Some(Ok(tokio_tungstenite::tungstenite::Message::Text(data))) => {
                        sender.send(WsReceiveData::TextData(data)).await?;
                    }
                    Some(Ok(tokio_tungstenite::tungstenite::Message::Ping(data))) => {
                        sender.send(WsReceiveData::BinaryData(data)).await?;
                    }
                    Some(Ok(tokio_tungstenite::tungstenite::Message::Pong(data))) => {
                        sender.send(WsReceiveData::BinaryData(data)).await?;
                    }
                    Some(Ok(tokio_tungstenite::tungstenite::Message::Close(_data))) => {
                        // TODO: send close code
                        sender.send(WsReceiveData::Close).await?;
                        break;
                    }
                    Some(Err(_)) => {
                        sender.send(WsReceiveData::Error).await?;
                        break;
                    }
                    None => {
                        sender.send(WsReceiveData::Close).await?;
                        break;
                    }
                }

            }

        }
    }
    Ok(())
}

#[op]
fn op_ws_create(
    op_state: Rc<RefCell<OpState>>,
    url: String,
    protocols: Vec<String>,
) -> Result<u32, AnyError> {
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
        let result = ws_poll(url, protocols, recv_send_data, send_ondata).await;
        tracing::info!("websocket task finished with result: {:?}", result);
    });

    Ok(ws_resource_id)
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
        Some(WsReceiveData::BinaryData(data)) => Ok(WsPoll {
            connected: true,
            binary_data: Some(data),
            text_data: None,
            closed: false,
        }),
        Some(WsReceiveData::TextData(data)) => Ok(WsPoll {
            connected: true,
            binary_data: None,
            text_data: Some(data),
            closed: false,
        }),
        Some(WsReceiveData::Connected) => Ok(WsPoll {
            connected: true,
            binary_data: None,
            text_data: None,
            closed: false,
        }),
        _ => Ok(WsPoll {
            connected: false,
            binary_data: None,
            text_data: None,
            closed: true,
        }),
    };

    let mut state = op_state.borrow_mut();
    let ws_state = state.borrow_mut::<WsState>();
    ws_state.ws_receiver.insert(res_id, receiver);

    data
}

#[op]
fn op_ws_send_text(op_state: &mut OpState, res_id: u32, data: String) -> Result<(), AnyError> {
    let sender = {
        let sender = op_state.borrow::<WsState>().ws_sender.get(&res_id);
        if sender.is_none() {
            return Err(anyhow::Error::msg("invalid resource id"));
        }
        sender.unwrap().clone()
    };

    sender.blocking_send(WsSendData::Text(data))?;

    Ok(())
}

#[op]
fn op_ws_send_bin(op_state: &mut OpState, res_id: u32, data: Vec<u8>) -> Result<(), AnyError> {
    let sender = {
        let sender = op_state.borrow::<WsState>().ws_sender.get(&res_id);
        if sender.is_none() {
            return Err(anyhow::Error::msg("invalid resource id"));
        }
        sender.unwrap().clone()
    };

    sender.blocking_send(WsSendData::Binary(data))?;

    Ok(())
}

#[op]
fn op_ws_close(op_state: &mut OpState, res_id: u32) -> Result<(), AnyError> {
    op_state.borrow_mut::<WsState>().ws_sender.remove(&res_id);
    Ok(())
}

#[op]
fn op_ws_cleanup(state: &mut OpState, res_id: u32) -> Result<(), AnyError> {
    let ws_state = state.borrow_mut::<WsState>();

    if let Some(mut receiver) = ws_state.ws_receiver.remove(&res_id) {
        receiver.close();
    }

    ws_state.ws_sender.remove(&res_id);

    Ok(())
}
