use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    avatars::avatar_scene::AvatarScene,
    comms::wallet::{self, AsH160},
    dcl::components::proto_components::kernel::comms::{
        rfc4::{self},
        rfc5::{ws_packet, WsIdentification, WsPacket, WsPeerUpdate, WsSignedChallenge},
    },
};
use ethers::types::{Signature, H160};
use godot::{
    engine::{TlsOptions, WebSocketPeer},
    prelude::*,
};
use prost::Message;
use tracing::error;

use super::{
    player_identity::PlayerIdentity,
    profile::{SerializedProfile, UserProfile},
};

#[derive(Clone)]
enum WsRoomState {
    Connecting,
    Connected,
    IdentMessageSent,
    ChallengeMessageSent,
    WelcomeMessageReceived,
}

struct Peer {
    address: H160,
    profile: Option<UserProfile>,
    announced_version: Option<u32>,
}

impl Peer {
    pub fn new(address: H160) -> Self {
        Self {
            address,
            profile: None,
            announced_version: None,
        }
    }
}

pub struct WebSocketRoom {
    state: WsRoomState,

    // Connection
    ws_url: GodotString,
    last_try_to_connect: Instant,
    ws_peer: Gd<WebSocketPeer>,
    tls_client: Gd<TlsOptions>,
    signature: Option<Signature>,

    // Self alias
    from_alias: u32,
    player_identity: Arc<PlayerIdentity>,
    peer_identities: HashMap<u32, Peer>,

    // Trade-off with other peers
    avatars: Gd<AvatarScene>,
    chats: Vec<(String, rfc4::Chat)>,
    last_profile_response_sent: Instant,
    last_profile_request_sent: Instant,
    last_profile_version_announced: u32,
}

impl WebSocketRoom {
    pub fn new(
        ws_url: &str,
        tls_client: Gd<TlsOptions>,
        player_identity: Arc<PlayerIdentity>,
        avatars: Gd<AvatarScene>,
    ) -> Self {
        let lower_url = ws_url.to_lowercase();
        let ws_url = if !lower_url.starts_with("ws://") && !lower_url.starts_with("wss://") {
            if !lower_url.starts_with("http://") && !lower_url.starts_with("https://") {
                format!("wss://{}", ws_url)
            } else {
                ws_url.to_string()
            }
        } else {
            ws_url.to_string()
        };

        let old_time = Instant::now() - Duration::from_secs(1000);

        Self {
            ws_peer: WebSocketPeer::new(),
            ws_url: GodotString::from(ws_url),
            state: WsRoomState::Connecting,
            tls_client,
            player_identity,
            from_alias: 0,
            peer_identities: HashMap::new(),
            avatars,
            chats: Vec::new(),
            signature: None,
            last_profile_response_sent: old_time,
            last_profile_request_sent: old_time,
            last_try_to_connect: old_time,
            last_profile_version_announced: 0,
        }
    }

    pub fn consume_chats(&mut self) -> Vec<(String, rfc4::Chat)> {
        std::mem::take(&mut self.chats)
    }

    pub fn send_rfc4(&mut self, packet: rfc4::Packet, unreliable: bool) -> bool {
        let mut buf = Vec::new();
        packet.encode(&mut buf).unwrap();

        let packet = WsPacket {
            message: Some(ws_packet::Message::PeerUpdateMessage(WsPeerUpdate {
                from_alias: self.from_alias,
                body: buf,
                unreliable,
            })),
        };
        self.send(packet, true)
    }

    pub fn send<T>(&mut self, packet: T, only_when_active: bool) -> bool
    where
        T: Message,
    {
        if let WsRoomState::Connecting = self.state {
            return false;
        }

        let should_send = if only_when_active {
            matches!(self.state, WsRoomState::WelcomeMessageReceived)
        } else {
            true
        };

        if !should_send {
            return false;
        }

        let mut buf = Vec::new();
        if packet.encode(&mut buf).is_err() {
            return false;
        }

        let buf = PackedByteArray::from_iter(buf);
        matches!(self.ws_peer.send(buf), godot::engine::global::Error::OK)
    }

