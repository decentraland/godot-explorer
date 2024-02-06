use std::time::{Duration, Instant};

use crate::{
    auth::ephemeral_auth_chain::EphemeralAuthChain,
    avatars::avatar_scene::AvatarScene,
    comms::profile::UserProfile,
    dcl::components::proto_components::{
        common::Position,
        kernel::comms::v3::{
            client_packet, server_packet, ChallengeRequestMessage, ClientPacket, Heartbeat,
            ServerPacket, SignedChallengeMessage,
        },
    },
};
use ethers::types::H160;
use godot::{engine::WebSocketPeer, prelude::*};
use prost::Message;

use super::{adapter_trait::Adapter, livekit::LivekitRoom};

#[derive(Clone)]
enum ArchipelagoState {
    Connecting,
    Connected,
    IdentMessageSent,
    ChallengeMessageSent,
    WelcomeMessageReceived,
}

pub struct ArchipelagoManager {
    state: ArchipelagoState,

    // Connection
    ws_url: GString,
    last_try_to_connect: Instant,
    ws_peer: Gd<WebSocketPeer>,

    player_address: H160,
    player_profile: Option<UserProfile>,
    player_position: Vector3,
    ephemeral_auth_chain: EphemeralAuthChain,
    avatar_scene: Gd<AvatarScene>,
    last_send_heartbeat: Instant,

    adapter: Option<Box<dyn Adapter>>,
}

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(3);

impl ArchipelagoManager {
    pub fn new(
        ws_url: &str,
        ephemeral_auth_chain: EphemeralAuthChain,
        player_profile: Option<UserProfile>,
        avatar_scene: Gd<AvatarScene>,
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

        Self {
            ws_peer: WebSocketPeer::new(),
            ws_url: GString::from(ws_url),
            state: ArchipelagoState::Connecting,
            player_address: ephemeral_auth_chain.signer(),
            ephemeral_auth_chain,
            player_profile,
            last_try_to_connect: Instant::now(),
            adapter: None,
            avatar_scene,
            player_position: Vector3::new(0.0, 0.0, 0.0),
            last_send_heartbeat: Instant::now(),
        }
    }

    pub fn adapter(&mut self) -> Option<&mut Box<dyn Adapter>> {
        self.adapter.as_mut()
    }

