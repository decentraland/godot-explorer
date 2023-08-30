pub trait Adapter {
    fn consume_chats(&mut self) -> Vec<(String, rfc4::Chat)>;
    fn send_rfc4(&mut self, packet: rfc4::Packet, unreliable: bool) -> bool;
    fn send<T>(&mut self, packet: T, only_when_active: bool) -> bool;
    fn change_profile(&mut self, new_profile: Arc<PlayerIdentity>);
}
