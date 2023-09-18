use std::{collections::HashMap, sync::Arc, time::Instant};

use ethers::types::H160;
use futures_util::StreamExt;
use godot::prelude::Gd;
use http::Uri;
use livekit::{
    options::TrackPublishOptions,
    track::{LocalAudioTrack, LocalTrack, TrackSource},
    webrtc::{
        audio_source::native::NativeAudioSource,
        prelude::{AudioSourceOptions, RtcAudioSource},
    },
    DataPacketKind, RoomOptions,
};
use prost::Message;

use crate::{
    avatars::avatar_scene::AvatarScene, comms::wallet::AsH160,
    dcl::components::proto_components::kernel::comms::rfc4,
};

use super::{
    player_identity::PlayerIdentity,
    profile::{SerializedProfile, UserProfile},
};

pub struct NetworkMessage {
    pub data: Vec<u8>,
    pub unreliable: bool,
}

struct IncomingMessage {
    message: rfc4::packet::Message,
    address: H160,
}

#[derive(Debug)]
struct Peer {
    alias: u32,
    profile: Option<UserProfile>,
    announced_version: Option<u32>,
}

pub struct LivekitRoom {
    sender_to_thread: tokio::sync::mpsc::Sender<NetworkMessage>,
    receiver_from_thread: tokio::sync::mpsc::Receiver<IncomingMessage>,
    player_identity: Arc<PlayerIdentity>,
    avatars: Gd<AvatarScene>,
    peer_identities: HashMap<H160, Peer>,
    peer_alias_counter: u32,
    last_profile_response_sent: Instant,
    last_profile_request_sent: Instant,
    last_profile_version_announced: u32,
    chats: Vec<(String, rfc4::Chat)>,
}

impl LivekitRoom {
    pub fn new(
        remote_address: String,
        player_identity: Arc<PlayerIdentity>,
        avatars: Gd<AvatarScene>,
    ) -> Self {
        tracing::debug!(">> lk connect async : {remote_address}");
        let (sender, receiver_from_thread) = tokio::sync::mpsc::channel(1000);
        let (sender_to_thread, receiver) = tokio::sync::mpsc::channel(1000);

        let _ = std::thread::Builder::new()
            .name("livekit dcl thread".into())
            .spawn(move || {
                spawn_livekit_task(remote_address, receiver, sender);
            })
            .unwrap();

        Self {
            sender_to_thread,
            receiver_from_thread,
            player_identity,
            avatars,
            peer_identities: HashMap::new(),
            last_profile_response_sent: Instant::now(),
            last_profile_request_sent: Instant::now(),
            peer_alias_counter: 0,
            last_profile_version_announced: 0,
            chats: Vec::new(),
        }
    }

    pub fn clean(&mut self) {}

