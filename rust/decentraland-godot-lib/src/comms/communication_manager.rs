use ethers_core::types::H160;
use godot::prelude::*;
use http::Uri;

use crate::{
    comms::{adapter::ws_room::WebSocketRoom, signed_login::SignedLoginMeta},
    dcl::components::proto_components::kernel::comms::rfc4,
    godot_classes::dcl_global::DclGlobal,
};

use super::{
    adapter::adapter_trait::Adapter,
    signed_login::{SignedLogin, SignedLoginPollStatus},
};

#[cfg(feature = "use_livekit")]
use crate::comms::adapter::{archipelago::ArchipelagoManager, livekit::LivekitRoom};

#[allow(clippy::large_enum_variant)]
enum CommsConnection {
    None,
    WaitingForIdentity(String),
    SignedLogin(SignedLogin),
    #[cfg(feature = "use_livekit")]
    Archipelago(ArchipelagoManager),
    Connected(Box<dyn Adapter>),
}

#[derive(GodotClass)]
#[class(base=Node)]
pub struct CommunicationManager {
    current_connection: CommsConnection,
    current_connection_str: String,
    last_position_broadcast_index: u64,
    voice_chat_enabled: bool,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for CommunicationManager {
    fn init(base: Base<Node>) -> Self {
        CommunicationManager {
            current_connection: CommsConnection::None,
            current_connection_str: String::default(),
            last_position_broadcast_index: 0,
            voice_chat_enabled: false,
            base,
        }
    }

    fn ready(&mut self) {
        self.base.call_deferred("init_rs".into(), &[]);
    }

    fn process(&mut self, _dt: f64) {
        match &mut self.current_connection {
            CommsConnection::None => {}
            CommsConnection::WaitingForIdentity(adapter_url) => {
                let player_identity = DclGlobal::singleton().bind().get_player_identity();

                if player_identity.bind().try_get_address().is_some() {
                    self.base
                        .call_deferred("change_adapter".into(), &[adapter_url.to_variant()]);
                }
            }
            CommsConnection::SignedLogin(signed_login) => match signed_login.poll() {
                SignedLoginPollStatus::Pending => {}
                SignedLoginPollStatus::Complete(response) => {
                    self.change_adapter(response.fixed_adapter.unwrap_or("offline".into()).into());
                }
                SignedLoginPollStatus::Error(e) => {
                    tracing::info!("Error in signed login: {:?}", e);
                    self.current_connection = CommsConnection::None;
                }
            },
            #[cfg(feature = "use_livekit")]
            CommsConnection::Archipelago(archipelago) => {
                archipelago.poll();
                if let Some(adapter) = archipelago.adapter_as_mut() {
                    let adapter = adapter.as_mut();
                    let adapter_polling_ok = adapter.poll();
                    let chats = adapter.consume_chats();

                    if !chats.is_empty() {
                        let chats_variant_array = get_chat_array(chats);
                        self.base.emit_signal(
                            "chat_message".into(),
                            &[chats_variant_array.to_variant()],
                        );
                    }

                    if !adapter_polling_ok {
                        self.current_connection = CommsConnection::None;
                    }
                }
            }
            CommsConnection::Connected(adapter) => {
                let adapter = adapter.as_mut();
                let adapter_polling_ok = adapter.poll();
                let chats = adapter.consume_chats();

                if !chats.is_empty() {
                    let chats_variant_array = get_chat_array(chats);
                    self.base
                        .emit_signal("chat_message".into(), &[chats_variant_array.to_variant()]);
                }

                if !adapter_polling_ok {
                    self.current_connection = CommsConnection::None;
                }
            }
        }
    }
}

impl CommunicationManager {
    pub fn send_scene_message(&mut self, scene_id: String, data: Vec<u8>) {
        let scene_message = rfc4::Packet {
            message: Some(rfc4::packet::Message::Scene(rfc4::Scene { scene_id, data })),
        };
        match &mut self.current_connection {
            CommsConnection::Connected(adapter) => {
                adapter.send_rfc4(scene_message, true);
            }
            #[cfg(feature = "use_livekit")]
            CommsConnection::Archipelago(archipelago) => {
                if let Some(adapter) = archipelago.adapter_as_mut() {
                    adapter.send_rfc4(scene_message, true);
                }
            }
            _ => {}
        }
    }

