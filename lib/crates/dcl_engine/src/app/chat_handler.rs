//! Turns the room's chat messages into the app's chat events. Messages that start with a control
//! character are protocol, not chat, and never reach the chat.

use godot::classes::{INode, Node};
use godot::prelude::*;

use crate::fetcher::globals;
use crate::godot_classes::dcl_global::DclGlobal;

/// The legacy emote marker, still sent by Bevy releases; dropped so it never shows as text.
const EMOTE: &str = "␐";
/// Ping request and acknowledgement, reserved and not implemented.
const REQUEST_PING: &str = "␑";
const ACK: &str = "␆";

/// Whether a message is a chat line rather than a protocol marker.
pub fn is_chat_text(message: &str) -> bool {
    ![EMOTE, REQUEST_PING, ACK]
        .iter()
        .any(|marker| message.starts_with(marker))
}

#[derive(GodotClass)]
#[class(base=Node, init)]
pub struct ChatHandler {
    base: Base<Node>,
}

#[godot_api]
impl INode for ChatHandler {
    fn ready(&mut self) {
        let me = self.to_gd();
        globals::comms()
            .signals()
            .chat_message()
            .connect_other(&me, |this: &mut Self, chats| this.on_chats_arrived(&chats));
    }
}

impl ChatHandler {
    /// Each entry is `[address, timestamp, message]`.
    fn on_chats_arrived(&mut self, chats: &VarArray) {
        for chat in chats.iter_shared() {
            let Ok(chat) = chat.try_to::<VarArray>() else {
                continue;
            };
            let (Some(address), Some(timestamp), Some(message)) = (
                chat.get(0).and_then(|v| v.try_to::<GString>().ok()),
                chat.get(1).and_then(|v| v.try_to::<f64>().ok()),
                chat.get(2).and_then(|v| v.try_to::<GString>().ok()),
            ) else {
                continue;
            };
            if is_chat_text(&message.to_string()) {
                DclGlobal::singleton().emit_signal(
                    "on_chat_message",
                    &[
                        address.to_variant(),
                        message.to_variant(),
                        timestamp.to_variant(),
                    ],
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::is_chat_text;

    #[test]
    fn protocol_markers_are_not_chat() {
        assert!(is_chat_text("hello"));
        assert!(!is_chat_text("␐wave"));
        assert!(!is_chat_text("␑"));
        assert!(!is_chat_text("␆"));
        assert!(is_chat_text(""));
    }
}
