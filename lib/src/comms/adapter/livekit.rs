use std::{collections::HashMap, sync::Arc};

use ethers_core::types::H160;
use futures_util::StreamExt;
use http::Uri;
use livekit::{id::ParticipantIdentity, DataPacket, RoomOptions};

#[cfg(feature = "use_voice_chat")]
use livekit::{
    options::TrackPublishOptions,
    track::{LocalAudioTrack, LocalTrack, TrackSource},
    webrtc::{
        audio_source::native::NativeAudioSource,
        prelude::{AudioSourceOptions, RtcAudioSource},
    },
};
use prost::Message;

use crate::{
    auth::wallet::AsH160,
    comms::profile::UserProfile,
    dcl::{components::proto_components::kernel::comms::rfc4, scene_apis::NetworkMessageRecipient},
    godot_classes::dcl_global::DclGlobal,
};

use super::{
    adapter_trait::Adapter,
    message_processor::{
        IncomingMessage, MessageType, Rfc4Message, StreamerAudioFrameData, StreamerAudioInitData,
        VideoFrameData, VideoInitData, VoiceFrameData, VoiceInitData,
    },
};

// Constants
const CHANNEL_SIZE: usize = 1000;

pub struct NetworkMessage {
    pub data: Vec<u8>,
    pub unreliable: bool,
    pub recipient: NetworkMessageRecipient,
}