    pub fn get_pending_messages(&mut self, scene_id: &str) -> Vec<(H160, Vec<u8>)> {
        match &mut self.current_connection {
            CommsConnection::Connected(adapter) => adapter.consume_scene_messages(scene_id),
            #[cfg(feature = "use_livekit")]
            CommsConnection::Archipelago(archipelago) => {
                if let Some(adapter) = archipelago.adapter_as_mut() {
                    adapter.consume_scene_messages(scene_id)
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    }
}

#[godot_api]
impl CommunicationManager {
    #[signal]
    fn chat_message(chats: VariantArray) {}

    #[signal]
    fn on_adapter_changed(voice_chat_enabled: bool, new_adapter: GString) {}

    #[func]
    fn broadcast_voice(&mut self, frame: PackedVector2Array) {
        let adapter = match &mut self.current_connection {
            CommsConnection::Connected(adapter) => adapter,
            #[cfg(feature = "use_livekit")]
            CommsConnection::Archipelago(archipelago) => {
                let Some(adapter) = archipelago.adapter_as_mut() else {
                    return;
                };

                adapter
            }
            _ => {
                return;
            }
        };
        if !adapter.support_voice_chat() {
            return;
        }

        let mut max_value = 0;
        let vec = frame
            .as_slice()
            .iter()
            .map(|v| {
                let value = ((0.5 * (v.x + v.y)) * i16::MAX as f32) as i16;

                max_value = std::cmp::max(max_value, value);
                value
            })
            .collect::<Vec<i16>>();

        if max_value > 0 {
            adapter.broadcast_voice(vec);
        }
    }

    #[func]
    fn is_voice_chat_enabled(&self) -> bool {
        self.voice_chat_enabled
    }

    #[func]
    fn broadcast_position_and_rotation(&mut self, position: Vector3, rotation: Quaternion) -> bool {
        let index = self.last_position_broadcast_index;
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

        let message_sent = match &mut self.current_connection {
            CommsConnection::None
            | CommsConnection::SignedLogin(_)
            | CommsConnection::WaitingForIdentity(_) => false,
            CommsConnection::Connected(adapter) => adapter.send_rfc4(get_packet(), true),
            #[cfg(feature = "use_livekit")]
            CommsConnection::Archipelago(archipelago) => {
                archipelago.update_position(position);
                if let Some(adapter) = archipelago.adapter_as_mut() {
                    adapter.send_rfc4(get_packet(), true)
                } else {
                    false
                }
            }
        };

        if message_sent {
            self.last_position_broadcast_index += 1;
        }
        message_sent
    }

    #[func]
    fn send_chat(&mut self, text: GString) -> bool {
        let get_packet = || rfc4::Packet {
            message: Some(rfc4::packet::Message::Chat(rfc4::Chat {
                message: text.to_string(),
                timestamp: 0.0,
            })),
        };

        match &mut self.current_connection {
            CommsConnection::None
            | CommsConnection::SignedLogin(_)
            | CommsConnection::WaitingForIdentity(_) => false,
            CommsConnection::Connected(adapter) => adapter.send_rfc4(get_packet(), false),
            #[cfg(feature = "use_livekit")]
            CommsConnection::Archipelago(archipelago) => {
                if let Some(adapter) = archipelago.adapter_as_mut() {
                    adapter.send_rfc4(get_packet(), false)
                } else {
                    false
                }
            }
        }
    }

    #[func]
    fn init_rs(&mut self) {
        DclGlobal::singleton().bind().get_realm().connect(
            "realm_changed".into(),
            self.base.callable("_on_realm_changed"),
        );

        let mut player_identity = DclGlobal::singleton().bind().get_player_identity();
        player_identity.connect(
            "profile_changed".into(),
            self.base.callable("_on_profile_changed"),
        );
    }

    #[func]
    fn _on_profile_changed(&mut self) {
        self.base.call_deferred("_on_update_profile".into(), &[]);
    }

    #[func]
    fn _on_realm_changed(&mut self) {
        self.base
            .call_deferred("_on_realm_changed_deferred".into(), &[]);
    }

    fn _internal_get_comms_from_realm(&self) -> Option<(String, Option<GString>)> {
        let realm = DclGlobal::singleton().bind().get_realm();
        let realm_about = Dictionary::from_variant(&realm.get("realm_about".into()));
        let comms = Dictionary::from_variant(&realm_about.get(StringName::from("comms"))?);
        let comms_protocol = String::from_variant(&comms.get(StringName::from("protocol"))?);

        let comms_fixed_adapter = if comms.contains_key("fixedAdapter") {
            comms
                .get(StringName::from("fixedAdapter"))
                .map(|v| GString::from_variant(&v))
        } else if comms.contains_key("adapter") {
            if let Some(temp) = comms
                .get(StringName::from("adapter"))
                .map(|v| GString::from_variant(&v).to_string())
            {
                if temp.starts_with("fixed-adapter:") {
                    Some(temp.replace("fixed-adapter:", "").into())
                } else if temp.starts_with("archipelago:") {
                    Some(temp.to_string()[12..].into())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        Some((comms_protocol, comms_fixed_adapter))
    }

    #[func]
    fn _on_realm_changed_deferred(&mut self) {
        self.clean();

        let comms = self._internal_get_comms_from_realm();
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
        self.change_adapter(comms_fixed_adapter_str.into());
    }

    #[func]
    fn change_adapter(&mut self, comms_fixed_adapter_gstr: GString) {
        let comms_fixed_adapter_str = comms_fixed_adapter_gstr.to_string();
        let Some((protocol, comms_address)) = comms_fixed_adapter_str.as_str().split_once(':')
        else {
            tracing::warn!("unrecognised fixed adapter string: {comms_fixed_adapter_str}");
            return;
        };

        let player_identity = DclGlobal::singleton().bind().get_player_identity();

        if player_identity.bind().try_get_address().is_none() {
            self.current_connection = CommsConnection::WaitingForIdentity(comms_fixed_adapter_str);
            return;
        }

        self.current_connection = CommsConnection::None;
        self.current_connection_str = comms_fixed_adapter_str.clone();
        let avatar_scene = DclGlobal::singleton().bind().get_avatars();

        tracing::info!("change_adapter to protocol {protocol} and address {comms_address}");

        let current_ephemeral_auth_chain = player_identity
            .bind()
            .try_get_ephemeral_auth_chain()
            .expect("ephemeral auth chain needed to start a comms connection");

        let player_profile = player_identity.bind().clone_profile();

        match protocol {
            "ws-room" => {
                self.current_connection = CommsConnection::Connected(Box::new(WebSocketRoom::new(
                    comms_address,
                    current_ephemeral_auth_chain,
                    player_profile,
                    avatar_scene,
                )));
            }
            "signed-login" => {
                let Ok(uri) = Uri::try_from(comms_address.to_string()) else {
                    tracing::warn!(
                        "failed to parse signed login comms_address as a uri: {comms_address}"
                    );
                    return;
                };

                let realm_url = DclGlobal::singleton()
                    .bind()
                    .get_realm()
                    .get("realm_url".into())
                    .to_string();
                let Ok(origin) = Uri::try_from(&realm_url) else {
                    tracing::warn!("failed to parse origin comms_address as a uri: {realm_url}");
                    return;
                };

                self.current_connection = CommsConnection::SignedLogin(SignedLogin::new(
                    uri,
                    current_ephemeral_auth_chain,
                    SignedLoginMeta::new(true, origin),
                ));
            }

            #[cfg(feature = "use_livekit")]
            "livekit" => {
                self.current_connection = CommsConnection::Connected(Box::new(LivekitRoom::new(
                    comms_address.to_string(),
                    current_ephemeral_auth_chain.signer(),
                    player_profile,
                    avatar_scene,
                )));
            }

            #[cfg(not(feature = "use_livekit"))]
            "livekit" => {
                tracing::error!("livekit wasn't included in this build");
            }

            "offline" => {
                tracing::info!("set offline");
            }
            #[cfg(feature = "use_livekit")]
            "archipelago" => {
                self.current_connection = CommsConnection::Archipelago(ArchipelagoManager::new(
                    comms_address,
                    current_ephemeral_auth_chain.clone(),
                    player_profile,
                    avatar_scene,
                ));
            }
            _ => {
                tracing::info!("unknown adapter {:?}", protocol);
            }
        }

        self.voice_chat_enabled = match &self.current_connection {
            CommsConnection::Connected(adapter) => adapter.support_voice_chat(),
            #[cfg(feature = "use_livekit")]
            CommsConnection::Archipelago(archipelago) => {
                if let Some(adapter) = archipelago.adapter() {
                    adapter.support_voice_chat()
                } else {
                    true
                }
            }
            _ => false,
        };

        self.base.emit_signal(
            "on_adapter_changed".into(),
            &[
                self.voice_chat_enabled.to_variant(),
                comms_fixed_adapter_gstr.to_variant(),
            ],
        );
    }

    fn clean(&mut self) {
        match &mut self.current_connection {
            CommsConnection::None
            | CommsConnection::SignedLogin(_)
            | CommsConnection::WaitingForIdentity(_) => {}
            CommsConnection::Connected(adapter) => {
                adapter.clean();
            }
            #[cfg(feature = "use_livekit")]
            CommsConnection::Archipelago(archipelago) => archipelago.clean(),
        }

        self.current_connection = CommsConnection::None;
        self.current_connection_str = String::default();
    }

    #[func]
    fn _on_update_profile(&mut self) {
        let dcl_player_identity = DclGlobal::singleton().bind().get_player_identity();
        let player_identity = dcl_player_identity.bind();
        let Some(player_profile) = player_identity.clone_profile() else {
            return;
        };
        match &mut self.current_connection {
            CommsConnection::Connected(adapter) => adapter.change_profile(player_profile),
            #[cfg(feature = "use_livekit")]
            CommsConnection::Archipelago(archipelago) => archipelago.change_profile(player_profile),
            _ => {}
        }
    }

    #[func]
    fn disconnect(&mut self, sign_out_session: bool) {
        self.clean();
        if sign_out_session {
            let mut player_identity = DclGlobal::singleton().bind().get_player_identity();
            player_identity.bind_mut().logout();
        }
    }

    #[func]
    pub fn get_current_adapter_conn_str(&self) -> GString {
        GString::from(self.current_connection_str.clone())
    }
}

fn get_chat_array(chats: Vec<(H160, rfc4::Chat)>) -> VariantArray {
    let mut chats_variant_array = VariantArray::new();
    for (address, chat) in chats {
        let mut chat_arr = VariantArray::new();
        let address = format!("{:#x}", address);
        chat_arr.push(address.to_variant());
        chat_arr.push(chat.timestamp.to_variant());
        chat_arr.push(chat.message.to_variant());

        chats_variant_array.push(chat_arr.to_variant());
    }
    chats_variant_array
}
