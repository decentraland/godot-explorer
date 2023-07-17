use std::{collections::HashMap, sync::Arc, time::Instant};

use crate::dcl::components::proto_components::kernel::comms::{
    rfc4,
    rfc5::{ws_packet, WsIdentification, WsPacket, WsSignedChallenge},
};
use ethers::signers::WalletError;
use godot::{
    engine::{TlsOptions, WebSocketPeer},
    prelude::*,
};
use prost::Message;

use super::{
    profile::UserProfile,
    wallet::{self, Wallet},
};

#[derive(Clone)]
enum WsRoomState {
    ResolvingUrl,
    Connecting,
    Connected,
    IdentMessageSent,
    ChallengeMessageWaitingPromise,
    ChallengeMessageSent,
    WelcomeMessageReceived,
}

#[derive(Default)]
struct InitialSignState {
    signed: bool,
    challenge_to_sign: String,

    signing_promise: Option<poll_promise::Promise<Result<ethers::types::Signature, WalletError>>>,
    signature: Option<ethers::types::Signature>,
}

pub struct WebSocketRoom {
    state: WsRoomState,
    ws_peer: Gd<WebSocketPeer>,
    tls_client: Gd<TlsOptions>,

    url: GodotString,
    last_try_time: Instant,
    resolving_url_promise: Option<poll_promise::Promise<Result<String, reqwest::Error>>>,

    wallet: Arc<Wallet>,
    signing_state: InitialSignState,

    from_alias: u32,
    peer_identities: HashMap<u32, String>,
}

async fn get_final_websocket_url(initial_url: &str) -> Result<String, reqwest::Error> {
    let client = reqwest::Client::new();
    let mut response = client.get(initial_url).send().await?;

    let final_url = match response.url().as_str() {
        "" => initial_url.to_string(),
        url => url.to_string(),
    };

    Ok(final_url)
}

