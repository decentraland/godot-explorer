use std::{collections::HashMap, sync::Arc, time::Instant};

use ethers_core::types::H160;
use futures_util::StreamExt;
use godot::prelude::{GString, Gd, PackedVector2Array};
use http::Uri;
use livekit::{
    options::TrackPublishOptions,
    track::{LocalAudioTrack, LocalTrack, TrackSource},
    webrtc::{
        audio_source::native::NativeAudioSource,
        prelude::{AudioSourceOptions, RtcAudioSource},
    },
    DataPacket, RoomOptions,
};
use prost::Message;

use crate::{
    auth::wallet::AsH160, avatars::avatar_scene::AvatarScene, comms::profile::{SerializedProfile, UserProfile}, content::profile::prepare_request_requirements, dcl::components::proto_components::kernel::comms::rfc4, scene_runner::tokio_runtime::TokioRuntime
};

use super::{adapter_trait::Adapter, movement_compressed::MovementCompressed};

use crate::{
    content::profile::request_lambda_profile
};

pub struct NetworkMessage {
    pub data: Vec<u8>,
    pub unreliable: bool,
}

struct Rfc4Message {
    message: rfc4::packet::Message,
    protocol_version: u32,
}

enum ToSceneMessage<'a> {
    Rfc4(Rfc4Message),
    InitVoice(livekit::webrtc::prelude::AudioFrame<'a>),
    VoiceFrame(livekit::webrtc::prelude::AudioFrame<'a>),
}

struct IncomingMessage<'a> {
    message: ToSceneMessage<'a>,
    address: H160,
}

struct ProfileUpdate {
    address: H160,
    peer_alias: u32,
    profile: UserProfile,
}

#[derive(Debug)]
struct Peer {
    alias: u32,
    profile: Option<UserProfile>,
    announced_version: Option<u32>,
    protocol_version: u32,
    last_movement_timestamp: Option<f32>,
    last_position_index: Option<u32>,
    last_activity: Instant,
}

pub struct LivekitRoom {
    sender_to_thread: tokio::sync::mpsc::Sender<NetworkMessage>,
    mic_sender_to_thread: tokio::sync::mpsc::Sender<Vec<i16>>,
    receiver_from_thread: tokio::sync::mpsc::Receiver<IncomingMessage<'static>>,
    player_address: H160,
    player_profile: Option<UserProfile>,
    avatars: Gd<AvatarScene>,
    peer_identities: HashMap<H160, Peer>,
    peer_alias_counter: u32,
    last_profile_response_sent: Instant,
    last_profile_request_sent: Instant,
    chats: Vec<(H160, rfc4::Chat)>,

    // Scene messges
    incoming_scene_messages: HashMap<String, Vec<(H160, Vec<u8>)>>,
    
    // Profile updates from async tasks
    profile_update_receiver: tokio::sync::mpsc::Receiver<ProfileUpdate>,
    profile_update_sender: tokio::sync::mpsc::Sender<ProfileUpdate>,
}

impl LivekitRoom {
    pub fn new(
        remote_address: String,
        player_address: H160,
        player_profile: Option<UserProfile>,
        avatars: Gd<AvatarScene>,
    ) -> Self {
        tracing::debug!(">> lk connect async : {remote_address}");
        let (sender, receiver_from_thread) = tokio::sync::mpsc::channel(1000);
        let (sender_to_thread, receiver) = tokio::sync::mpsc::channel(1000);
        let (mic_sender_to_thread, mic_receiver) = tokio::sync::mpsc::channel(1000);
        let (profile_update_sender, profile_update_receiver) = tokio::sync::mpsc::channel(100);

        let _ = std::thread::Builder::new()
            .name("livekit dcl thread".into())
            .spawn(move || {
                spawn_livekit_task(remote_address, receiver, sender, mic_receiver);
            })
            .unwrap();

        Self {
            sender_to_thread,
            mic_sender_to_thread,
            receiver_from_thread,
            player_address,
            player_profile,
            avatars,
            peer_identities: HashMap::new(),
            last_profile_response_sent: Instant::now(),
            last_profile_request_sent: Instant::now(),
            peer_alias_counter: 0,
            chats: Vec::new(),
            incoming_scene_messages: HashMap::new(),
            profile_update_receiver,
            profile_update_sender,
        }
    }