    pub fn poll(&mut self) {
        loop {
            match self.receiver_from_thread.try_recv() {
                Ok(message) => {
                    let peer = if let Some(value) = self.peer_identities.get_mut(&message.address) {
                        value
                    } else {
                        self.peer_alias_counter += 1;
                        self.peer_identities.insert(
                            message.address,
                            Peer {
                                alias: self.peer_alias_counter,
                                profile: None,
                                announced_version: None,
                            },
                        );
                        self.avatars.bind_mut().add_avatar(self.peer_alias_counter);
                        self.peer_identities.get_mut(&message.address).unwrap()
                    };

                    match message.message {
                        rfc4::packet::Message::Position(position) => {
                            self.avatars
                                .bind_mut()
                                .update_transform(peer.alias, &position);
                        }
                        rfc4::packet::Message::Chat(chat) => {
                            let peer_name = {
                                if let Some(profile) = peer.profile.as_ref() {
                                    profile.content.name.clone()
                                } else {
                                    message.address.to_string()
                                }
                            };
                            self.chats.push((peer_name, chat));
                        }
                        rfc4::packet::Message::ProfileVersion(announce_profile_version) => {
                            peer.announced_version = Some(
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
                                        tracing::error!(
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

                            if incoming_version <= current_version {
                                tracing::error!(
                                    "comms > old or same version ProfileResponse {:?}",
                                    profile_response
                                );
                                continue;
                            }

                            self.avatars.bind_mut().update_avatar(
                                peer.alias,
                                &serialized_profile,
                                &profile_response.base_url,
                            );

                            peer.profile = Some(UserProfile {
                                version: incoming_version,
                                content: serialized_profile,
                                base_url: profile_response.base_url,
                            });
                        }
                        rfc4::packet::Message::Scene(_scene) => {}
                        rfc4::packet::Message::Voice(_voice) => {}
                    }
                }

                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                Err(err) => {
                    tracing::error!("error polling livekit thread: {err}");
                    break;
                }
            }
        }

        if self.last_profile_request_sent.elapsed().as_secs_f32() > 10.0 {
            self.last_profile_request_sent = Instant::now();

            let to_request = self
                .peer_identities
                .iter()
                .filter_map(|(address, peer)| {
                    if peer.profile.is_some() {
                        let announced_version = peer.announced_version.unwrap_or(0);
                        let current_version = peer.profile.as_ref().unwrap().version;

                        if announced_version > current_version {
                            None
                        } else {
                            Some((*address, announced_version))
                        }
                    } else {
                        Some((*address, peer.announced_version.unwrap_or(0)))
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

    pub fn send_rfc4(&mut self, packet: rfc4::Packet, unreliable: bool) -> bool {
        let mut data: Vec<u8> = Vec::new();
        packet.encode(&mut data).unwrap();

        self.sender_to_thread
            .blocking_send(NetworkMessage { data, unreliable })
            .is_ok()
    }

    pub fn change_profile(&mut self, new_profile: Arc<PlayerIdentity>) {
        self.player_identity = new_profile;
    }

    pub fn consume_chats(&mut self) -> Vec<(String, rfc4::Chat)> {
        std::mem::take(&mut self.chats)
    }
}

fn spawn_livekit_task(
    remote_address: String,
    mut receiver: tokio::sync::mpsc::Receiver<NetworkMessage>,
    sender: tokio::sync::mpsc::Sender<IncomingMessage>,
) {
    let url = Uri::try_from(remote_address).unwrap();
    let address = format!(
        "{}://{}{}",
        url.scheme_str().unwrap_or_default(),
        url.host().unwrap_or_default(),
        url.path()
    );
    let params: HashMap<String, String> =
        HashMap::from_iter(url.query().unwrap_or_default().split('&').flat_map(|par| {
            par.split_once('=')
                .map(|(a, b)| (a.to_owned(), b.to_owned()))
        }));
    tracing::debug!("{params:?}");
    let token = params.get("access_token").cloned().unwrap_or_default();

    let rt = Arc::new(
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap(),
    );

    let rt2 = rt.clone();

    let task = rt.spawn(async move {
        let (room, mut network_rx) = livekit::prelude::Room::connect(&address, &token, RoomOptions{ auto_subscribe: true, adaptive_stream: false, dynacast: false }).await.unwrap();
        let native_source = NativeAudioSource::new(AudioSourceOptions{
            echo_cancellation: true,
            noise_suppression: true,
            auto_gain_control: true,
        });
        let mic_track = LocalTrack::Audio(LocalAudioTrack::create_audio_track("mic", RtcAudioSource::Native(native_source.clone())));
        room.local_participant().publish_track(mic_track, TrackPublishOptions{ source: TrackSource::Microphone, ..Default::default() }).await.unwrap();

        // TODO: mic
        // rt2.spawn(async move {
        //     while let Ok(frame) = mic.recv().await {
        //         let data = frame.data.iter().map(|f| (f * i16::MAX as f32) as i16).collect();
        //         native_source.capture_frame(&AudioFrame {
        //             data,
        //             sample_rate: frame.sample_rate,
        //             num_channels: frame.num_channels,
        //             samples_per_channel: frame.data.len() as u32,
        //         })
        //     }
        // });

        'stream: loop {
            tokio::select!(
                incoming = network_rx.recv() => {
                    tracing::debug!("in: {:?}", incoming);
                    let Some(incoming) = incoming else {
                        tracing::debug!("network pipe broken, exiting loop");
                        break 'stream;
                    };

                    match incoming {
                        livekit::RoomEvent::DataReceived { payload, participant, .. } => {
                            if let Some(address) = participant.identity().0.as_str().as_h160() {
                                let packet = match rfc4::Packet::decode(payload.as_slice()) {
                                    Ok(packet) => packet,
                                    Err(e) => {
                                        tracing::warn!("unable to parse packet body: {e}");
                                        continue;
                                    }
                                };
                                let Some(message) = packet.message else {
                                    tracing::warn!("received empty packet body");
                                    continue;
                                };
                                tracing::warn!("received packet {message:?} from {address}");
                                if let Err(e) = sender.send(IncomingMessage {
                                    message,
                                    address,
                                }).await {
                                    tracing::warn!("app pipe broken ({e}), existing loop");
                                    break 'stream;
                                }
                            }
                        },
                        livekit::RoomEvent::TrackSubscribed { track, publication: _, participant } => {
                            if let Some(_address) = participant.identity().0.as_str().as_h160() {
                                match track {
                                    livekit::track::RemoteTrack::Audio(audio) => {
                                        let _sender = sender.clone();
                                        rt2.spawn(async move {
                                            let mut x = livekit::webrtc::audio_stream::native::NativeAudioStream::new(audio.rtc_track());

                                            tracing::debug!("remove track from {:?}", participant.identity().0.as_str());

                                            // get first frame to set sample rate
                                            let Some(_frame) = x.next().await else {
                                                tracing::warn!("dropped audio track without samples");
                                                return;
                                            };

                                            let (frame_sender, _frame_receiver) = tokio::sync::mpsc::channel(10);

                                            // let bridge = LivekitKiraBridge {
                                            //     sample_rate: frame.sample_rate,
                                            //     receiver: frame_receiver,
                                            // };

                                            // let sound_data = kira::sound::streaming::StreamingSoundData::from_decoder(
                                            //     bridge,
                                            //     kira::sound::streaming::StreamingSoundSettings::new(),
                                            // );

                                            // let _ = sender.send(PlayerUpdate {
                                            //     transport_id,
                                            //     message: PlayerMessage::AudioStream(Box::new(sound_data)),
                                            //     address,
                                            // }).await;

                                            while let Some(frame) = x.next().await {
                                                match frame_sender.try_send(frame) {
                                                    Ok(()) => (),
                                                    Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => (),
                                                    Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                                                        tracing::warn!("livekit audio receiver dropped, exiting task");
                                                        return;
                                                    },
                                                }
                                            }

                                            tracing::warn!("track ended, exiting task");
                                        });
                                    },
                                    _ => tracing::warn!("not processing video tracks"),
                                }
                            }
                        }
                        _ => { tracing::debug!("Event: {:?}", incoming); }
                    };
                }
                outgoing = receiver.recv() => {
                    let Some(outgoing) = outgoing else {
                        tracing::debug!("app pipe broken, exiting loop");
                        break 'stream;
                    };

                    let kind = if outgoing.unreliable {
                        DataPacketKind::Lossy
                    } else {
                        DataPacketKind::Reliable
                    };
                    if let Err(e) = room.local_participant().publish_data(outgoing.data, kind, Default::default()).await {
                        tracing::debug!("outgoing failed: {e}; not exiting loop though since it often fails at least once or twice at the start...");
                        // break 'stream;
                    };
                }
            );
        }

        room.close().await.unwrap();
    });

    let _ = rt.block_on(task);
}