impl WebSocketRoom {
    pub fn new(ws_url: &str, tls_client: Gd<TlsOptions>, wallet: Arc<Wallet>) -> Self {
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
            url: GodotString::from(ws_url),
            last_try_time: Instant::now(),
            tls_client,
            wallet,
            signing_state: InitialSignState::default(),
            state: WsRoomState::Connecting,
            from_alias: 0,
            peer_identities: HashMap::new(),
            resolving_url_promise: None,
        }
    }

    pub fn send<T>(&mut self, packet: T, only_when_active: bool) -> bool
    where
        T: Message,
    {
        if let WsRoomState::Connecting = self.state {
            return false;
        }

        let should_send = if only_when_active {
            if let WsRoomState::WelcomeMessageReceived = self.state {
                true
            } else {
                false
            }
        } else {
            true
        };

        if !should_send {
            return false;
        }

        let mut buf = Vec::new();
        if let Err(_) = packet.encode(&mut buf) {
            return false;
        }

        let buf = PackedByteArray::from_iter(buf.into_iter());

        if let godot::engine::global::Error::OK = self.ws_peer.send(buf) {
            true
        } else {
            false
        }
    }

    pub fn poll(&mut self) {
        let mut peer = self.ws_peer.share();
        peer.poll();

        let ws_state = peer.get_ready_state();

        match self.state.clone() {
            WsRoomState::ResolvingUrl => {
                if self.resolving_url_promise.is_none() {
                    let url = self.url.to_string();
                    self.resolving_url_promise = Some(poll_promise::Promise::spawn_thread(
                        "resolving_url",
                        move || {
                            futures_lite::future::block_on(get_final_websocket_url(url.as_str()))
                        },
                    ));
                } else {
                    let promise = self.resolving_url_promise.as_ref().unwrap();

                    if let Some(ret) = promise.ready() {
                        match ret {
                            Ok(url) => {
                                self.url = GodotString::from(url.as_str());
                            }
                            Err(err) => {
                                godot_print!("Error resolving url {:?}", err);
                            }
                        }

                        self.resolving_url_promise = None;
                        self.state = WsRoomState::Connecting;
                    }
                }
            }
            WsRoomState::Connecting => match ws_state {
                godot::engine::web_socket_peer::State::STATE_CLOSED => {
                    if (Instant::now() - self.last_try_time).as_secs() > 1 {
                        // TODO: see if the tls client is really required for now
                        let _tls_client = self.tls_client.share();

                        let ws_protocols = {
                            let mut v = PackedStringArray::new();
                            v.push(GodotString::from("rfc5"));
                            v
                        };

                        peer.set("supported_protocols".into(), ws_protocols.to_variant());
                        peer.call("connect_to_url".into(), &[self.url.clone().to_variant()]);

                        self.last_try_time = Instant::now();
                        self.peer_identities.clear();
                        self.from_alias = 0;
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
                                    address: format!("{:#x}", self.wallet.address()),
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
                    while let Some((packet_length, message)) = get_next_packet(peer.share()) {
                        match message {
                            ws_packet::Message::ChallengeMessage(challenge_msg) => {
                                godot_print!("comms > peer msg {:?}", challenge_msg);

                                self.signing_state.challenge_to_sign =
                                    challenge_msg.challenge_to_sign.clone();

                                let wallet = self.wallet.clone();
                                let challenge_to_sign = challenge_msg.challenge_to_sign.clone();

                                self.signing_state.signing_promise =
                                    Some(poll_promise::Promise::spawn_thread(
                                        "sign_challenge_message",
                                        move || {
                                            futures_lite::future::block_on(
                                                wallet.sign_message(challenge_to_sign.as_bytes()),
                                            )
                                        },
                                    ));

                                self.state = WsRoomState::ChallengeMessageWaitingPromise;
                            }
                            _ => {
                                godot_print!(
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
            WsRoomState::ChallengeMessageWaitingPromise => match ws_state {
                godot::engine::web_socket_peer::State::STATE_OPEN => {
                    if !self.signing_state.signed && self.signing_state.signing_promise.is_some() {
                        let promise = self.signing_state.signing_promise.as_ref().unwrap();
                        if let Some(ret) = promise.ready() {
                            if let Ok(signature) = ret {
                                self.signing_state.signed = true;
                                self.signing_state.signature = Some(*signature);

                                let chain = wallet::SimpleAuthChain::new(
                                    self.wallet.address(),
                                    self.signing_state.challenge_to_sign.clone(),
                                    *signature,
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
                    while let Some((packet_length, message)) = get_next_packet(peer.share()) {
                        match message {
                            ws_packet::Message::WelcomeMessage(welcome_msg) => {
                                // welcome_msg.
                                self.state = WsRoomState::WelcomeMessageReceived;
                                self.from_alias = welcome_msg.alias;
                                self.peer_identities = welcome_msg
                                    .peer_identities
                                    .iter()
                                    .map(|(k, v)| (*k, v.clone()))
                                    .collect();

                                let mut profile = UserProfile::default();
                                profile.content.user_id =
                                    Some(format!("{:#x}", self.wallet.address()));

                                self.send(
                                    rfc4::Packet {
                                        message: Some(rfc4::packet::Message::ProfileVersion(
                                            rfc4::AnnounceProfileVersion {
                                                profile_version: profile.version,
                                            },
                                        )),
                                    },
                                    false,
                                );

                                self.send(
                                    rfc4::Packet {
                                        message: Some(rfc4::packet::Message::ProfileResponse(
                                            rfc4::ProfileResponse {
                                                serialized_profile: serde_json::to_string(
                                                    &profile.content,
                                                )
                                                .unwrap(),
                                                base_url: profile.base_url.clone(),
                                            },
                                        )),
                                    },
                                    false,
                                );
                            }
                            _ => {
                                godot_print!(
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
                    while let Some((packet_length, message)) = get_next_packet(peer.share()) {
                        match message {
                            ws_packet::Message::ChallengeMessage(_)
                            | ws_packet::Message::PeerIdentification(_)
                            | ws_packet::Message::SignedChallengeForServer(_)
                            | ws_packet::Message::WelcomeMessage(_) => {
                                // warn!("unexpected bau message: {message:?}");
                                godot_print!("comms > unexpected bau message {:?}", message);
                            }
                            ws_packet::Message::PeerJoinMessage(peer) => {
                                godot_print!("comms > received PeerJoinMessage {:?}", peer);
                                // debug!("peer joined: {} -> {}", peer.alias, peer.address);
                                // if let Some(h160) = peer.address.as_h160() {
                                //     foreign_aliases.insert(peer.alias, h160);
                                // } else {
                                //     warn!("failed to parse hash: {}", peer.address);
                                // }
                            }
                            ws_packet::Message::PeerLeaveMessage(peer) => {
                                godot_print!("comms > received PeerLeaveMessage {:?}", peer);
                                // debug!(
                                //     "peer left: {} -> {:?}",
                                //     peer.alias,
                                //     foreign_aliases.get_by_left(&peer.alias)
                                // );
                                // foreign_aliases.remove_by_left(&peer.alias);
                            }
                            ws_packet::Message::PeerUpdateMessage(update) => {
                                godot_print!("comms > received PeerUpdateMessage {:?}", update);
                                // let packet = match rfc4::Packet::decode(update.body.as_slice()) {
                                //     Ok(packet) => packet,
                                //     Err(e) => {
                                //         warn!("unable to parse packet body: {e}");
                                //         continue;
                                //     }
                                // };
                                // let Some(message) = packet.message else {
                                //     warn!("received empty packet body");
                                //     continue;
                                // };

                                // let Some(address) = foreign_aliases.get_by_left(&update.from_alias).cloned() else {
                                //     warn!("received packet for unknown alias {}", update.from_alias);
                                //     continue;
                                // };

                                // debug!(
                                //     "[tid: {:?}] received message {:?} from {:?}",
                                //     transport_id, message, address
                                // );
                                // sender
                                //     .send(PlayerUpdate {
                                //         transport_id,
                                //         message,
                                //         address,
                                //     })
                                //     .await?;
                            }
                            ws_packet::Message::PeerKicked(reason) => {
                                godot_print!("comms > received PeerKicked {:?}", reason);
                                // warn!("kicked: {}", reason.reason);
                                // return Ok(());
                            } // _ => {
                              //     godot_print!(
                              //         "comms > received unknown message {} bytes",
                              //         packet_length
                              //     );
                              // }
                        }
                    }
                }
                _ => {
                    self.state = WsRoomState::Connecting;
                }
            },
        }
    }

    pub fn clean(&self) {
        let mut peer = self.ws_peer.share();
        peer.close();
        match peer.get_ready_state() {
            godot::engine::web_socket_peer::State::STATE_OPEN
            | godot::engine::web_socket_peer::State::STATE_CONNECTING => {
                peer.close();
            }
            _ => {}
        }
    }
}

fn get_next_packet(mut peer: Gd<WebSocketPeer>) -> Option<(usize, ws_packet::Message)> {
    if peer.get_available_packet_count() > 0 {
        let packet = peer.get_packet();
        let packet_length = packet.len();
        let packet = WsPacket::decode(packet.as_slice());
        if let Ok(packet) = packet {
            if packet.message.is_none() {
                return None;
            }
            return Some((packet_length, packet.message.unwrap()));
        }
    }
    None
}
