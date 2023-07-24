use std::sync::Arc;

use godot::{engine::TlsOptions, prelude::*};

use crate::{
    avatars::avatar_scene::AvatarScene, dcl::components::proto_components::kernel::comms::rfc4,
};

use super::{player_identity::PlayerIdentity, ws_room::WebSocketRoom};

#[allow(clippy::large_enum_variant)]
enum Adapter {
    None,
    WsRoom(WebSocketRoom),
}

#[derive(GodotClass)]
#[class(base=Node)]
pub struct CommunicationManager {
    #[base]
    base: Base<Node>,
    current_adapter: Adapter,
    tls_client: Option<Gd<TlsOptions>>,
    player_identity: Arc<PlayerIdentity>,
    last_index: u64,
}

#[godot_api]
impl NodeVirtual for CommunicationManager {
    fn init(base: Base<Node>) -> Self {
        CommunicationManager {
            base,
            current_adapter: Adapter::None,
            tls_client: None,
            player_identity: Arc::new(PlayerIdentity::new()),
            last_index: 0,
        }
    }

    fn ready(&mut self) {
        self.call_deferred("init_rs".into(), &[]);
    }

    fn process(&mut self, _dt: f64) {
        match &mut self.current_adapter {
            Adapter::None => {}
            Adapter::WsRoom(ws_room) => {
                ws_room.poll();
                let chats = ws_room.consume_chats();
                if !chats.is_empty() {
                    let mut chats_variant_array = VariantArray::new();
                    for (addr, chat) in chats {
                        let mut chat_arr = VariantArray::new();
                        // TODO: change to the name?
                        chat_arr.push(addr.to_string().to_variant());
                        chat_arr.push(chat.timestamp.to_variant());
                        chat_arr.push(chat.message.to_variant());

                        chats_variant_array.push(chat_arr.to_variant());
                    }
                    self.emit_signal("chat_message".into(), &[chats_variant_array.to_variant()]);
                }
            }
        }
    }
}

impl CommunicationManager {}

#[godot_api]
impl CommunicationManager {
    fn realm(&self) -> Gd<Node> {
        self.base.get_node("/root/realm".into()).unwrap()
    }

    #[signal]
    fn chat_message(chats: VariantArray) {}

    #[func]
    fn broadcast_position_and_rotation(&mut self, position: Vector3, rotation: Quaternion) -> bool {
        let index = self.last_index;
        let get_packet = || {
            let position_packet = rfc4::Position {
                index: index as u32,
                position_x: position.x,
                position_y: position.y,
                position_z: -position.z,
                rotation_x: rotation.x,
                rotation_y: rotation.y,
                rotation_z: -rotation.z,
                rotation_w: -rotation.w,
            };

            rfc4::Packet {
                message: Some(rfc4::packet::Message::Position(position_packet)),
            }
        };

        match &mut self.current_adapter {
            Adapter::None => false,
            Adapter::WsRoom(ws_room) => {
                self.last_index += 1;
                ws_room.send_rfc4(get_packet(), true)
            }
        }
    }

    #[func]
    fn send_chat(&mut self, text: GodotString) -> bool {
        let get_packet = || rfc4::Packet {
            message: Some(rfc4::packet::Message::Chat(rfc4::Chat {
                message: text.to_string(),
                timestamp: 0.0,
            })),
        };

        match &mut self.current_adapter {
            Adapter::None => false,
            Adapter::WsRoom(ws_room) => ws_room.send_rfc4(get_packet(), false),
        }
    }

    #[func]
    fn init_rs(&mut self) {
        let mut realm = self.realm();
        let on_realm_changed =
            Callable::from_object_method(self.base.share(), StringName::from("_on_realm_changed"));

        realm.connect("realm_changed".into(), on_realm_changed);

        if self.tls_client.is_none() {
            let tls_client = self
                .get_node("/root/Global".into())
                .unwrap()
                .call("get_tls_client".into(), &[]);
            let tls_client: Gd<TlsOptions> = Gd::from_variant(&tls_client);
            self.tls_client = Some(tls_client);
        }
    }

    #[func]
    fn _on_realm_changed(&mut self) {
        self.call_deferred("_on_realm_changed_deferred".into(), &[]);
    }

    fn _internal_get_comms_from_real(&self) -> Option<(String, Option<GodotString>)> {
        let realm = self.realm();
        let realm_about = Dictionary::from_variant(&realm.get("realm_about".into()));
        let comms = Dictionary::from_variant(&realm_about.get(StringName::from("comms"))?);
        let comms_protocol = String::from_variant(&comms.get(StringName::from("protocol"))?);
        let comms_fixed_adapter = comms
            .get(StringName::from("fixedAdapter"))
            .map(|v| GodotString::from_variant(&v));

        Some((comms_protocol, comms_fixed_adapter))
    }

    #[func]
    fn _on_realm_changed_deferred(&mut self) {
        self.clean();

        let comms = self._internal_get_comms_from_real();
        if comms.is_none() {
            godot_print!("comms > invalid comms from realm.");
            return;
        }

        let (comms_protocol, comms_fixed_adapter) = comms.unwrap();
        if comms_protocol != "v3" {
            godot_print!("comms > Only protocol 'v3' is supported.");
            return;
        }

        if comms_fixed_adapter.is_none() {
            godot_print!("comms > As far, only fixedAdapter is supported.");
            return;
        }

        let comms_fixed_adapter_str = comms_fixed_adapter.unwrap().to_string();
        let fixed_adapter: Vec<&str> = comms_fixed_adapter_str.splitn(2, ':').collect();
        let adapter_protocol = *fixed_adapter.first().unwrap();

        let avatar_scene = self
            .get_node("/root/avatars".into())
            .unwrap()
            .cast::<AvatarScene>();

        self.current_adapter = Adapter::None;

        match adapter_protocol {
            "ws-room" => {
                if let Some(ws_url) = fixed_adapter.get(1) {
                    godot_print!("comms > websocket to {}", ws_url);
                    self.current_adapter = Adapter::WsRoom(WebSocketRoom::new(
                        ws_url,
                        self.tls_client.as_ref().unwrap().share(),
                        self.player_identity.clone(),
                        avatar_scene,
                    ));
                }
            }
            "offline" => {
                godot_print!("comms > set offline");
            }
            _ => {
                godot_print!("comms > unknown adapter {:?}", adapter_protocol);
            }
        }
    }

    fn clean(&mut self) {
        match &self.current_adapter {
            Adapter::None => {}
            Adapter::WsRoom(ws_room) => {
                ws_room.clean();
            }
        }

        self.current_adapter = Adapter::None;
    }
}
