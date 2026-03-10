use std::time::{Duration, Instant};

use crate::{
    auth::ephemeral_auth_chain::EphemeralAuthChain,
    comms::profile::UserProfile,
    dcl::components::proto_components::{
        common::Position,
        kernel::comms::v3::{
            client_packet, server_packet, ChallengeRequestMessage, ClientPacket, Heartbeat,
            ServerPacket, SignedChallengeMessage,
        },
    },
};
use ethers_core::types::H160;
use godot::{classes::WebSocketPeer, prelude::*};
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
    last_send_heartbeat: Instant,

    adapter: Option<Box<dyn Adapter>>,
    shared_processor_sender:
        Option<tokio::sync::mpsc::Sender<super::message_processor::IncomingMessage>>,

    // Reconnection state for LiveKit island rooms
    last_island_conn_str: Option<String>,
    last_island_id: Option<String>,
    island_reconnect_at: Option<Instant>,
}

// Constants
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(3);
const RECONNECT_INTERVAL_SECS: u64 = 5;
const DCL_CHALLENGE_PREFIX: &str = "dcl-";

impl ArchipelagoManager {
    pub fn new(
        ws_url: &str,
        ephemeral_auth_chain: EphemeralAuthChain,
        player_profile: Option<UserProfile>,
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
            ws_peer: WebSocketPeer::new_gd(),
            ws_url: GString::from(&ws_url),
            state: ArchipelagoState::Connecting,
            player_address: ephemeral_auth_chain.signer(),
            ephemeral_auth_chain,
            player_profile,
            last_try_to_connect: Instant::now(),
            adapter: None,
            shared_processor_sender: None,
            player_position: Vector3::new(0.0, 0.0, 0.0),
            last_send_heartbeat: Instant::now(),
            last_island_conn_str: None,
            last_island_id: None,
            island_reconnect_at: None,
        }
    }

    pub fn set_shared_processor_sender(
        &mut self,
        sender: tokio::sync::mpsc::Sender<super::message_processor::IncomingMessage>,
    ) {
        self.shared_processor_sender = Some(sender);
    }

    pub fn adapter_as_mut(&mut self) -> Option<&mut Box<dyn Adapter>> {
        self.adapter.as_mut()
    }

    #[allow(clippy::borrowed_box)]
    pub fn adapter(&self) -> Option<&Box<dyn Adapter>> {
        self.adapter.as_ref()
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
        matches!(self.ws_peer.send(&buf), godot::global::Error::OK)
    }

    pub fn poll(&mut self) {
        let mut peer = self.ws_peer.clone();
        peer.poll();

        let ws_state = peer.get_ready_state();

        match self.state.clone() {
            ArchipelagoState::Connecting => match ws_state {
                godot::classes::web_socket_peer::State::CLOSED => {
                    if (Instant::now() - self.last_try_to_connect).as_secs()
                        > RECONNECT_INTERVAL_SECS
                    {
                        let ws_protocols = {
                            let mut v = PackedStringArray::new();
                            v.push(&GString::from("archipelago"));
                            v
                        };

                        peer.set("supported_protocols", &ws_protocols.to_variant());
                        peer.call("connect_to_url", &[self.ws_url.clone().to_variant()]);

                        self.last_try_to_connect = Instant::now();
                    }
                }
                godot::classes::web_socket_peer::State::OPEN => {
                    self.state = ArchipelagoState::Connected;
                }
                _ => {}
            },
            ArchipelagoState::Connected => match ws_state {
                godot::classes::web_socket_peer::State::OPEN => {
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
                godot::classes::web_socket_peer::State::OPEN => {
                    while let Some((packet_length, message)) = get_next_packet(peer.clone()) {
                        match message {
                            server_packet::Message::ChallengeResponse(challenge_msg) => {
                                tracing::debug!("comms > peer msg {:?}", challenge_msg);

                                let challenge_to_sign = challenge_msg.challenge_to_sign.clone();

                                if !challenge_to_sign.starts_with(DCL_CHALLENGE_PREFIX) {
                                    tracing::error!("invalid challenge to sign");
                                    return;
                                }

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
                                tracing::warn!(
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
                godot::classes::web_socket_peer::State::OPEN => {
                    while let Some((packet_length, message)) = get_next_packet(peer.clone()) {
                        match message {
                            server_packet::Message::Welcome(_welcome) => {
                                self.state = ArchipelagoState::WelcomeMessageReceived;
                            }
                            _ => {
                                tracing::warn!(
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
                godot::classes::web_socket_peer::State::OPEN => {
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
            tracing::warn!(
                "🔌 Archipelago LiveKit room disconnected, scheduling reconnection in 1s"
            );
            self.adapter = None;
            self.island_reconnect_at = Some(Instant::now() + Duration::from_secs(1));
        }

        // Attempt to reconnect to the island LiveKit room
        if self.adapter.is_none()
            && self
                .island_reconnect_at
                .is_some_and(|t| t <= Instant::now())
        {
            if let Some(conn_str) = &self.last_island_conn_str.clone() {
                if let Some((protocol, comms_address)) = conn_str.as_str().split_once(':') {
                    if protocol == "livekit" {
                        if let Some(shared_sender) = &self.shared_processor_sender {
                            let island_id = self.last_island_id.as_deref().unwrap_or("unknown");
                            tracing::debug!(
                                "🔄 Reconnecting to island '{}' LiveKit room",
                                island_id
                            );

                            let mut livekit_room = LivekitRoom::new(
                                comms_address.to_string(),
                                format!("archipelago-livekit-{}", island_id),
                            );
                            livekit_room.set_message_processor_sender(shared_sender.clone());
                            self.adapter = Some(Box::new(livekit_room));
                            self.island_reconnect_at = None;
                        }
                    }
                }
            }
            // If we couldn't reconnect, schedule next attempt with longer backoff
            if self.adapter.is_none() {
                self.island_reconnect_at = Some(Instant::now() + Duration::from_secs(5));
            }
        }
    }

    pub fn clean(&mut self) {
        let mut peer = self.ws_peer.clone();
        peer.close();
        match peer.get_ready_state() {
            godot::classes::web_socket_peer::State::OPEN
            | godot::classes::web_socket_peer::State::CONNECTING => {
                peer.close();
            }
            _ => {}
        }
        self.last_island_conn_str = None;
        self.last_island_id = None;
        self.island_reconnect_at = None;
    }

    fn _handle_messages(&mut self) {
        while let Some((_packet_length, message)) = get_next_packet(self.ws_peer.clone()) {
            match message {
                server_packet::Message::Kicked(msg) => {
                    tracing::debug!("comms > received PeerKicked {:?}", msg.reason);
                    // TODO: message announcing the kick
                    self.ws_peer.close();
                    self.state = ArchipelagoState::Connecting;
                }
                server_packet::Message::IslandChanged(msg) => {
                    tracing::debug!("connecting to island {:?}", msg.island_id);
                    let Some((protocol, comms_address)) = msg.conn_str.as_str().split_once(':')
                    else {
                        tracing::error!("unrecognised connection adapter string: {:?}", msg);
                        continue;
                    };
                    match protocol {
                        "livekit" => {
                            // Store connection info for reconnection
                            self.last_island_conn_str = Some(msg.conn_str.clone());
                            self.last_island_id = Some(msg.island_id.clone());
                            self.island_reconnect_at = None;

                            if let Some(shared_sender) = &self.shared_processor_sender {
                                tracing::debug!(
                                    "Using shared MessageProcessor for archipelago LiveKit room"
                                );

                                // Create LiveKit room with MessageProcessor connection
                                // Archipelago rooms use auto_subscribe: true (default) to automatically receive all peers
                                let mut livekit_room = LivekitRoom::new(
                                    comms_address.to_string(),
                                    format!("archipelago-livekit-{}", msg.island_id),
                                );
                                livekit_room.set_message_processor_sender(shared_sender.clone());

                                self.adapter = Some(Box::new(livekit_room));
                            } else {
                                tracing::error!(
                                    "Cannot create LiveKit adapter: shared_processor_sender is not set"
                                );
                            }
                        }
                        _ => {
                            tracing::warn!(
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
        self.player_profile = Some(new_profile.clone());
        if let Some(adapter) = self.adapter.as_mut() {
            adapter.change_profile(new_profile);
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
