use std::{cell::RefCell, rc::Rc};

use deno_core::{op, JsBuffer, Op, OpDecl, OpState};
use ethers::types::H160;

use crate::dcl::scene_apis::RpcCall;

#[derive(Default)]
pub(crate) struct InternalPendingBinaryMessages {
    pub messages: Vec<(H160, Vec<u8>)>,
}

// list of op declarations
pub fn ops() -> Vec<OpDecl> {
    vec![op_comms_send_string::DECL, op_comms_send_binary::DECL]
}

pub(crate) const COMMS_MSG_TYPE_STRING: u8 = 1;
pub(crate) const COMMS_MSG_TYPE_BINARY: u8 = 2;

#[op]
async fn op_comms_send_string(
    state: Rc<RefCell<OpState>>,
    message: String,
) -> Result<(), anyhow::Error> {
    let mut message = message.into_bytes();
    message.insert(0, COMMS_MSG_TYPE_STRING);
    comms_send(state, vec![message]).await?;
    Ok(())
}

#[op]
async fn op_comms_send_binary(
    state: Rc<RefCell<OpState>>,
    messages: Vec<JsBuffer>,
) -> Result<Vec<Vec<u8>>, anyhow::Error> {
    let messages = messages
        .iter()
        .map(|m| {
            let mut m = m.as_ref().to_vec();
            m.insert(0, COMMS_MSG_TYPE_BINARY);
            m
        })
        .collect();

    comms_send(state.clone(), messages).await?;

    // Get pending Binary messages
    if let Some(pending_messages) = state
        .borrow_mut()
        .try_take::<InternalPendingBinaryMessages>()
    {
        let messages = pending_messages
            .messages
            .into_iter()
            .map(|(sender_address, mut data)| {
                let sender_address_str = format!("{:#x}", sender_address);
                let sender_address_str_bytes = sender_address_str.as_bytes();

                // Remove the comms message type(-1 byte), add the sender address size (+1 byte)
                //  and add the address in bytes (for H160=20 to string 2+40)
                let sender_len = sender_address_str_bytes.len();
                let original_data_len = data.len();
                let new_data_size = original_data_len + 1 + sender_len - 1;

                // Resize to fit the sender address
                data.resize(new_data_size, 0);

                // Shift the data to the right to fit the sender address
                data.copy_within(1..original_data_len, sender_len + 1);

                // Add the sender address size at the beginning of the data
                data[0] = sender_len as u8;

                // Add the sender address at the beginning of the data
                data[1..=sender_len].copy_from_slice(sender_address_str_bytes);

                data
            })
            .collect();
        Ok(messages)
    } else {
        Ok(vec![])
    }
}

async fn comms_send(
    state: Rc<RefCell<OpState>>,
    message: Vec<Vec<u8>>,
) -> Result<(), anyhow::Error> {
    let (sx, rx) = tokio::sync::oneshot::channel::<Result<(), String>>();

    state
        .borrow_mut()
        .borrow_mut::<Vec<RpcCall>>()
        .push(RpcCall::SendCommsMessage {
            body: message,
            response: sx.into(),
        });

    rx.await
        .map_err(|e| anyhow::anyhow!(e))?
        .map_err(anyhow::Error::msg)
}
