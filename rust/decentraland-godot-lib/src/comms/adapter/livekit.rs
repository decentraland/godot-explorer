use std::{collections::HashMap, sync::Arc, time::Instant};

use ethers::types::H160;
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
    DataPacketKind, RoomOptions,
};
use prost::Message;

use crate::{
    auth::wallet::AsH160,
    avatars::avatar_scene::AvatarScene,
    comms::profile::{SerializedProfile, UserProfile},
    dcl::components::proto_components::kernel::comms::rfc4,
};

use super::adapter_trait::Adapter;

pub struct NetworkMessage {
    pub data: Vec<u8>,
    pub unreliable: bool,
}

enum ToSceneMessage {
    Rfc4(rfc4::packet::Message),
    InitVoice(livekit::webrtc::prelude::AudioFrame),
    VoiceFrame(livekit::webrtc::prelude::AudioFrame),
}

struct IncomingMessage {
    message: ToSceneMessage,
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
    mic_sender_to_thread: tokio::sync::mpsc::Sender<Vec<i16>>,
    receiver_from_thread: tokio::sync::mpsc::Receiver<IncomingMessage>,
    player_address: H160,
    player_profile: Option<UserProfile>,
    avatars: Gd<AvatarScene>,
    peer_identities: HashMap<H160, Peer>,
    peer_alias_counter: u32,
    last_profile_response_sent: Instant,
    last_profile_request_sent: Instant,
    last_profile_version_announced: u32,
    chats: Vec<(String, String, rfc4::Chat)>,
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
            last_profile_version_announced: 0,
            chats: Vec::new(),
        }
    }

    fn _clean(&mut self) {}

    fn _poll(&mut self) -> bool {
        let mut avatar_scene_ref = self.avatars.clone();
        let mut avatar_scene = avatar_scene_ref.bind_mut();

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
                        avatar_scene.add_avatar(
                            self.peer_alias_counter,
                            GString::from(format!("{:#x}", message.address)),
                        );
                        self.peer_identities.get_mut(&message.address).unwrap()
                    };

                    match message.message {
                        ToSceneMessage::Rfc4(rfc4::packet::Message::Position(position)) => {
                            avatar_scene
                                .update_avatar_transform_with_rfc4_position(peer.alias, &position);
                        }
                        ToSceneMessage::Rfc4(rfc4::packet::Message::Chat(chat)) => {
                            let address = format!("{:#x}", message.address);
                            let peer_name = {
                                if let Some(profile) = peer.profile.as_ref() {
                                    profile.content.name.clone()
                                } else {
                                    address.clone()
                                }
                            };
                            self.chats.push((address, peer_name, chat));
                        }
                        ToSceneMessage::Rfc4(rfc4::packet::Message::ProfileVersion(
                            announce_profile_version,
                        )) => {
                            peer.announced_version = Some(
                                announce_profile_version
                                    .profile_version
                                    .max(peer.announced_version.unwrap_or(0)),
                            );
                        }
                        ToSceneMessage::Rfc4(rfc4::packet::Message::ProfileRequest(
                            profile_request,
                        )) => {
                            if self.last_profile_response_sent.elapsed().as_secs_f32() < 10.0 {
                                continue;
                            }

                            tracing::info!("comms > received ProfileRequest {:?}", profile_request);

                            if let Some(addr) = profile_request.address.as_h160() {
                                if addr == self.player_address {
                                    if let Some(profile) = self.player_profile.as_ref() {
                                        self.last_profile_response_sent = Instant::now();

                                        self.send_rfc4(
                                            rfc4::Packet {
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
                        ToSceneMessage::Rfc4(rfc4::packet::Message::ProfileResponse(
                            profile_response,
                        )) => {
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

                            let profile = UserProfile {
                                version: incoming_version,
                                content: serialized_profile.clone(),
                                base_url: profile_response.base_url.clone(),
                            };

                            avatar_scene.update_avatar_by_alias(peer.alias, &profile);
                            peer.profile = Some(profile);
                        }
                        ToSceneMessage::Rfc4(rfc4::packet::Message::Scene(_scene)) => {}
                        ToSceneMessage::Rfc4(rfc4::packet::Message::Voice(_voice)) => {}
                        ToSceneMessage::InitVoice(frame) => {
                            avatar_scene.spawn_voice_channel(
                                peer.alias,
                                frame.sample_rate,
                                frame.num_channels,
                                frame.samples_per_channel,
                            );
                        }
                        ToSceneMessage::VoiceFrame(frame) => {
                            let frame = PackedVector2Array::from_iter(frame.data.iter().map(|c| {
                                let val = (*c as f32) / (i16::MAX as f32);
                                godot::prelude::Vector2 { x: val, y: val }
                            }));

                            avatar_scene.push_voice_frame(peer.alias, frame);
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
        self.player_profile = Some(new_profile);
    }

    fn _consume_chats(&mut self) -> Vec<(String, String, rfc4::Chat)> {
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

    fn consume_chats(&mut self) -> Vec<(String, String, rfc4::Chat)> {
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
}

fn spawn_livekit_task(
    remote_address: String,
    mut receiver: tokio::sync::mpsc::Receiver<NetworkMessage>,
    sender: tokio::sync::mpsc::Sender<IncomingMessage>,
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
        let (room, mut network_rx) = livekit::prelude::Room::connect(&address, &token, RoomOptions{ auto_subscribe: true, adaptive_stream: false, dynacast: false }).await.unwrap();
        let native_source = NativeAudioSource::new(AudioSourceOptions{
            echo_cancellation: true,
            noise_suppression: true,
            auto_gain_control: true,
        });
        let mic_track = LocalTrack::Audio(LocalAudioTrack::create_audio_track("mic", RtcAudioSource::Native(native_source.clone())));
        room.local_participant().publish_track(mic_track, TrackPublishOptions{ source: TrackSource::Microphone, ..Default::default() }).await.unwrap();

        rt2.spawn(async move {
            while let Some(data) = mic_receiver.recv().await {
                let samples_per_channel = data.len() as u32;
                native_source.capture_frame(&livekit::webrtc::prelude::AudioFrame {
                    data,
                    sample_rate: 48000,
                    num_channels: 1,
                    samples_per_channel
                })
            }
        });

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
                                    message: ToSceneMessage::Rfc4(message),
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
                                            let mut x = livekit::webrtc::audio_stream::native::NativeAudioStream::new(audio.rtc_track());

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
