use std::sync::Arc;

use godot::{engine::TlsOptions, prelude::*};
use http::Uri;

use crate::{
    avatars::avatar_scene::AvatarScene, comms::signed_login::SignedLoginMeta,
    dcl::components::proto_components::kernel::comms::rfc4,
};

use super::{
    livekit::LivekitRoom,
    player_identity::PlayerIdentity,
    signed_login::{SignedLogin, SignedLoginPollStatus},
    ws_room::WebSocketRoom,
};

#[allow(clippy::large_enum_variant)]
enum Adapter {
    None,
    WsRoom(WebSocketRoom),
    SignedLogin(SignedLogin),
    Livekit(LivekitRoom),
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
        self.base.call_deferred("init_rs".into(), &[]);
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
                    self.base
                        .emit_signal("chat_message".into(), &[chats_variant_array.to_variant()]);
                }
            }
            Adapter::SignedLogin(signed_login) => match signed_login.poll() {
                SignedLoginPollStatus::Pending => {}
                SignedLoginPollStatus::Complete(response) => {
                    self.change_adapter(response.fixed_adapter.unwrap_or("offline".into()));
                }
                SignedLoginPollStatus::Error(e) => {
                    tracing::info!("Error in signed login: {:?}", e);
                    self.current_adapter = Adapter::None;
                }
            },
            Adapter::Livekit(livekit_room) => {
                if livekit_room.poll() {
                    let chats = livekit_room.consume_chats();
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
                        self.base.emit_signal(
                            "chat_message".into(),
                            &[chats_variant_array.to_variant()],
                        );
                    }
                } else {
                    self.current_adapter = Adapter::None;
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

    #[signal]
    fn profile_changed(new_profile: Dictionary) {}

    #[func]
    fn broadcast_voice(&mut self, frame: PackedVector2Array) {
        match &mut self.current_adapter {
            Adapter::Livekit(livekit_room) => {
                let mut max_value = 0;
                let vec = frame
                    .as_slice()
                    .iter()
                    .map(|v| ((0.5 * (v.x + v.y)) * i16::MAX as f32) as i16)
                    .collect::<Vec<i16>>();

                livekit_room.broadcast_voice(vec);
            }
            _ => {}
        };
    }

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

        let message_sent = match &mut self.current_adapter {
            Adapter::None | Adapter::SignedLogin(_) => false,
            Adapter::WsRoom(ws_room) => ws_room.send_rfc4(get_packet(), true),
            Adapter::Livekit(livekit_room) => livekit_room.send_rfc4(get_packet(), true),
        };

        if message_sent {
            self.last_index += 1;
        }
        message_sent
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
            Adapter::None | Adapter::SignedLogin(_) => false,
            Adapter::WsRoom(ws_room) => ws_room.send_rfc4(get_packet(), false),
            Adapter::Livekit(livekit_room) => livekit_room.send_rfc4(get_packet(), false),
        }
    }

    #[func]
    fn init_rs(&mut self) {
        let mut realm = self.realm();
        let on_realm_changed =
            Callable::from_object_method(self.base.clone(), StringName::from("_on_realm_changed"));

        realm.connect("realm_changed".into(), on_realm_changed);

        if self.tls_client.is_none() {
            let tls_client = self
                .base
                .get_node("/root/Global".into())
                .unwrap()
                .call("get_tls_client".into(), &[]);
            let tls_client: Gd<TlsOptions> = Gd::from_variant(&tls_client);
            self.tls_client = Some(tls_client);
        }
    }

    #[func]
    fn _on_realm_changed(&mut self) {
        self.base
            .call_deferred("_on_realm_changed_deferred".into(), &[]);
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
            tracing::info!("invalid comms from realm.");
            return;
        }

        let (comms_protocol, comms_fixed_adapter) = comms.unwrap();
        if comms_protocol != "v3" {
            tracing::info!("Only protocol 'v3' is supported.");
            return;
        }

        if comms_fixed_adapter.is_none() {
            tracing::info!("As far, only fixedAdapter is supported.");
            return;
        }

        let comms_fixed_adapter_str = comms_fixed_adapter.unwrap().to_string();
        self.change_adapter(comms_fixed_adapter_str);
    }

    fn change_adapter(&mut self, comms_fixed_adapter_str: String) {
        let Some((protocol, address)) = comms_fixed_adapter_str.as_str().split_once(':') else {
            tracing::warn!("unrecognised fixed adapter string: {comms_fixed_adapter_str}");
            return;
        };

        let avatar_scene = self
            .base
            .get_node("/root/avatars".into())
            .unwrap()
            .cast::<AvatarScene>();
        self.current_adapter = Adapter::None;

        tracing::info!("change_adapter to protocol {protocol} and address {address}");

        match protocol {
            "ws-room" => {
                self.current_adapter = Adapter::WsRoom(WebSocketRoom::new(
                    address,
                    self.tls_client.as_ref().unwrap().clone(),
                    self.player_identity.clone(),
                    avatar_scene,
                ));
            }
            "signed-login" => {
                let Ok(uri) = Uri::try_from(address.to_string()) else {
                    tracing::warn!("failed to parse signed login address as a uri: {address}");
                    return;
                };

                let realm_url = self.realm().get("realm_url".into()).to_string();
                let Ok(origin) = Uri::try_from(&realm_url) else {
                    tracing::warn!("failed to parse origin address as a uri: {realm_url}");
                    return;
                };

                self.current_adapter = Adapter::SignedLogin(SignedLogin::new(
                    uri,
                    self.player_identity.wallet(),
                    SignedLoginMeta::new(true, origin),
                ));
            }
            "livekit" => {
                self.current_adapter = Adapter::Livekit(LivekitRoom::new(
                    address.to_string(),
                    self.player_identity.clone(),
                    avatar_scene,
                ));
            }
            "offline" => {
                tracing::info!("set offline");
            }
            _ => {
                tracing::info!("unknown adapter {:?}", protocol);
            }
        }
    }

    fn clean(&mut self) {
        match &self.current_adapter {
            Adapter::None | Adapter::SignedLogin(_) => {}
            Adapter::Livekit(_livekit_room) => {
                // livekit_room.clean();
            }
            Adapter::WsRoom(ws_room) => {
                ws_room.clean();
            }
        }

        self.current_adapter = Adapter::None;
    }

    #[func]
    fn update_profile_avatar(&mut self, new_profile: Dictionary) {
        let player_identity = Arc::<PlayerIdentity>::make_mut(&mut self.player_identity);
        player_identity.update_profile_from_dictionary(&new_profile);

        match &mut self.current_adapter {
            Adapter::None | Adapter::SignedLogin(_) => {}
            Adapter::Livekit(_livekit_room) => {
                // livekit_room.change_profile(self.player_identity.clone());
            }
            Adapter::WsRoom(ws_room) => {
                ws_room.change_profile(self.player_identity.clone());
            }
        }

        self.base
            .emit_signal("profile_changed".into(), &[new_profile.to_variant()]);
    }
}
