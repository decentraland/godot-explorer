use std::sync::Arc;

use godot::{engine::TlsOptions, prelude::*};

use crate::dcl::components::proto_components::kernel::comms::rfc4;

use super::{
    avatar_scene::AvatarScene,
    wallet::{self, Wallet},
    ws_room::WebSocketRoom,
};

enum Adapter {
    None,
    WsRoom(WebSocketRoom),
}

#[derive(GodotClass)]
#[class(base=Node)]
pub struct Comms {
    #[base]
    base: Base<Node>,
    current_adapter: Adapter,
    tls_client: Option<Gd<TlsOptions>>,
    wallet: Arc<Wallet>,
    last_index: u64,
}

#[godot_api]
impl NodeVirtual for Comms {
    fn init(base: Base<Node>) -> Self {
        Comms {
            base,
            current_adapter: Adapter::None,
            tls_client: None,
            wallet: Arc::new(Wallet::new_local_wallet()),
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
            }
        }
    }
}

impl Comms {}

#[godot_api]
impl Comms {
    fn realm(&self) -> Gd<Node> {
        self.base.get_node("/root/realm".into()).unwrap()
    }

    #[func]
    fn send_position(&mut self, transform: Transform3D) -> bool {
        let get_packet = || {
            let dcl_rotation = transform.basis.to_quat();
            let position_packet = rfc4::Position {
                index: self.last_index as u32,
                position_x: transform.origin.x,
                position_y: transform.origin.y,
                position_z: -transform.origin.z,
                rotation_x: dcl_rotation.x,
                rotation_y: dcl_rotation.y,
                rotation_z: -dcl_rotation.z,
                rotation_w: -dcl_rotation.w,
            };

            rfc4::Packet {
                message: Some(rfc4::packet::Message::Position(position_packet)),
            }
        };

        match &mut self.current_adapter {
            Adapter::None => false,
            Adapter::WsRoom(ws_room) => ws_room.send_rfc4(get_packet(), true),
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
            Adapter::WsRoom(ws_room) => ws_room.send_rfc4(get_packet(), true),
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

        match adapter_protocol {
            "ws-room" => {
                if let Some(ws_url) = fixed_adapter.get(1) {
                    godot_print!("comms > websocket to {}", ws_url);
                    self.current_adapter = Adapter::WsRoom(WebSocketRoom::new(
                        ws_url,
                        self.tls_client.as_ref().unwrap().share(),
                        self.wallet.clone(),
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