    fn _clean(&mut self) {}

    fn _poll(&mut self) -> bool {
        let mut avatar_scene_ref = self.avatars.clone();
        let mut avatar_scene = avatar_scene_ref.bind_mut();

        // First, handle any pending profile updates from async tasks
        while let Ok(update) = self.profile_update_receiver.try_recv() {
            tracing::warn!(
                "comms > received profile update for {:#x}: {:?}",
                update.address,
                update.profile
            );
            avatar_scene.update_avatar_by_alias(update.peer_alias, &update.profile);
            if let Some(peer) = self.peer_identities.get_mut(&update.address) {
                peer.profile = Some(update.profile);
            }
        }

        loop {
            match self.receiver_from_thread.try_recv() {
                Ok(message) => {
                    let peer = if let Some(value) = self.peer_identities.get_mut(&message.address) {
                        value
                    } else {
                        // Check if there's an existing peer with the same address (reconnection case)
                        if let Some(existing_peer) = self.peer_identities.remove(&message.address) {
                            tracing::info!("Removing old peer {:#x} (alias: {}) due to reconnection", message.address, existing_peer.alias);
                            avatar_scene.remove_avatar(existing_peer.alias);
                        }

                        self.peer_alias_counter += 1;
                        self.peer_identities.insert(
                            message.address,
                            Peer {
                                alias: self.peer_alias_counter,
                                profile: None,
                                announced_version: None,
                                protocol_version: 100,
                                last_movement_timestamp: None,
                                last_position_index: None,
                                last_activity: Instant::now(),
                            },
                        );
                        avatar_scene.add_avatar(
                            self.peer_alias_counter,
                            GString::from(format!("{:#x}", message.address)),
                        );

                        // Immediately request profile for new peer
                        self.send_rfc4(
                            rfc4::Packet {
                                message: Some(rfc4::packet::Message::ProfileRequest(
                                    rfc4::ProfileRequest {
                                        address: format!("{:#x}", message.address),
                                        profile_version: 0,
                                    },
                                )),
                                protocol_version: 100,
                            },
                            true,
                        );

                        self.peer_identities.get_mut(&message.address).unwrap()
                    };

                    // Update the peer's protocol version and activity
                    if let ToSceneMessage::Rfc4(rfc4_msg) = &message.message {
                        peer.protocol_version = rfc4_msg.protocol_version;
                    }
                    peer.last_activity = Instant::now();

                    match message.message {
                        ToSceneMessage::Rfc4(rfc4_msg) => match rfc4_msg.message {
                            rfc4::packet::Message::Position(position) => {
                                if peer.last_movement_timestamp.is_some() {
                                    continue; // If we received a Movement message, we will comunicate with the Movement message and ignore the Positions
                                }

                                if let Some(last_index) = peer.last_position_index {
                                    if last_index >= position.index {
                                        continue; // Skip if the position index is not newer than the last one
                                    }
                                }

                                // Skip position messages for now
                                tracing::info!(
                                    "Received Position from {:#x}: pos({}, {}, {}), rot({}, {}, {}, {})", 
                                    message.address,
                                    position.position_x, position.position_y, position.position_z,
                                    position.rotation_x, position.rotation_y, position.rotation_z, position.rotation_w
                                );
                                avatar_scene
                                    .update_avatar_transform_with_rfc4_position(peer.alias, &position);
                                peer.last_position_index = Some(position.index);
                            }
                            rfc4::packet::Message::Movement(movement) => {
                                tracing::info!(
                                    "Received Movement from {:#x}: timestamp({}) pos({}, {}, {}), rot_y({}), vel({}, {}, {}) blend({}), slide_blend({})", 
                                    message.address,
                                    movement.timestamp,
                                    movement.position_x, movement.position_y, movement.position_z,
                                    movement.rotation_y,
                                    movement.velocity_x, movement.velocity_y, movement.velocity_z,
                                    movement.movement_blend_value,
                                    movement.slide_blend_value,

                                );

                                // discard if movement.timestamp is too older than the last one
                                if let Some(last_timestamp) = peer.last_movement_timestamp {
                                    if movement.timestamp < last_timestamp {
                                        continue;
                                    }
                                }

                                avatar_scene
                                    .update_avatar_transform_with_movement(peer.alias, &movement);

                                peer.last_movement_timestamp = Some(movement.timestamp);
                            }
                            rfc4::packet::Message::MovementCompressed(
                                movement_compressed,
                            ) => {
                                tracing::debug!("movement compressed data: {movement_compressed:?}");
                                
                                // Decompress movement data
                                let movement = MovementCompressed::from_proto(movement_compressed);

                                // Get realm bounds - you'll need to get these from the actual realm configuration
                                // For now using reasonable default bounds, but this should come from the realm
                                let realm_min = godot::prelude::Vector2i::new(-150, -150);
                                let realm_max = godot::prelude::Vector2i::new(163, 158);
                                
                                // Get position from compressed movement with proper realm bounds
                                let pos = movement.position(realm_min, realm_max);
                                let velocity = movement.velocity();
                                let rotation_rad = movement.temporal.rotation_f32();
                                let timestamp = movement.temporal.timestamp_f32();

                                if let Some(last_timestamp) = peer.last_movement_timestamp {
                                    if timestamp < last_timestamp {
                                        continue; // Skip if the timestamp is not newer than the last one
                                    }
                                }

                                tracing::info!(
                                    "Received MovementCompressed from {:#x}: pos({}, {}, {}), rot_rad({}), vel({}, {}, {}), timestamp({})", 
                                    message.address,
                                    pos.x, pos.y, -pos.z,
                                    rotation_rad,
                                    velocity.x, velocity.y, velocity.z,
                                    timestamp
                                );

                                avatar_scene.update_avatar_transform_with_movement_compressed(
                                    peer.alias,
                                    pos,
                                    rotation_rad,
                                );

                                peer.last_movement_timestamp = Some(timestamp);
                            }
                            rfc4::packet::Message::Chat(chat) => {
                                tracing::info!("Received Chat from {:#x}: {:?}", message.address, chat);
                                self.chats.push((message.address, chat));
                            }
                            rfc4::packet::Message::ProfileVersion(
                                announce_profile_version,
                            ) => {
                                tracing::info!(
                                    "Received ProfileVersion from {:#x}: {:?}",
                                    message.address,
                                    announce_profile_version
                                );

                                let announced_version = announce_profile_version.profile_version;
                                let current_version =
                                    peer.profile.as_ref().map(|p| p.version).unwrap_or(0);

                                // Update the peer's announced version
                                peer.announced_version = Some(announced_version);

                                // If the announced version is newer than what we have, request the profile
                                if announced_version > current_version {
                                    tracing::info!(
                                        "Requesting newer profile from {:#x}: announced={}, current={}",
                                        message.address,
                                        announced_version,
                                        current_version
                                    );

                                    // if peer protocol version is 100, instead of requesting profile, we fetch from lambda
                                    tracing::info!(
                                        "comms > requesting profile from lambda for {:#x}",
                                        message.address
                                    );
                                    // Spawn a task to fetch the profile from the lambda
                                    let address = message.address;
                                    let peer_alias = peer.alias;
                                    let profile_sender = self.profile_update_sender.clone();
                                    let (lamda_server_base_url, profile_base_url, http_requester) =
                                        prepare_request_requirements();

                                    TokioRuntime::spawn(async move {
                                        let result = request_lambda_profile(
                                            address,
                                            lamda_server_base_url.as_str(),
                                            profile_base_url.as_str(),
                                            http_requester,
                                        )
                                        .await;
                                        if let Ok(profile) = result {
                                            tracing::warn!(
                                                "fetch profile lambda > fetch profile from lambda for {:#x}: {:?}",
                                                address,
                                                profile.clone()
                                            );
                                            let _ = profile_sender.send(ProfileUpdate {
                                                address,
                                                peer_alias,
                                                profile,
                                            }).await;
                                        } else {
                                            tracing::error!(
                                                "fetch profile lambda > failed to fetch profile from lambda for {:#x}: {:?}",
                                                address,
                                                result
                                            );
                                        }
                                    });
                                }
                            }
                            rfc4::packet::Message::ProfileRequest(
                                profile_request,
                            ) => {
                                tracing::info!(
                                    "Received ProfileRequest from {:#x}: {:?}",
                                    message.address,
                                    profile_request
                                );

                                tracing::info!("comms > received ProfileRequest {:?}", profile_request);

                                if let Some(addr) = profile_request.address.as_h160() {
                                    if addr == self.player_address {
                                        if let Some(profile) = self.player_profile.as_ref() {
                                            self.last_profile_response_sent = Instant::now();

                                            self.send_rfc4(
                                                rfc4::Packet {
                                                    protocol_version: 100,
                                                    message: Some(
                                                        rfc4::packet::Message::ProfileResponse(
                                                            rfc4::ProfileResponse {
                                                                serialized_profile:
                                                                    serde_json::to_string(
                                                                        &profile.content,
                                                                    )
                                                                    .unwrap(),
                                                                base_url: profile.base_url.clone(),
                                                            },
                                                        ),
                                                    ),
                                                },
                                                false,
                                            );
                                        }
                                    }
                                }
                            }
                            rfc4::packet::Message::ProfileResponse(
                                profile_response,
                            ) => {
                                tracing::info!(
                                    "Received ProfileResponse from {:#x}: {:?}",
                                    message.address,
                                    profile_response
                                );
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
                                    continue;
                                }

                                let profile = UserProfile {
                                    version: incoming_version,
                                    content: serialized_profile.clone(),
                                    base_url: profile_response.base_url.clone(),
                                };

                                avatar_scene.update_avatar_by_alias(peer.alias, &profile);
                                peer.profile = Some(profile);
                            }
                            rfc4::packet::Message::Scene(scene) => {
                                let entry = self
                                    .incoming_scene_messages
                                    .entry(scene.scene_id)
                                    .or_default();

                                // TODO: should we limit the size of the queue or accumulated bytes?
                                entry.push((message.address, scene.data));
                            }
                            rfc4::packet::Message::Voice(_voice) => {}
                            _ => {
                                tracing::debug!("comms > unhandled rfc4 message");
                            }
                        }
                        ToSceneMessage::InitVoice(frame) => {
                            avatar_scene.spawn_voice_channel(
                                peer.alias,
                                frame.sample_rate,
                                frame.num_channels,
                                frame.samples_per_channel,
                            );
                        }
                        ToSceneMessage::VoiceFrame(frame) => {
                            // If all the frame.data is less than 10, we skip the frame
                            if frame.data.iter().all(|&c| c.abs() < 10) {
                                continue;
                            }

                            let frame = PackedVector2Array::from_iter(frame.data.iter().map(|c| {
                                let val = (*c as f32) / (i16::MAX as f32);
                                godot::prelude::Vector2 { x: val, y: val }
                            }));

                            avatar_scene.push_voice_frame(peer.alias, frame);
                        }
                        _ => {
                            tracing::debug!("comms > unhandled message");
                        }
                    }
                }

                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                Err(err) => {
                    tracing::error!("error polling livekit thread: {err}");
                    return false;
                }
            }
        }