    pub fn poll(&mut self) {
        let mut peer = self.ws_peer.clone();
        peer.poll();

        let ws_state = peer.get_ready_state();

        match self.state.clone() {
            WsRoomState::Connecting => match ws_state {
                godot::engine::web_socket_peer::State::STATE_CLOSED => {
                    if (Instant::now() - self.last_try_to_connect).as_secs() > 1 {
                        // TODO: see if the tls client is really required for now
                        let _tls_client = self.tls_client.clone();

                        let ws_protocols = {
                            let mut v = PackedStringArray::new();
                            v.push(GodotString::from("rfc5"));
                            v
                        };

                        peer.set("supported_protocols".into(), ws_protocols.to_variant());
                        peer.call("connect_to_url".into(), &[self.ws_url.clone().to_variant()]);

                        self.last_try_to_connect = Instant::now();
                        self.peer_identities.clear();
                        self.from_alias = 0;
                        self.signature = None;
                    }
                }
                godot::engine::web_socket_peer::State::STATE_OPEN => {
                    self.state = WsRoomState::Connected;
                }
                _ => {}
            },
            WsRoomState::Connected => match ws_state {
                godot::engine::web_socket_peer::State::STATE_OPEN => {
                    self.send(
                        WsPacket {
                            message: Some(ws_packet::Message::PeerIdentification(
                                WsIdentification {
                                    address: format!(
                                        "{:#x}",
                                        self.player_identity.wallet().address()
                                    ),
                                },
                            )),
                        },
                        false,
                    );

                    self.state = WsRoomState::IdentMessageSent;
                }
                _ => {
                    self.state = WsRoomState::Connecting;
                }
            },
            WsRoomState::IdentMessageSent => match ws_state {
                godot::engine::web_socket_peer::State::STATE_OPEN => {
                    while let Some((packet_length, message)) = get_next_packet(peer.clone()) {
                        match message {
                            ws_packet::Message::ChallengeMessage(challenge_msg) => {
                                tracing::info!("comms > peer msg {:?}", challenge_msg);

                                let challenge_to_sign = challenge_msg.challenge_to_sign.clone();

                                // TODO: this should be async, now it's a local wallet and it's blocking
                                let sign = futures_lite::future::block_on(
                                    self.player_identity
                                        .wallet()
                                        .sign_message(challenge_to_sign.as_bytes()),
                                );

                                if let Ok(sign) = sign {
                                    self.signature = Some(sign);

                                    let chain = wallet::SimpleAuthChain::new(
                                        self.player_identity.wallet().address(),
                                        challenge_to_sign.clone(),
                                        *self.signature.as_ref().unwrap(),
                                    );
                                    let auth_chain_json = serde_json::to_string(&chain).unwrap();

                                    self.send(
                                        WsPacket {
                                            message: Some(
                                                ws_packet::Message::SignedChallengeForServer(
                                                    WsSignedChallenge { auth_chain_json },
                                                ),
                                            ),
                                        },
                                        false,
                                    );

                                    self.state = WsRoomState::ChallengeMessageSent;
                                } else {
                                    peer.close();
                                    self.state = WsRoomState::Connecting;
                                }
                            }
                            _ => {
                                tracing::info!(
                                    "comms > received unknown message {} bytes",
                                    packet_length
                                );
                            }
                        }
                    }
                }
                _ => {
                    self.state = WsRoomState::Connecting;
                }
            },
            WsRoomState::ChallengeMessageSent => match ws_state {
                godot::engine::web_socket_peer::State::STATE_OPEN => {
                    while let Some((packet_length, message)) = get_next_packet(peer.clone()) {
                        match message {
                            ws_packet::Message::WelcomeMessage(welcome_msg) => {
                                // welcome_msg.
                                self.state = WsRoomState::WelcomeMessageReceived;
                                self.from_alias = welcome_msg.alias;
                                self.peer_identities = HashMap::from_iter(
                                    welcome_msg.peer_identities.into_iter().flat_map(
                                        |(alias, address)| {
                                            address.as_h160().map(|h160| (alias, Peer::new(h160)))
                                        },
                                    ),
                                );

                                self.send_rfc4(
                                    rfc4::Packet {
                                        message: Some(rfc4::packet::Message::ProfileResponse(
                                            rfc4::ProfileResponse {
                                                serialized_profile: serde_json::to_string(
                                                    &self.player_identity.profile().content,
                                                )
                                                .unwrap(),
                                                base_url: self
                                                    .player_identity
                                                    .profile()
                                                    .base_url
                                                    .clone(),
                                            },
                                        )),
                                    },
                                    false,
                                );

                                self.last_profile_version_announced =
                                    self.player_identity.profile().version;

                                self.send_rfc4(
                                    rfc4::Packet {
                                        message: Some(rfc4::packet::Message::ProfileVersion(
                                            rfc4::AnnounceProfileVersion {
                                                profile_version: self
                                                    .last_profile_version_announced,
                                            },
                                        )),
                                    },
                                    false,
                                );

                                self.avatars.bind_mut().clean();
                                for (alias, _) in self.peer_identities.iter() {
                                    self.avatars.bind_mut().add_avatar(*alias);
                                }
                            }
                            _ => {
                                tracing::info!(
                                    "comms > received unknown message {} bytes",
                                    packet_length
                                );
                            }
                        }
                    }
                }
                _ => {
                    self.state = WsRoomState::Connecting;
                }
            },
            WsRoomState::WelcomeMessageReceived => match ws_state {
                godot::engine::web_socket_peer::State::STATE_OPEN => {
                    self._handle_messages();
                }
                _ => {
                    self.state = WsRoomState::Connecting;
                }
            },
        }
    }

