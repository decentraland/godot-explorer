use std::{cell::RefCell, rc::Rc};

use deno_core::{op2, JsBuffer, OpDecl, OpState};
use ethers_core::types::H160;

use crate::dcl::scene_apis::RpcCall;

#[derive(Default)]
pub(crate) struct InternalPendingBinaryMessages {
    pub messages: Vec<(H160, Vec<u8>)>,
}

// list of op declarations
pub fn ops() -> Vec<OpDecl> {
    vec![
        op_comms_send_string(),
        op_comms_send_binary(),
        op_comms_send_binary_single(),
        op_comms_recv_binary(),
    ]
}

pub(crate) const COMMS_MSG_TYPE_STRING: u8 = 1;
pub(crate) const COMMS_MSG_TYPE_BINARY: u8 = 2;

/// Helper function to parse an address string (with or without 0x prefix) to H160
fn parse_address(address: &str) -> Option<H160> {
    let addr = if let Some(stripped) = address.strip_prefix("0x") {
        stripped
    } else {
        address
    };

    let hex_bytes = ethers_core::utils::hex::decode(addr).ok()?;
    if hex_bytes.len() != H160::len_bytes() {
        return None;
    }

    Some(H160::from_slice(hex_bytes.as_slice()))
}

#[op2(async)]
async fn op_comms_send_string(
    state: Rc<RefCell<OpState>>,
    #[string] message: String,
) -> Result<(), anyhow::Error> {
    let mut data = vec![COMMS_MSG_TYPE_STRING];
    data.extend(message.into_bytes());
    comms_send_single(state, data, None).await
}

/// Send a single binary message with optional recipient address
/// This is the new-style operation that supports targeted messaging
#[op2(async)]
async fn op_comms_send_binary_single(
    state: Rc<RefCell<OpState>>,
    #[buffer] message: JsBuffer,
    #[string] recipient: Option<String>,
) -> Result<(), anyhow::Error> {
    let mut data = vec![COMMS_MSG_TYPE_BINARY];
    data.extend(message.as_ref());

    let recipient = recipient.and_then(|r| parse_address(&r));

    comms_send_single(state, data, recipient).await
}

/// Internal helper to receive pending binary messages
fn recv_binary_internal(state: Rc<RefCell<OpState>>) -> Vec<Vec<u8>> {
    if let Some(pending_messages) = state
        .borrow_mut()
        .try_take::<InternalPendingBinaryMessages>()
    {
        pending_messages
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
            .collect()
    } else {
        vec![]
    }
}

/// Receive pending binary messages from other peers
/// Returns messages with sender address prepended
#[op2(async)]
#[serde]
async fn op_comms_recv_binary(state: Rc<RefCell<OpState>>) -> Result<Vec<Vec<u8>>, anyhow::Error> {
    Ok(recv_binary_internal(state))
}

/// Legacy operation for backwards compatibility
/// Sends multiple binary messages (old-style, broadcasts to all)
/// and returns pending received messages
#[op2(async)]
#[serde]
async fn op_comms_send_binary(
    state: Rc<RefCell<OpState>>,
    #[serde] messages: Vec<JsBuffer>,
) -> Result<Vec<Vec<u8>>, anyhow::Error> {
    // Send all messages (old style - broadcast to all)
    for message in messages.iter() {
        let mut data = vec![COMMS_MSG_TYPE_BINARY];
        data.extend(message.as_ref());
        comms_send_single(state.clone(), data, None).await?;
    }

    // Return pending messages
    Ok(recv_binary_internal(state))
}

/// Internal helper to send a single message with optional recipient
async fn comms_send_single(
    state: Rc<RefCell<OpState>>,
    body: Vec<u8>,
    recipient: Option<H160>,
) -> Result<(), anyhow::Error> {
    state
        .borrow_mut()
        .borrow_mut::<Vec<RpcCall>>()
        .push(RpcCall::SendCommsMessage { body, recipient });

    Ok(())
}