        // Remove inactive avatars (avatars that haven't sent messages for 5+ seconds)
        let inactive_threshold = std::time::Duration::from_secs(5);
        let inactive_peers: Vec<H160> = self
            .peer_identities
            .iter()
            .filter_map(|(address, peer)| {
                if peer.last_activity.elapsed() > inactive_threshold {
                    Some(*address)
                } else {
                    None
                }
            })
            .collect();

        for address in inactive_peers {
            if let Some(peer) = self.peer_identities.remove(&address) {
                tracing::info!("Removing inactive avatar {:#x} (alias: {})", address, peer.alias);
                avatar_scene.remove_avatar(peer.alias);
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
                        protocol_version: 100,
                    },
                    true,
                );
            }

            if let Some(profile) = &self.player_profile {
                let profile_version = profile.version;
                tracing::warn!(
                    "comms > announcing profile version {profile_version} to all peers"
                );
                self.send_rfc4(
                    rfc4::Packet {
                        message: Some(rfc4::packet::Message::ProfileVersion(
                            rfc4::AnnounceProfileVersion {
                                profile_version,
                            },
                        )),
                        protocol_version: 100,
                    },
                    false,
                );
            }
        }

        true
    }

    fn _send_rfc4(&mut self, packet: rfc4::Packet, unreliable: bool) -> bool {
        let mut data: Vec<u8> = Vec::new();
        packet.encode(&mut data).unwrap();

        self.sender_to_thread
            .blocking_send(NetworkMessage { data, unreliable })
            .is_ok()
    }

    fn _change_profile(&mut self, new_profile: UserProfile) {
        let profile_version = new_profile.version;
        self.player_profile = Some(new_profile);
        tracing::warn!(
            "comms > changing profile to version {profile_version} for player {:#x}",
            self.player_address
        );

        self.send_rfc4(
            rfc4::Packet {
                message: Some(rfc4::packet::Message::ProfileVersion(
                    rfc4::AnnounceProfileVersion {
                        profile_version,
                    },
                )),
                protocol_version: 100,
            },
            false,
        );
    }

    fn _consume_chats(&mut self) -> Vec<(H160, rfc4::Chat)> {
        std::mem::take(&mut self.chats)
    }

    fn _broadcast_voice(&mut self, frame: Vec<i16>) {
        let _ = self.mic_sender_to_thread.blocking_send(frame);
    }
}

