use std::sync::Arc;

use prost::Message;

use crate::dcl::components::proto_components::kernel::comms::rfc4;

use super::player_identity::PlayerIdentity;

// TODO: make this generic works
pub trait Adapter {
    fn consume_chats(&mut self) -> Vec<(String, rfc4::Chat)>;
    fn send_rfc4(&mut self, packet: rfc4::Packet, unreliable: bool) -> bool;
    fn send<T: Message>(&mut self, packet: T, only_when_active: bool) -> bool;
    fn change_profile(&mut self, new_profile: Arc<PlayerIdentity>);
}