    fn ws_internal_send<T>(&mut self, packet: T, only_when_active: bool) -> bool
    where
        T: Message,
    {
        if let ArchipelagoState::Connecting = self.state {
            return false;
        }

        let should_send = if only_when_active {
            matches!(self.state, ArchipelagoState::WelcomeMessageReceived)
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
            ArchipelagoState::Connecting => match ws_state {
                godot::engine::web_socket_peer::State::STATE_CLOSED => {
                    if (Instant::now() - self.last_try_to_connect).as_secs() > 1 {
                        let ws_protocols = {
                            let mut v = PackedStringArray::new();
                            v.push(GString::from("archipelago"));
                            v
                        };

                        peer.set("supported_protocols".into(), ws_protocols.to_variant());
                        peer.call("connect_to_url".into(), &[self.ws_url.clone().to_variant()]);

                        self.last_try_to_connect = Instant::now();
                    }
                }
                godot::engine::web_socket_peer::State::STATE_OPEN => {
                    self.state = ArchipelagoState::Connected;
                }
                _ => {}
            },
            ArchipelagoState::Connected => match ws_state {
                godot::engine::web_socket_peer::State::STATE_OPEN => {
                    let client_packet = ClientPacket {
                        message: Some(client_packet::Message::ChallengeRequest(
                            ChallengeRequestMessage {
                                address: format!("{:#x}", self.player_address),
                            },
                        )),
                    };
                    self.ws_internal_send(client_packet, false);
                    self.state = ArchipelagoState::IdentMessageSent;
                }
                _ => {
                    self.state = ArchipelagoState::Connecting;
                }
            },
            ArchipelagoState::IdentMessageSent => match ws_state {
                godot::engine::web_socket_peer::State::STATE_OPEN => {
                    while let Some((packet_length, message)) = get_next_packet(peer.clone()) {
                        match message {
                            server_packet::Message::ChallengeResponse(challenge_msg) => {
                                tracing::info!("comms > peer msg {:?}", challenge_msg);

                                let challenge_to_sign = challenge_msg.challenge_to_sign.clone();

                                // TODO: should this block_on be async? the ephemeral wallet is sync
                                let signature = futures_lite::future::block_on(
                                    self.ephemeral_auth_chain
                                        .ephemeral_wallet()
                                        .sign_message(challenge_to_sign.as_bytes()),
                                )
                                .expect("signature by ephemeral should always work");

                                let mut chain = self.ephemeral_auth_chain.auth_chain().clone();
                                chain.add_signed_entity(challenge_to_sign, signature);

                                let auth_chain_json = serde_json::to_string(&chain).unwrap();
                                let client_packet = ClientPacket {
                                    message: Some(client_packet::Message::SignedChallenge(
                                        SignedChallengeMessage { auth_chain_json },
                                    )),
                                };

                                self.ws_internal_send(client_packet, false);
                                self.state = ArchipelagoState::ChallengeMessageSent;
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
                    self.state = ArchipelagoState::Connecting;
                }
            },
            ArchipelagoState::ChallengeMessageSent => match ws_state {
                godot::engine::web_socket_peer::State::STATE_OPEN => {
                    while let Some((packet_length, message)) = get_next_packet(peer.clone()) {
                        match message {
                            server_packet::Message::Welcome(_welcome) => {
                                self.state = ArchipelagoState::WelcomeMessageReceived;
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
                    self.state = ArchipelagoState::Connecting;
                }
            },
            ArchipelagoState::WelcomeMessageReceived => match ws_state {
                godot::engine::web_socket_peer::State::STATE_OPEN => {
                    self._handle_messages();
                    if (Instant::now() - self.last_send_heartbeat) > HEARTBEAT_INTERVAL {
                        let client_packet = ClientPacket {
                            message: Some(client_packet::Message::Heartbeat(Heartbeat {
                                position: Some(Position {
                                    x: self.player_position.x,
                                    y: self.player_position.y,
                                    z: -self.player_position.z,
                                }),
                                desired_room: None,
                            })),
                        };
                        self.ws_internal_send(client_packet, true);
                        self.last_send_heartbeat = Instant::now();
                    }
                }
                _ => {
                    self.state = ArchipelagoState::Connecting;
                }
            },
        }

        let adapter_ok = if let Some(adapter) = self.adapter.as_mut() {
            adapter.poll()
        } else {
            true
        };
        if !adapter_ok {
            self.adapter = None;
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
                server_packet::Message::Kicked(msg) => {
                    tracing::info!("comms > received PeerKicked {:?}", msg.reason);
                    // TODO: message announcing the kick
                    self.ws_peer.close();
                    self.state = ArchipelagoState::Connecting;
                }
                server_packet::Message::IslandChanged(msg) => {
                    tracing::info!("connecting to island {:?}", msg.island_id);
                    let Some((protocol, comms_address)) = msg.conn_str.as_str().split_once(':')
                    else {
                        tracing::error!("unrecognised connection adapter string: {:?}", msg);
                        continue;
                    };
                    match protocol {
                        "livekit" => {
                            self.adapter = Some(Box::new(LivekitRoom::new(
                                comms_address.to_string(),
                                self.ephemeral_auth_chain.signer(),
                                self.player_profile.clone(),
                                self.avatar_scene.clone(),
                            )));
                        }
                        _ => {
                            tracing::info!(
                                "protocol not supported as child of archipelago {:?}",
                                msg
                            )
                        }
                    }
                }
                _ => {}
            }
        }
    }

    pub fn change_profile(&mut self, new_profile: UserProfile) {
        self.player_profile = Some(new_profile);
        if let Some(adapter) = self.adapter.as_mut() {
            adapter.change_profile(self.player_profile.clone().unwrap());
        }
    }

    pub fn update_position(&mut self, position: Vector3) {
        self.player_position = position;
    }
}

fn get_next_packet(mut peer: Gd<WebSocketPeer>) -> Option<(usize, server_packet::Message)> {
    if peer.get_available_packet_count() > 0 {
        let packet = peer.get_packet();
        let packet_length = packet.len();
        let packet = ServerPacket::decode(packet.as_slice());
        if let Ok(packet) = packet {
            packet.message.as_ref()?;
            return Some((packet_length, packet.message.unwrap()));
        }
    }
    None
}