pub struct LivekitRoom {
    sender_to_thread: tokio::sync::mpsc::Sender<NetworkMessage>,
    #[cfg(feature = "use_voice_chat")]
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
        Self::new_with_options(remote_address, room_id, true)
    }

    pub fn new_with_options(
        remote_address: String,
        room_id: String,
        #[cfg(feature = "use_voice_chat")] auto_subscribe: bool,
        #[cfg(not(feature = "use_voice_chat"))] _auto_subscribe: bool,
    ) -> Self {
        // Disable auto_subscribe if voice chat is disabled
        #[cfg(not(feature = "use_voice_chat"))]
        let auto_subscribe = false;
        #[cfg(feature = "use_voice_chat")]
        let auto_subscribe = auto_subscribe;
        let room_type = if auto_subscribe {
            "Archipelago/Main"
        } else {
            "Scene"
        };
        tracing::debug!(
            "🔧 Creating {} LiveKit room '{}' with auto_subscribe={}",
            room_type,
            room_id,
            auto_subscribe
        );
        let (sender, receiver_from_thread) = tokio::sync::mpsc::channel(CHANNEL_SIZE);
        let (sender_to_thread, receiver) = tokio::sync::mpsc::channel(CHANNEL_SIZE);

        #[cfg(feature = "use_voice_chat")]
        let (mic_sender_to_thread, mic_receiver) = tokio::sync::mpsc::channel(CHANNEL_SIZE);
        #[cfg(not(feature = "use_voice_chat"))]
        let (_, mic_receiver) = tokio::sync::mpsc::channel(CHANNEL_SIZE);

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
                    auto_subscribe,
                );
            })
            .unwrap();

        Self {
            sender_to_thread,
            #[cfg(feature = "use_voice_chat")]
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
            loop {
                match self.receiver_from_thread.try_recv() {
                    Ok(message) => {
                        if let Err(err) = processor_sender.try_send(message) {
                            tracing::warn!("Failed to forward message to processor: {}", err);
                        }
                    }
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => return true,
                    Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => return false,
                }
            }
        } else {
            loop {
                match self.receiver_from_thread.try_recv() {
                    Ok(_) => {} // drain
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => return true,
                    Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => return false,
                }
            }
        }
    }

    fn _send_rfc4(&mut self, packet: rfc4::Packet, unreliable: bool) -> bool {
        self._send_rfc4_targeted(packet, unreliable, NetworkMessageRecipient::All)
    }

    fn _send_rfc4_targeted(
        &mut self,
        packet: rfc4::Packet,
        unreliable: bool,
        recipient: NetworkMessageRecipient,
    ) -> bool {
        let mut data: Vec<u8> = Vec::new();
        packet.encode(&mut data).unwrap();

        self.sender_to_thread
            .blocking_send(NetworkMessage {
                data,
                unreliable,
                recipient,
            })
            .is_ok()
    }

    pub fn send_rfc4_targeted(
        &mut self,
        packet: rfc4::Packet,
        unreliable: bool,
        recipient: NetworkMessageRecipient,
    ) -> bool {
        self._send_rfc4_targeted(packet, unreliable, recipient)
    }

    fn _broadcast_voice(&mut self, frame: Vec<i16>) {
        #[cfg(feature = "use_voice_chat")]
        {
            let _ = self.mic_sender_to_thread.blocking_send(frame);
        }
        #[cfg(not(feature = "use_voice_chat"))]
        {
            // Voice chat disabled - drop the frame
            let _ = frame;
        }
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
        tracing::error!("Profile changes should be handled by MessageProcessor");
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
        #[cfg(feature = "use_voice_chat")]
        {
            true
        }
        #[cfg(not(feature = "use_voice_chat"))]
        {
            false
        }
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
    auto_subscribe: bool,
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
        tracing::debug!("🔌 LiveKit connecting - room: '{}', auto_subscribe: {}", room_id, auto_subscribe);
        let connect_result = livekit::prelude::Room::connect(&address, &token, RoomOptions{ auto_subscribe, adaptive_stream: false, dynacast: false, ..Default::default() }).await;

        let (room, mut network_rx) = match connect_result {
            Ok(result) => {
                tracing::debug!("🔌 LiveKit connection successful - room: '{}'", room_id);
                result
            }
            Err(e) => {
                tracing::warn!("🔌 LiveKit connection failed - room: '{}', error: {:?}", room_id, e);
                return;
            }
        };

        // Set participant metadata (version, agent, platform)
        let local_identity = room.local_participant().identity().0.clone();
        {
            let version = DclGlobal::get_version().to_string();
            let metadata = serde_json::json!({
                "dcl_version": version,
                "agent": "godot",
                "platform": "mobile"
            }).to_string();

            if let Err(e) = room.local_participant().set_metadata(metadata).await {
                tracing::warn!("Failed to set participant metadata: {:?}", e);
            } else {
                tracing::debug!("Set participant metadata: version={}, agent=godot, platform=mobile", version);
            }
        }

        // Only initialize microphone if voice chat feature is enabled
        #[cfg(feature = "use_voice_chat")]
        {
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
        }

        // Drain mic_receiver if voice chat is disabled to prevent blocking
        #[cfg(not(feature = "use_voice_chat"))]
        {
            rt2.spawn(async move {
                while mic_receiver.recv().await.is_some() {
                    // Just drain the channel
                }
            });
        }

        'stream: loop {
            tokio::select!(
                incoming = network_rx.recv() => {
                    let Some(incoming) = incoming else {
                        tracing::debug!("🔌 LiveKit session ended - room: '{}', reason: network pipe broken", room_id);
                        break 'stream;
                    };

                    match incoming {
                        livekit::RoomEvent::Connected { participants_with_tracks } => {
                            tracing::debug!("Connected to LiveKit room with {} participants", participants_with_tracks.len());

                            // Subscribe to video tracks from streamers already in the room
                            // and check metadata from existing participants
                            for (participant, publications) in participants_with_tracks {
                                let identity = participant.identity();
                                let identity_str = identity.0.as_str();

                                // Check if this is a streamer (identity ends with "-streamer")
                                if identity_str.ends_with("-streamer") {
                                    tracing::debug!("Found streamer {} with {} publications", identity_str, publications.len());
                                    for publication in publications {
                                        tracing::debug!("Subscribing to streamer publication: {:?} (kind: {:?})",
                                            publication.sid(), publication.kind());
                                        publication.set_subscribed(true);
                                    }
                                }

                                // Check metadata from existing participants (for version reporting)
                                if let Some(address) = identity_str.as_h160() {
                                    let metadata = participant.metadata();
                                    if !metadata.is_empty() {
                                        tracing::debug!(
                                            "Existing participant {:#x} has metadata: {}",
                                            address,
                                            metadata
                                        );
                                        if let Err(e) = sender.send(IncomingMessage {
                                            message: MessageType::PeerMetadata(metadata),
                                            address,
                                            room_id: room_id.clone(),
                                        }).await {
                                            tracing::warn!("Failed to send PeerMetadata for existing participant: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                        livekit::RoomEvent::TrackPublished { publication, participant } => {
                            let identity = participant.identity();
                            let identity_str = identity.0.as_str();

                            // Auto-subscribe to video tracks from streamers
                            if identity_str.ends_with("-streamer") {
                                tracing::debug!("Streamer {} published track: {:?} (kind: {:?})",
                                    identity_str, publication.sid(), publication.kind());
                                publication.set_subscribed(true);
                            }
                        }
                        livekit::RoomEvent::DataReceived { payload, participant, .. } => {
                            if participant.is_none() {
                                return;
                            }
                            let participant = participant.unwrap();
                            let identity_str = participant.identity().0.clone();

                            // Resolve address: valid H160 for regular peers, synthetic for
                            // special identities like "authoritative-server"
                            let address = if let Some(addr) = identity_str.as_str().as_h160() {
                                addr
                            } else {
                                H160::from_low_u64_be(1)
                            };

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
                        },
                        livekit::RoomEvent::TrackSubscribed { track, publication: _, participant } => {
                            let identity = participant.identity();
                            let identity_str = identity.0.as_str();
                            let address = identity_str.as_h160();
                            let is_streamer = identity_str.ends_with("-streamer");

                            match track {
                                livekit::track::RemoteTrack::Audio(audio) => {
                                    if is_streamer {
                                        // Streamer audio -> video player audio
                                        let sender = sender.clone();
                                        let room_id_clone = room_id.clone();
                                        let identity_owned = identity_str.to_string();
                                        // Use zero address for streamers
                                        let address = address.unwrap_or_default();

                                        rt2.spawn(async move {
                                            let mut stream = livekit::webrtc::audio_stream::native::NativeAudioStream::new(audio.rtc_track(), 48000, 1);

                                            tracing::debug!("streamer audio track from {:?}", identity_owned);

                                            // get first frame to set sample rate
                                            let Some(frame) = stream.next().await else {
                                                tracing::warn!("dropped streamer audio track without samples");
                                                return;
                                            };

                                            tracing::debug!(
                                                "Streamer audio first frame: sample_rate={}, num_channels={}, samples_per_channel={}, data_len={}",
                                                frame.sample_rate,
                                                frame.num_channels,
                                                frame.samples_per_channel,
                                                frame.data.len()
                                            );

                                            if let Err(e) = sender.send(IncomingMessage {
                                                message: MessageType::InitStreamerAudio(StreamerAudioInitData {
                                                    sample_rate: frame.sample_rate,
                                                    num_channels: frame.num_channels,
                                                    samples_per_channel: frame.samples_per_channel,
                                                }),
                                                address,
                                                room_id: room_id_clone.clone(),
                                            }).await {
                                                tracing::warn!("Failed to send InitStreamerAudio message: {}", e);
                                                return;
                                            }

                                            while let Some(frame) = stream.next().await {
                                                let frame: livekit::webrtc::prelude::AudioFrame = frame;
                                                match sender.try_send(IncomingMessage {
                                                    message: MessageType::StreamerAudioFrame(StreamerAudioFrameData {
                                                        data: frame.data.to_vec(),
                                                    }),
                                                    address,
                                                    room_id: room_id_clone.clone(),
                                                }) {
                                                    Ok(()) => (),
                                                    Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                                                        tracing::debug!("Streamer audio frame dropped: channel full");
                                                    }
                                                    Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                                                        tracing::warn!("streamer audio receiver dropped, exiting task");
                                                        return;
                                                    },
                                                }
                                            }

                                            tracing::debug!("streamer audio track ended, exiting task");
                                        });
                                    } else if let Some(address) = address {
                                        // Regular participant audio -> voice chat
                                        let sender = sender.clone();
                                        let room_id_clone = room_id.clone();
                                        let identity_owned = identity_str.to_string();
                                        rt2.spawn(async move {
                                            let mut x = livekit::webrtc::audio_stream::native::NativeAudioStream::new(audio.rtc_track(), 48000, 1);

                                            tracing::debug!("voice chat audio track from {:?}", identity_owned);

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

                                            tracing::debug!("audio track ended, exiting task");
                                        });
                                    }
                                },
                                livekit::track::RemoteTrack::Video(video) => {
                                    // Video tracks: accept from streamers (no address needed) or participants with addresses
                                    if is_streamer || address.is_some() {
                                        let sender = sender.clone();
                                        let room_id_clone = room_id.clone();
                                        // Use zero address for streamers without ethereum identity
                                        let address = address.unwrap_or_default();
                                        let identity_owned = identity_str.to_string();

                                        rt2.spawn(async move {
                                            use livekit::webrtc::video_stream::native::NativeVideoStream;
                                            use livekit::webrtc::video_frame::VideoBuffer;
                                            use livekit::webrtc::native::yuv_helper;

                                            let mut stream = NativeVideoStream::new(video.rtc_track());

                                            tracing::debug!("video track subscribed from {:?}", identity_owned);

                                            // Get first frame for initialization
                                            let Some(frame) = stream.next().await else {
                                                tracing::warn!("dropped video track without frames");
                                                return;
                                            };

                                            // Get buffer dimensions
                                            let buffer = &frame.buffer;
                                            let width = buffer.width();
                                            let height = buffer.height();

                                            tracing::debug!("Received first video frame: {}x{}, type: {:?}", width, height, buffer.buffer_type());

                                            // Send init message
                                            if let Err(e) = sender.send(IncomingMessage {
                                                message: MessageType::InitVideo(VideoInitData {
                                                    width,
                                                    height,
                                                }),
                                                address,
                                                room_id: room_id_clone.clone(),
                                            }).await {
                                                tracing::warn!("Failed to send InitVideo: {}", e);
                                                return;
                                            }

                                            // Helper function to convert video buffer to RGBA
                                            fn convert_to_rgba(buffer: &dyn VideoBuffer) -> Option<Vec<u8>> {
                                                let width = buffer.width();
                                                let height = buffer.height();
                                                let stride_rgba = width * 4;
                                                let mut rgba_data = vec![0u8; (width * height * 4) as usize];

                                                // Convert to I420 first (common format)
                                                let i420 = buffer.to_i420();

                                                let (stride_y, stride_u, stride_v) = i420.strides();
                                                let (data_y, data_u, data_v) = i420.data();

                                                // Use yuv_helper to convert I420 to ABGR (which is RGBA in little-endian memory)
                                                yuv_helper::i420_to_abgr(
                                                    data_y,
                                                    stride_y,
                                                    data_u,
                                                    stride_u,
                                                    data_v,
                                                    stride_v,
                                                    &mut rgba_data,
                                                    stride_rgba,
                                                    width as i32,
                                                    height as i32,
                                                );

                                                Some(rgba_data)
                                            }

                                            // Convert and send the first frame
                                            if let Some(rgba_data) = convert_to_rgba(buffer.as_ref()) {
                                                if let Err(e) = sender.try_send(IncomingMessage {
                                                    message: MessageType::VideoFrame(VideoFrameData {
                                                        data: rgba_data,
                                                        width,
                                                        height,
                                                    }),
                                                    address,
                                                    room_id: room_id_clone.clone(),
                                                }) {
                                                    tracing::warn!("Failed to send first video frame: {:?}", e);
                                                }
                                            }

                                            // Process subsequent frames
                                            while let Some(frame) = stream.next().await {
                                                let buffer = &frame.buffer;
                                                let width = buffer.width();
                                                let height = buffer.height();

                                                if let Some(rgba_data) = convert_to_rgba(buffer.as_ref()) {
                                                    match sender.try_send(IncomingMessage {
                                                        message: MessageType::VideoFrame(VideoFrameData {
                                                            data: rgba_data,
                                                            width,
                                                            height,
                                                        }),
                                                        address,
                                                        room_id: room_id_clone.clone(),
                                                    }) {
                                                        Ok(()) => (),
                                                        Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                                                            // Drop frame if channel is full (backpressure)
                                                        },
                                                        Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                                                            tracing::warn!("Video receiver dropped, exiting task");
                                                            return;
                                                        },
                                                    }
                                                }
                                            }

                                            tracing::debug!("video track ended, exiting task");
                                        });
                                    }
                                },
                            }
                        }
                        livekit::RoomEvent::ParticipantConnected(participant) => {
                            let identity_str = participant.identity().0.clone();
                            let address = identity_str.as_str().as_h160()
                                .unwrap_or_else(|| H160::from_low_u64_be(1));
                            tracing::debug!("👋 Participant {} ({:#x}) connected to LiveKit room", identity_str, address);
                            if let Err(e) = sender.send(IncomingMessage {
                                message: MessageType::PeerJoined,
                                address,
                                room_id: room_id.clone(),
                            }).await {
                                tracing::warn!("Failed to send PeerJoined: {}", e);
                            }
                        }
                        livekit::RoomEvent::ParticipantDisconnected(participant) => {
                            let identity_str = participant.identity().0.clone();
                            let address = identity_str.as_str().as_h160()
                                .unwrap_or_else(|| H160::from_low_u64_be(1));
                            tracing::debug!("👋 Participant {} ({:#x}) disconnected from LiveKit room", identity_str, address);
                            if let Err(e) = sender.send(IncomingMessage {
                                message: MessageType::PeerLeft,
                                address,
                                room_id: room_id.clone(),
                            }).await {
                                tracing::warn!("Failed to send PeerLeft: {}", e);
                            }
                        }
                        livekit::RoomEvent::Disconnected { reason } => {
                            // Log LiveKit session end with reason
                            tracing::debug!(
                                "🔌 LiveKit session ended - room: '{}', reason: {:?}",
                                room_id,
                                reason
                            );

                            // Map LiveKit disconnect reason to our DisconnectReason
                            use super::message_processor::DisconnectReason;
                            let disconnect_reason = match reason {
                                livekit::DisconnectReason::DuplicateIdentity => DisconnectReason::DuplicateIdentity,
                                livekit::DisconnectReason::RoomClosed => DisconnectReason::RoomClosed,
                                livekit::DisconnectReason::ParticipantRemoved => DisconnectReason::Kicked,
                                livekit::DisconnectReason::RoomDeleted => DisconnectReason::RoomClosed,
                                _ => DisconnectReason::Other,
                            };

                            if let Err(e) = sender.send(IncomingMessage {
                                message: MessageType::Disconnected(disconnect_reason),
                                address: H160::zero(),
                                room_id: room_id.clone(),
                            }).await {
                                tracing::warn!("Failed to send Disconnected: {}", e);
                            }
                            break 'stream;
                        }
                        livekit::RoomEvent::ParticipantMetadataChanged { participant, metadata, .. } => {
                            let identity_str = participant.identity().0.clone();

                            // Skip local participant's own metadata changes
                            if identity_str == local_identity {
                                continue;
                            }

                            // Handle metadata changes from remote participants (version reporting for staging/dev)
                            if let Some(address) = identity_str.as_str().as_h160() {
                                tracing::debug!(
                                    "Received metadata from {:#x}: {}",
                                    address,
                                    metadata
                                );
                                if let Err(e) = sender.send(IncomingMessage {
                                    message: MessageType::PeerMetadata(metadata),
                                    address,
                                    room_id: room_id.clone(),
                                }).await {
                                    tracing::warn!("Failed to send PeerMetadata message: {}", e);
                                }
                            }
                        }
                        _ => { tracing::debug!("Event: {:?}", incoming); }
                    };
                }
                outgoing = receiver.recv() => {
                    let Some(outgoing) = outgoing else {
                        tracing::debug!("🔌 LiveKit session ended - room: '{}', reason: app pipe broken", room_id);
                        break 'stream;
                    };

                    let reliable = !outgoing.unreliable;
                    let destination_identities = match outgoing.recipient {
                        NetworkMessageRecipient::All => Vec::new(),
                        NetworkMessageRecipient::Peer(address) => {
                            vec![ParticipantIdentity(format!("{address:#x}"))]
                        }
                        NetworkMessageRecipient::AuthServer => {
                            vec![ParticipantIdentity("authoritative-server".to_string())]
                        }
                    };

                    if let Err(e) = room.local_participant().publish_data(DataPacket {
                        payload: outgoing.data,
                        reliable,
                        destination_identities,
                        ..Default::default()
                    }).await {
                        tracing::debug!("outgoing failed: {e}; not exiting loop though since it often fails at least once or twice at the start...");
                        // break 'stream;
                    };
                }
            );
        }

        tracing::debug!("🔌 LiveKit room closing - room: '{}'", room_id);
        if let Err(e) = room.close().await {
            tracing::warn!("🔌 LiveKit room.close() failed - room: '{}', error: {}", room_id, e);
        }
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
