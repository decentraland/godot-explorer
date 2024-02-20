use ethers::types::H160;

use crate::{comms::profile::UserProfile, dcl::components::proto_components::kernel::comms::rfc4};

pub trait Adapter {
    fn poll(&mut self) -> bool;
    fn clean(&mut self);

    fn consume_chats(&mut self) -> Vec<(H160, rfc4::Chat)>;
    fn consume_scene_messages(&mut self, scene_id: &str) -> Vec<(H160, Vec<u8>)>;
    fn change_profile(&mut self, new_profile: UserProfile);

    fn send_rfc4(&mut self, packet: rfc4::Packet, unreliable: bool) -> bool;

    fn broadcast_voice(&mut self, frame: Vec<i16>);
    fn support_voice_chat(&self) -> bool;
}
