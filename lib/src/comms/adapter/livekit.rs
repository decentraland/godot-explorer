use std::{collections::HashMap, sync::Arc};

use ethers_core::types::H160;
use futures_util::StreamExt;
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
    auth::wallet::AsH160, comms::profile::UserProfile,
    dcl::components::proto_components::kernel::comms::rfc4,
};

use super::{
    adapter_trait::Adapter,
    message_processor::{IncomingMessage, MessageType, Rfc4Message, VoiceFrameData, VoiceInitData},
};

// Constants
const CHANNEL_SIZE: usize = 1000;

pub struct NetworkMessage {
    pub data: Vec<u8>,
    pub unreliable: bool,
}

pub struct LivekitRoom {
    sender_to_thread: tokio::sync::mpsc::Sender<NetworkMessage>,
    mic_sender_to_thread: tokio::sync::mpsc::Sender<Vec<i16>>,
    receiver_from_thread:
        tokio::sync::mpsc::Receiver<crate::comms::adapter::message_processor::IncomingMessage>,
    #[allow(dead_code)]
    room_id: String,
    message_processor_sender: Option<
        tokio::sync::mpsc::Sender<crate::comms::adapter::message_processor::IncomingMessage>,
    >,
}

impl LivekitRoom {
    pub fn new(remote_address: String, room_id: String) -> Self {
        tracing::debug!(">> lk connect async : {remote_address}");
        let (sender, receiver_from_thread) = tokio::sync::mpsc::channel(CHANNEL_SIZE);
        let (sender_to_thread, receiver) = tokio::sync::mpsc::channel(CHANNEL_SIZE);
        let (mic_sender_to_thread, mic_receiver) = tokio::sync::mpsc::channel(CHANNEL_SIZE);

        let room_id_clone = room_id.clone();
        let _ = std::thread::Builder::new()
            .name("livekit dcl thread".into())
            .spawn(move || {
                spawn_livekit_task(
                    remote_address,
                    receiver,
                    sender,
                    mic_receiver,
                    room_id_clone,
                );
            })
            .unwrap();

        Self {
            sender_to_thread,
            mic_sender_to_thread,
            receiver_from_thread,
            room_id,
            message_processor_sender: None,
        }
    }

    pub fn set_message_processor_sender(
        &mut self,
        sender: tokio::sync::mpsc::Sender<
            crate::comms::adapter::message_processor::IncomingMessage,
        >,
    ) {
        self.message_processor_sender = Some(sender);
    }

    fn _clean(&mut self) {}

    fn _poll(&mut self) -> bool {
        if let Some(processor_sender) = &self.message_processor_sender {
            // Forward all messages from the LiveKit thread to the message processor
            while let Ok(message) = self.receiver_from_thread.try_recv() {
                if let Err(err) = processor_sender.try_send(message) {
                    tracing::warn!("Failed to forward message to processor: {}", err);
                }
            }
        } else {
            // If no processor is connected, just drain the messages to prevent backing up
            while self.receiver_from_thread.try_recv().is_ok() {}
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

    fn change_profile(&mut self, _new_profile: UserProfile) {
        // Profile changes are now handled by MessageProcessor
        tracing::warn!("Profile changes should be handled by MessageProcessor");
    }

    fn consume_chats(&mut self) -> Vec<(H160, rfc4::Chat)> {
        // Chats are now handled by MessageProcessor
        Vec::new()
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

    fn consume_scene_messages(&mut self, _scene_id: &str) -> Vec<(H160, Vec<u8>)> {
        // Scene messages are now handled by MessageProcessor
        Vec::new()
    }
}

fn spawn_livekit_task(
    remote_address: String,
    mut receiver: tokio::sync::mpsc::Receiver<NetworkMessage>,
    sender: tokio::sync::mpsc::Sender<crate::comms::adapter::message_processor::IncomingMessage>,
    mut mic_receiver: tokio::sync::mpsc::Receiver<Vec<i16>>,
    room_id: String,
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
                                    message: MessageType::Rfc4(Rfc4Message {
                                        message,
                                        protocol_version: packet.protocol_version,
                                    }),
                                    address,
                                    room_id: room_id.clone(),
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
                                        let room_id_clone = room_id.clone();
                                        rt2.spawn(async move {
                                            let mut x = livekit::webrtc::audio_stream::native::NativeAudioStream::new(audio.rtc_track(), 48000, 1);

                                            tracing::debug!("remove track from {:?}", participant.identity().0.as_str());

                                            // get first frame to set sample rate
                                            let Some(frame) = x.next().await else {
                                                tracing::warn!("dropped audio track without samples");
                                                return;
                                            };

                                            if let Err(e) = sender.send(IncomingMessage {
                                                message: MessageType::InitVoice(VoiceInitData {
                                                    sample_rate: frame.sample_rate,
                                                    num_channels: frame.num_channels,
                                                    samples_per_channel: frame.samples_per_channel,
                                                }),
                                                address,
                                                room_id: room_id_clone.clone(),
                                            }).await {
                                                tracing::warn!("Failed to send InitVoice message: {}", e);
                                                return;
                                            }

                                            while let Some(frame) = x.next().await {
                                                let frame: livekit::webrtc::prelude::AudioFrame = frame;
                                                match sender.try_send(IncomingMessage {
                                                    message: MessageType::VoiceFrame(VoiceFrameData {
                                                        data: frame.data.to_vec(),
                                                    }),
                                                    address,
                                                    room_id: room_id_clone.clone(),
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
                        livekit::RoomEvent::ParticipantConnected(participant) => {
                            if let Some(address) = participant.identity().0.as_str().as_h160() {
                                tracing::info!("👋 Participant {:#x} connected to LiveKit room", address);
                                if let Err(e) = sender.send(IncomingMessage {
                                    message: MessageType::PeerJoined,
                                    address,
                                    room_id: room_id.clone(),
                                }).await {
                                    tracing::warn!("Failed to send PeerJoined: {}", e);
                                }
                            }
                        }
                        livekit::RoomEvent::ParticipantDisconnected(participant) => {
                            if let Some(address) = participant.identity().0.as_str().as_h160() {
                                tracing::info!("👋 Participant {:#x} disconnected from LiveKit room", address);
                                if let Err(e) = sender.send(IncomingMessage {
                                    message: MessageType::PeerLeft,
                                    address,
                                    room_id: room_id.clone(),
                                }).await {
                                    tracing::warn!("Failed to send PeerLeft: {}", e);
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