    pub fn clean(&self) {
        let mut peer = self.ws_peer.clone();
        peer.close();
        match peer.get_ready_state() {
            godot::engine::web_socket_peer::State::STATE_OPEN
            | godot::engine::web_socket_peer::State::STATE_CONNECTING => {
                peer.close();
            }
            _ => {}
        }
    }

    fn _handle_messages(&mut self) {
        while let Some((_packet_length, message)) = get_next_packet(self.ws_peer.clone()) {
            match message {
                ws_packet::Message::ChallengeMessage(_)
                | ws_packet::Message::PeerIdentification(_)
                | ws_packet::Message::SignedChallengeForServer(_)
                | ws_packet::Message::WelcomeMessage(_) => {
                    // TODO: invalid message when it's already connected
                }
                ws_packet::Message::PeerJoinMessage(peer) => {
                    if let Some(h160) = peer.address.as_h160() {
                        self.peer_identities.insert(peer.alias, Peer::new(h160));
                        self.avatars.bind_mut().add_avatar(peer.alias);
                        // TODO: message XXX joined
                    } else {
                        // TODO: Invalid address
                    }
                }
                ws_packet::Message::PeerLeaveMessage(peer) => {
                    self.peer_identities.remove(&peer.alias);
                    self.avatars.bind_mut().remove_avatar(peer.alias);
                    // TODO: message XXX left
                }
                ws_packet::Message::PeerUpdateMessage(update) => {
                    let packet = match rfc4::Packet::decode(update.body.as_slice()) {
                        Ok(packet) => packet,
                        Err(_e) => {
                            error!("comms > invalid data packet {:?}", update);
                            continue;
                        }
                    };
                    let Some(message) = packet.message else {
                        error!("comms > empty data packet {:?}", update);
                        continue;
                    };

                    let Some(peer) = self.peer_identities.get(&update.from_alias) else {
                        error!("comms > peer not found {:?}", update);
                        continue;
                    };

                    match message {
                        rfc4::packet::Message::Position(position) => {
                            self.avatars
                                .bind_mut()
                                .update_transform(update.from_alias, &position);
                        }
                        rfc4::packet::Message::Chat(chat) => {
                            let peer_name = {
                                if let Some(profile) = peer.profile.as_ref() {
                                    profile.content.name.clone()
                                } else {
                                    peer.address.to_string()
                                }
                            };
                            self.chats.push((peer_name, chat));
                        }
                        rfc4::packet::Message::ProfileVersion(announce_profile_version) => {
                            self.peer_identities
                                .get_mut(&update.from_alias)
                                .unwrap()
                                .announced_version = Some(
                                announce_profile_version
                                    .profile_version
                                    .max(peer.announced_version.unwrap_or(0)),
                            );
                        }
                        rfc4::packet::Message::ProfileRequest(profile_request) => {
                            if self.last_profile_response_sent.elapsed().as_secs_f32() < 10.0 {
                                continue;
                            }

                            tracing::info!("comms > received ProfileRequest {:?}", profile_request);

                            if let Some(addr) = profile_request.address.as_h160() {
                                if addr == self.player_identity.wallet().address() {
                                    self.last_profile_response_sent = Instant::now();

                                    self.send_rfc4(
                                        rfc4::Packet {
                                            message: Some(rfc4::packet::Message::ProfileResponse(
                                                rfc4::ProfileResponse {
                                                    serialized_profile: serde_json::to_string(
                                                        &self.player_identity.profile().content,
                                                    )
                                                    .unwrap(),
                                                    base_url: self
                                                        .player_identity
                                                        .profile()
                                                        .base_url
                                                        .clone(),
                                                },
                                            )),
                                        },
                                        false,
                                    );
                                }
                            }
                        }
                        rfc4::packet::Message::ProfileResponse(profile_response) => {
                            let serialized_profile: SerializedProfile =
                                match serde_json::from_str(&profile_response.serialized_profile) {
                                    Ok(p) => p,
                                    Err(_e) => {
                                        error!(
                                            "comms > invalid data ProfileResponse {:?}",
                                            profile_response
                                        );
                                        continue;
                                    }
                                };

                            let incoming_version = serialized_profile.version as u32;
                            let current_version = if let Some(profile) = peer.profile.as_ref() {
                                profile.version
                            } else {
                                0
                            };

                            if incoming_version < current_version {
                                error!(
                                    "comms > old version ProfileResponse {:?}",
                                    profile_response
                                );
                                continue;
                            }

                            self.avatars.bind_mut().update_avatar(
                                update.from_alias,
                                &serialized_profile,
                                &profile_response.base_url,
                            );

                            self.peer_identities
                                .get_mut(&update.from_alias)
                                .unwrap()
                                .profile = Some(UserProfile {
                                version: incoming_version,
                                content: serialized_profile,
                                base_url: profile_response.base_url,
                            });
                        }
                        rfc4::packet::Message::Scene(_scene) => {}
                        rfc4::packet::Message::Voice(_voice) => {}
                    }
                }
                ws_packet::Message::PeerKicked(reason) => {
                    tracing::info!("comms > received PeerKicked {:?}", reason);
                    // TODO: message announcing the kick
                    self.ws_peer.close();
                    self.state = WsRoomState::Connecting;
                }
            }
        }

        if self.last_profile_request_sent.elapsed().as_secs_f32() > 10.0 {
            self.last_profile_request_sent = Instant::now();

            let to_request = self
                .peer_identities
                .iter()
                .filter_map(|(_, peer)| {
                    if peer.profile.is_some() {
                        let announced_version = peer.announced_version.unwrap_or(0);
                        let current_version = peer.profile.as_ref().unwrap().version;

                        if announced_version > current_version {
                            None
                        } else {
                            Some((peer.address, announced_version))
                        }
                    } else {
                        Some((peer.address, peer.announced_version.unwrap_or(0)))
                    }
                })
                .collect::<Vec<(H160, u32)>>();

            for (address, profile_version) in to_request {
                self.send_rfc4(
                    rfc4::Packet {
                        message: Some(rfc4::packet::Message::ProfileRequest(
                            rfc4::ProfileRequest {
                                address: format!("{:#x}", address),
                                profile_version,
                            },
                        )),
                    },
                    true,
                );
            }
        }

        if self.last_profile_version_announced != self.player_identity.profile().version {
            self.last_profile_version_announced = self.player_identity.profile().version;
            self.send_rfc4(
                rfc4::Packet {
                    message: Some(rfc4::packet::Message::ProfileVersion(
                        rfc4::AnnounceProfileVersion {
                            profile_version: self.last_profile_version_announced,
                        },
                    )),
                },
                false,
            );
        }
    }

    pub fn change_profile(&mut self, new_profile: Arc<PlayerIdentity>) {
        self.player_identity = new_profile;
    }
}

fn get_next_packet(mut peer: Gd<WebSocketPeer>) -> Option<(usize, ws_packet::Message)> {
    if peer.get_available_packet_count() > 0 {
        let packet = peer.get_packet();
        let packet_length = packet.len();
        let packet = WsPacket::decode(packet.as_slice());
        if let Ok(packet) = packet {
            packet.message.as_ref()?;
            return Some((packet_length, packet.message.unwrap()));
        }
    }
    None
}