impl Adapter for LivekitRoom {
    fn poll(&mut self) -> bool {
        self._poll()
    }

    fn clean(&mut self) {
        self._clean();
    }

    fn change_profile(&mut self, new_profile: UserProfile) {
        self._change_profile(new_profile);
    }

    fn consume_chats(&mut self) -> Vec<(H160, rfc4::Chat)> {
        self._consume_chats()
    }

    fn send_rfc4(&mut self, packet: rfc4::Packet, unreliable: bool) -> bool {
        self._send_rfc4(packet, unreliable)
    }

    fn broadcast_voice(&mut self, frame: Vec<i16>) {
        self._broadcast_voice(frame);
    }

    fn support_voice_chat(&self) -> bool {
        true
    }

    fn consume_scene_messages(&mut self, scene_id: &str) -> Vec<(H160, Vec<u8>)> {
        if let Some(messages) = self.incoming_scene_messages.get_mut(scene_id) {
            std::mem::take(messages)
        } else {
            Vec::new()
        }
    }
}

fn spawn_livekit_task(
    remote_address: String,
    mut receiver: tokio::sync::mpsc::Receiver<NetworkMessage>,
    sender: tokio::sync::mpsc::Sender<IncomingMessage<'static>>,
    mut mic_receiver: tokio::sync::mpsc::Receiver<Vec<i16>>,
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
        let (room, mut network_rx) = livekit::prelude::Room::connect(&address, &token, RoomOptions{ auto_subscribe: true, adaptive_stream: false, dynacast: false, ..Default::default() }).await.unwrap();
        let native_source = NativeAudioSource::new(AudioSourceOptions{
            echo_cancellation: true,
            noise_suppression: true,
            auto_gain_control: true,
        }, 48000, 1, None);
        let mic_track = LocalTrack::Audio(LocalAudioTrack::create_audio_track("mic", RtcAudioSource::Native(native_source.clone())));
        room.local_participant().publish_track(mic_track, TrackPublishOptions{ source: TrackSource::Microphone, ..Default::default() }).await.unwrap();

        rt2.spawn(async move {
            while let Some(data) = mic_receiver.recv().await {
                let samples_per_channel = data.len() as u32;
                let res = native_source.capture_frame(&livekit::webrtc::prelude::AudioFrame {
                    data: data.into(),
                    sample_rate: 48000,
                    num_channels: 1,
                    samples_per_channel
                }).await;

                if res.is_err() {
                    break;
                }
            }
        });

        'stream: loop {
            tokio::select!(
                incoming = network_rx.recv() => {
                    let Some(incoming) = incoming else {
                        tracing::debug!("network pipe broken, exiting loop");
                        break 'stream;
                    };

                    match incoming {
                        livekit::RoomEvent::DataReceived { payload, participant, .. } => {
                            if participant.is_none() {
                                return;
                            }
                            let participant = participant.unwrap();

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
                                if let Err(e) = sender.send(IncomingMessage {
                                    message: ToSceneMessage::Rfc4(Rfc4Message {
                                        message,
                                        protocol_version: packet.protocol_version,
                                    }),
                                    address,
                                }).await {
                                    tracing::warn!("app pipe broken ({e}), existing loop");
                                    break 'stream;
                                }
                            }
                        },
                        livekit::RoomEvent::TrackSubscribed { track, publication: _, participant } => {
                            if let Some(address) = participant.identity().0.as_str().as_h160() {
                                match track {
                                    livekit::track::RemoteTrack::Audio(audio) => {
                                        let sender = sender.clone();
                                        rt2.spawn(async move {
                                            let mut x = livekit::webrtc::audio_stream::native::NativeAudioStream::new(audio.rtc_track(), 48000, 1);

                                            tracing::debug!("remove track from {:?}", participant.identity().0.as_str());

                                            // get first frame to set sample rate
                                            let Some(frame) = x.next().await else {
                                                tracing::warn!("dropped audio track without samples");
                                                return;
                                            };

                                            let _ = sender.send(IncomingMessage {
                                                message: ToSceneMessage::InitVoice(frame),
                                                address,
                                            }).await;

                                            while let Some(frame) = x.next().await {
                                                let frame: livekit::webrtc::prelude::AudioFrame = frame;
                                                match sender.try_send(IncomingMessage {
                                                    message: ToSceneMessage::VoiceFrame(frame),
                                                    address,
                                                }) {
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

                    let reliable = !outgoing.unreliable;
                    if let Err(e) = room.local_participant().publish_data(DataPacket { payload: outgoing.data, reliable, ..Default::default() }).await {
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

#[cfg(target_os = "android")]
pub mod android {
    use jni::{
        sys::{jint, JNI_VERSION_1_6},
        JavaVM,
    };
    use std::ffi::c_void;

    #[allow(non_snake_case)]
    #[no_mangle]
    pub extern "C" fn JNI_OnLoad(vm: JavaVM, _: *mut c_void) -> jint {
        tracing::debug!("Initializing JNI_OnLoad");
        livekit::webrtc::android::initialize_android(&vm);
        JNI_VERSION_1_6
    }

    #[allow(non_snake_case)]
    #[no_mangle]
    pub extern "C" fn Java_org_webrtc_LibaomAv1Decoder_nativeIsSupported() -> bool {
        tracing::debug!("nativeIsSupported");
        true
    }
}
