use ethers_core::types::H160;
use godot::prelude::*;
use http::Uri;
#[cfg(feature = "use_livekit")]
use std::sync::Arc;
use std::time::{Duration, Instant};

#[cfg(feature = "use_livekit")]
use crate::{
    auth::wallet, comms::consts::DISABLE_ARCHIPELAGO, scene_runner::tokio_runtime::TokioRuntime,
};
use crate::{
    comms::{
        adapter::{
            message_processor::MessageProcessor, movement_compressed::MoveKind,
            ws_room::WebSocketRoom,
        },
        consts::DEFAULT_PROTOCOL_VERSION,
        signed_login::SignedLoginMeta,
    },
    dcl::{components::proto_components::kernel::comms::rfc4, scene_apis::NetworkMessageRecipient},
    godot_classes::dcl_global::DclGlobal,
};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use super::{
    adapter::adapter_trait::Adapter,
    signed_login::{SignedLogin, SignedLoginPollStatus},
};

use crate::comms::adapter::movement_compressed::{Movement, MovementCompressed, Temporal};

#[derive(Serialize, Deserialize)]
pub struct GatekeeperResponse {
    adapter: String,
}

#[derive(Debug)]
pub enum SceneRoomConnectionRequest {
    Connect {
        scene_id: String,
        livekit_url: String,
    },
    Banned {
        scene_id: String,
    },
}

#[derive(Debug)]
enum SceneAdapterError {
    Banned,
    Other(String),
}

#[allow(clippy::large_enum_variant)]
enum MainRoom {
    WebSocket(WebSocketRoom),
    #[cfg(feature = "use_livekit")]
    LiveKit(LivekitRoom),
}

impl MainRoom {
    fn poll(&mut self) -> bool {
        match self {
            MainRoom::WebSocket(ws_room) => {
                ws_room.poll();
                true
            }
            #[cfg(feature = "use_livekit")]
            MainRoom::LiveKit(livekit_room) => livekit_room.poll(),
        }
    }

    fn send_rfc4(&mut self, packet: rfc4::Packet, unreliable: bool) -> bool {
        match self {
            MainRoom::WebSocket(ws_room) => ws_room.send_rfc4(packet, unreliable),
            #[cfg(feature = "use_livekit")]
            MainRoom::LiveKit(livekit_room) => livekit_room.send_rfc4(packet, unreliable),
        }
    }

    fn clean(&mut self) {
        match self {
            MainRoom::WebSocket(ws_room) => ws_room.clean(),
            #[cfg(feature = "use_livekit")]
            MainRoom::LiveKit(livekit_room) => livekit_room.clean(),
        }
    }

    fn support_voice_chat(&self) -> bool {
        match self {
            MainRoom::WebSocket(_) => false,
            #[cfg(feature = "use_livekit")]
            MainRoom::LiveKit(livekit_room) => livekit_room.support_voice_chat(),
        }
    }
}

#[cfg(feature = "use_livekit")]
use crate::{
    comms::adapter::{archipelago::ArchipelagoManager, livekit::LivekitRoom},
    http_request::http_queue_requester::HttpQueueRequester,
};

#[allow(clippy::large_enum_variant)]
enum CommsConnection {
    None,
    WaitingForIdentity(String),
    SignedLogin(SignedLogin),
    #[cfg(feature = "use_livekit")]
    Archipelago(ArchipelagoManager),
    #[allow(dead_code)]
    Connected(Box<dyn Adapter>),
}

/// Main communication orchestrator for Decentraland Godot Explorer
///
/// Manages all network communications including:
/// - Main room connections (WebSocket or LiveKit for archipelago)
/// - Scene-specific rooms (LiveKit for proximity voice/data)
/// - Profile announcements and updates
/// - Movement and position updates
/// - Voice chat
///
/// Architecture:
/// - Uses a shared MessageProcessor for centralized message handling
/// - Supports multiple simultaneous room connections
/// - Handles peer lifecycle across different rooms
#[derive(GodotClass)]
#[class(base=Node)]
pub struct CommunicationManager {
    current_connection: CommsConnection,
    current_connection_str: GString,
    last_position_broadcast_index: u64,
    last_emote_incremental_id: u32,
    is_emoting: bool,
    voice_chat_enabled: bool,
    start_time: Instant,
    last_profile_version_broadcast: Instant,
    archipelago_profile_announced: bool,
    /// Flag to prevent automatic reconnection after DuplicateIdentity disconnect
    block_auto_reconnect: bool,

    /// When true, all comms are disconnected/deferred until loading finishes
    comms_on_hold: bool,
    /// Saved adapter string so we can reconnect after loading finishes
    saved_adapter_for_resume: GString,

    realm_min_bounds: Vector2i,
    realm_max_bounds: Vector2i,

    // LiveKit debug mode
    livekit_debug: bool,
    livekit_debug_last_update: Instant,

    // Shared message processor for all adapters
    message_processor: Option<MessageProcessor>,

    // Store active rooms
    main_room: Option<MainRoom>,
    #[cfg(feature = "use_livekit")]
    scene_room: Option<LivekitRoom>,
    current_scene_id: Option<GString>,

    // Reconnection timer for scene rooms
    #[cfg(feature = "use_livekit")]
    scene_room_reconnect_at: Option<Instant>,

    // Channel for scene room connection requests from async tasks
    #[cfg(feature = "use_livekit")]
    scene_room_connection_receiver: mpsc::Receiver<SceneRoomConnectionRequest>,
    #[cfg(feature = "use_livekit")]
    scene_room_connection_sender: mpsc::Sender<SceneRoomConnectionRequest>,

    // Channel for scene access check results
    scene_access_receiver: mpsc::Receiver<(String, bool, String)>,
    scene_access_sender: mpsc::Sender<(String, bool, String)>,

    base: Base<Node>,
}

#[godot_api]
impl INode for CommunicationManager {
    fn init(base: Base<Node>) -> Self {
        #[cfg(feature = "use_livekit")]
        let (scene_room_connection_sender, scene_room_connection_receiver) = mpsc::channel(10);
        let (scene_access_sender, scene_access_receiver) = mpsc::channel(10);

        CommunicationManager {
            current_connection: CommsConnection::None,
            current_connection_str: GString::default(),
            last_position_broadcast_index: 0,
            last_emote_incremental_id: 0,
            is_emoting: false,
            voice_chat_enabled: false,
            start_time: Instant::now(),
            last_profile_version_broadcast: Instant::now(),
            archipelago_profile_announced: false,
            block_auto_reconnect: false,
            comms_on_hold: false,
            saved_adapter_for_resume: GString::default(),
            livekit_debug: false,
            livekit_debug_last_update: Instant::now(),
            message_processor: None,
            main_room: None,
            #[cfg(feature = "use_livekit")]
            scene_room: None,
            current_scene_id: None,
            #[cfg(feature = "use_livekit")]
            scene_room_reconnect_at: None,
            #[cfg(feature = "use_livekit")]
            scene_room_connection_receiver,
            #[cfg(feature = "use_livekit")]
            scene_room_connection_sender,
            scene_access_receiver,
            scene_access_sender,
            realm_min_bounds: godot::prelude::Vector2i::new(-150, -150),
            realm_max_bounds: godot::prelude::Vector2i::new(163, 158),
            base,
        }
    }

    fn ready(&mut self) {
        self.base_mut().call_deferred("init_rs", &[]);
    }

    fn process(&mut self, _dt: f64) {
        // Handle scene access check results
        while let Ok((scene_id, allowed, error_message)) = self.scene_access_receiver.try_recv() {
            self.base_mut().emit_signal(
                "scene_access_checked",
                &[
                    GString::from(&scene_id).to_variant(),
                    allowed.to_variant(),
                    GString::from(&error_message).to_variant(),
                ],
            );
        }

        // Handle scene room connection requests from async tasks
        #[cfg(feature = "use_livekit")]
        if !self.comms_on_hold {
            while let Ok(request) = self.scene_room_connection_receiver.try_recv() {
                self.handle_scene_room_connection_request(request);
            }
        }

        // Check if we need to announce profile for archipelago (before borrowing)
        #[cfg(feature = "use_livekit")]
        let should_announce_archipelago =
            if let CommsConnection::Archipelago(ref archipelago) = &self.current_connection {
                !self.archipelago_profile_announced && archipelago.adapter().is_some()
            } else {
                false
            };
        #[cfg(not(feature = "use_livekit"))]
        let should_announce_archipelago = false;

        match &mut self.current_connection {
            CommsConnection::None => {}
            CommsConnection::WaitingForIdentity(adapter_url) => {
                let player_identity = DclGlobal::singleton().bind().get_player_identity();

                if player_identity.bind().try_get_address().is_some() {
                    let var = adapter_url.to_variant();
                    self.base_mut().call_deferred("change_adapter", &[var]);
                }
            }
            CommsConnection::SignedLogin(signed_login) => match signed_login.poll() {
                SignedLoginPollStatus::Pending => {}
                SignedLoginPollStatus::Complete(response) => {
                    self.change_adapter(
                        response
                            .fixed_adapter
                            .unwrap_or_else(|| "offline".to_string())
                            .to_godot(),
                    );
                }
                SignedLoginPollStatus::Error(e) => {
                    tracing::error!("Error in signed login: {:?}", e);
                    self.current_connection = CommsConnection::None;
                }
            },
            #[cfg(feature = "use_livekit")]
            CommsConnection::Archipelago(archipelago) => {
                archipelago.poll();
            }
            CommsConnection::Connected(adapter) => {
                let adapter = adapter.as_mut();
                let adapter_polling_ok = adapter.poll();
                let chats = adapter.consume_chats();

                if !chats.is_empty() {
                    let chats_variant_array = get_chat_array(chats);
                    self.base_mut()
                        .emit_signal("chat_message", &[chats_variant_array.to_variant()]);
                }

                if !adapter_polling_ok {
                    self.current_connection = CommsConnection::None;
                }
            }
        }

        // Announce profile for archipelago if needed (after releasing the borrow)
        if should_announce_archipelago {
            self.announce_initial_profile();
            self.archipelago_profile_announced = true;
            tracing::debug!("📡 Initial profile announced for archipelago connection");
        }

        // Poll the shared message processor (if active)
        let mut processor_reset = false;
        let mut chat_signals = Vec::new();
        let mut outgoing_messages = Vec::new();
        let mut disconnect_info: Option<(
            crate::comms::adapter::message_processor::DisconnectReason,
            String,
        )> = None;

        if let Some(processor) = &mut self.message_processor {
            let processor_polling_ok = processor.poll();
            let chats = processor.consume_chats();

            if !chats.is_empty() {
                chat_signals.push(get_chat_array(chats));
            }

            // Handle outgoing messages from MessageProcessor (like ProfileResponse)
            outgoing_messages = processor.consume_outgoing_messages();
            if !outgoing_messages.is_empty() {
                tracing::debug!(
                    "📤 Consumed {} outgoing messages from MessageProcessor",
                    outgoing_messages.len()
                );
            }

            // Check if disconnected from the server (returns reason + room_id)
            disconnect_info = processor.consume_disconnect_reason();

            // Check if room metadata indicates the player was banned.
            // Only clean the scene room — keep main room alive for navigation.
            if processor.consume_room_metadata_banned() {
                tracing::warn!("Detected ban via room metadata — emitting kicked disconnect");
                #[cfg(feature = "use_livekit")]
                {
                    if let Some(scene_room) = &mut self.scene_room {
                        scene_room.clean();
                    }
                    self.scene_room = None;
                    self.scene_room_reconnect_at = None;
                }
                self.current_scene_id = None;
                self.base_mut()
                    .emit_signal("disconnected", &[2i32.to_variant()]);
            }

            if !processor_polling_ok {
                // Reset the message processor if it fails
                processor_reset = true;
            }
        }

        // Handle chat signals after borrowing is done
        for chats_variant_array in chat_signals {
            self.base_mut()
                .emit_signal("chat_message", &[chats_variant_array.to_variant()]);
        }

        // Handle outgoing messages after borrowing is done
        for outgoing in outgoing_messages {
            // Always broadcast to all rooms
            if let Some(main_room) = &mut self.main_room {
                main_room.send_rfc4(outgoing.packet.clone(), outgoing.unreliable);
            }
            #[cfg(feature = "use_livekit")]
            if let Some(scene_room) = &mut self.scene_room {
                scene_room.send_rfc4(outgoing.packet.clone(), outgoing.unreliable);
            }
            tracing::debug!("📤 Broadcast outgoing message to all rooms");
        }

        if processor_reset {
            self.message_processor = None;
        }

        // Emit disconnected signal if needed (after borrowing is done)
        if let Some((reason, room_id)) = disconnect_info {
            use crate::comms::adapter::message_processor::DisconnectReason;

            if reason == DisconnectReason::DuplicateIdentity {
                // DuplicateIdentity from ANY room → full disconnect (existing behavior)
                tracing::warn!("Disconnected from '{}': DuplicateIdentity - another client connected with same identity", room_id);
                self.block_auto_reconnect = true;
                let saved_connection_str = self.current_connection_str.clone();
                self.clean();
                self.current_connection_str = saved_connection_str;

                self.base_mut()
                    .emit_signal("disconnected", &[0i32.to_variant()]);
            } else if room_id.starts_with("scene-") {
                if reason == DisconnectReason::Kicked {
                    // Kicked from scene room → full disconnect (clean all rooms)
                    tracing::warn!("Kicked from scene room '{}' — disconnecting fully", room_id);
                    self.clean();
                    self.base_mut()
                        .emit_signal("disconnected", &[2i32.to_variant()]);
                } else {
                    // Other scene room disconnects → handled by reconnection timer
                    tracing::debug!(
                        "Scene room '{}' disconnected ({:?}) — reconnection scheduled",
                        room_id,
                        reason
                    );
                }
            } else if room_id.starts_with("archipelago-livekit-") {
                // Archipelago LiveKit room disconnect → handled by ArchipelagoManager
                tracing::debug!(
                    "Archipelago room '{}' disconnected ({:?}) — reconnection handled internally",
                    room_id,
                    reason
                );
            } else {
                // Main room disconnect → emit signal (DisconnectHandler handles)
                let reason_code: i32 = match reason {
                    DisconnectReason::DuplicateIdentity => 0,
                    DisconnectReason::RoomClosed => 1,
                    DisconnectReason::Kicked => 2,
                    DisconnectReason::Other => 3,
                };
                tracing::warn!("Disconnected from '{}': {:?}", room_id, reason);
                self.base_mut()
                    .emit_signal("disconnected", &[reason_code.to_variant()]);
            }
        }

        // Poll main room (if active)
        {
            let main_room_alive = if let Some(main_room) = &mut self.main_room {
                main_room.poll()
            } else {
                true
            };
            if !main_room_alive {
                tracing::warn!("🔌 Main room disconnected");
                self.main_room = None;
            }
        }

        // Poll scene room (if active)
        // Skip if we already handled a kick (clean() already removed the scene room)
        #[cfg(feature = "use_livekit")]
        {
            let scene_room_alive = if let Some(scene_room) = &mut self.scene_room {
                scene_room.poll()
            } else {
                true
            };
            if !scene_room_alive {
                self.scene_room = None; // Remove dead room, keep current_scene_id
                                        // Only schedule reconnection if we weren't kicked
                                        // (kick handling above already called clean() which clears current_scene_id)
                if self.current_scene_id.is_some() {
                    tracing::warn!("🔌 Scene room disconnected, scheduling reconnection in 5s");
                    self.scene_room_reconnect_at = Some(Instant::now() + Duration::from_secs(5));
                }
            }
        }

        // Attempt scene room reconnection if timer has expired
        #[cfg(feature = "use_livekit")]
        if !self.comms_on_hold
            && self.scene_room.is_none()
            && self.current_scene_id.is_some()
            && self
                .scene_room_reconnect_at
                .is_some_and(|t| t <= Instant::now())
        {
            self.reconnect_scene_room();
            // Schedule next retry in case this attempt fails
            self.scene_room_reconnect_at = Some(Instant::now() + Duration::from_secs(5));
        }

        // Periodic ProfileVersion broadcasting (every 10 seconds)
        if self.last_profile_version_broadcast.elapsed().as_secs() >= 10 {
            self.broadcast_profile_version();
            self.last_profile_version_broadcast = Instant::now();
        }

        // LiveKit debug: update avatar room labels (~1/sec)
        if self.livekit_debug && self.livekit_debug_last_update.elapsed().as_secs() >= 1 {
            self.livekit_debug_last_update = Instant::now();
            if let Some(processor) = &self.message_processor {
                let avatar_scene = DclGlobal::singleton().bind().get_avatars();
                for (address, rooms) in processor.get_peer_room_info() {
                    let address_str = format!("{:#x}", address);
                    let mut avatar_scene_ref = avatar_scene.clone();
                    avatar_scene_ref
                        .bind_mut()
                        .set_avatar_room_debug_by_address(address_str.to_godot(), rooms.to_godot());
                }
            }
        }
    }
}

impl CommunicationManager {
    #[cfg(feature = "use_livekit")]
    fn create_fallback_connection(&mut self) {
        tracing::debug!("🔧 Creating fallback MessageProcessor for scene room support");

        // Ensure we have a MessageProcessor for scene rooms to work
        let _ = self.ensure_message_processor();

        // Set voice chat to false since we don't have a main connection with voice support
        self.voice_chat_enabled = false;

        let voice_chat_enabled = self.voice_chat_enabled.to_variant();
        self.base_mut().emit_signal(
            "on_adapter_changed",
            &[voice_chat_enabled, "fallback".to_variant()],
        );

        tracing::debug!("✅ Fallback connection established - scene rooms will work");
    }

    fn ensure_message_processor(
        &mut self,
    ) -> mpsc::Sender<crate::comms::adapter::message_processor::IncomingMessage> {
        if self.message_processor.is_none() {
            let player_identity = DclGlobal::singleton().bind().get_player_identity();
            let player_identity_bind = player_identity.bind();
            let player_address = player_identity_bind.get_address();
            let player_profile = player_identity_bind.clone_profile();
            let avatar_scene = DclGlobal::singleton().bind().get_avatars();

            let mut processor = MessageProcessor::new(player_address, player_profile, avatar_scene);

            // Set the social blacklist if available
            let global = DclGlobal::singleton();
            let global_bind = global.bind();
            processor.set_social_blacklist(global_bind.social_blacklist.clone());

            let sender = processor.get_message_sender();
            self.message_processor = Some(processor);
            sender
        } else {
            self.message_processor
                .as_ref()
                .unwrap()
                .get_message_sender()
        }
    }

    #[cfg(feature = "use_livekit")]
    fn reconnect_scene_room(&self) {
        let Some(scene_entity_id) = &self.current_scene_id else {
            return;
        };
        let scene_entity_id = scene_entity_id.to_string();

        let player_identity = DclGlobal::singleton().bind().get_player_identity();
        let player_identity_bind = player_identity.bind();
        let Some(ephemeral_auth_chain) = player_identity_bind.try_get_ephemeral_auth_chain() else {
            tracing::error!("No ephemeral auth chain for scene room reconnection");
            return;
        };
        let realm = DclGlobal::singleton().bind().get_realm();
        let realm_name = realm.bind().get_realm_name().to_string();
        let http_requester: Arc<HttpQueueRequester> = DclGlobal::singleton()
            .bind()
            .get_http_requester()
            .bind()
            .get_http_queue_requester();
        let connection_sender = self.scene_room_connection_sender.clone();

        TokioRuntime::spawn(async move {
            tracing::debug!("🔄 Reconnecting scene room for scene: {}", scene_entity_id);
            match get_scene_adapter(
                http_requester,
                &scene_entity_id,
                &realm_name,
                &ephemeral_auth_chain,
            )
            .await
            {
                Ok(adapter_url) => {
                    if adapter_url.starts_with("livekit:") {
                        let livekit_url =
                            adapter_url.strip_prefix("livekit:").unwrap_or(&adapter_url);
                        let _ = connection_sender
                            .send(SceneRoomConnectionRequest::Connect {
                                scene_id: scene_entity_id,
                                livekit_url: livekit_url.to_string(),
                            })
                            .await;
                    }
                }
                Err(SceneAdapterError::Banned) => {
                    tracing::warn!("🚫 Scene room reconnection denied: user is banned");
                    let _ = connection_sender
                        .send(SceneRoomConnectionRequest::Banned {
                            scene_id: scene_entity_id,
                        })
                        .await;
                }
                Err(SceneAdapterError::Other(e)) => {
                    tracing::warn!("🔄 Scene room reconnection failed (will retry): {}", e);
                }
            }
        });
    }

    pub fn send_scene_message(
        &mut self,
        scene_id: String,
        data: Vec<u8>,
        recipient: NetworkMessageRecipient,
    ) {
        let data_len = data.len();
        let scene_message = rfc4::Packet {
            message: Some(rfc4::packet::Message::Scene(rfc4::Scene { scene_id, data })),
            protocol_version: DEFAULT_PROTOCOL_VERSION,
        };

        // Send only through scene room (matching bevy behavior - scene comms
        // are isolated to the scene room channel, not broadcast via main room)
        #[cfg(feature = "use_livekit")]
        if let Some(scene_room) = &mut self.scene_room {
            tracing::debug!(
                "📤 Sending scene message ({} bytes) via scene room, recipient: {:?}",
                data_len,
                recipient
            );
            scene_room.send_rfc4_targeted(scene_message, false, recipient);
        }
        #[cfg(feature = "use_livekit")]
        if self.scene_room.is_none() {
            tracing::warn!(
                "⚠️ send_scene_message called but scene_room is None — message dropped!"
            );
        }
    }

    pub fn get_pending_messages(&mut self, scene_id: &str) -> Vec<(H160, Vec<u8>)> {
        // Use shared message processor if available
        if let Some(processor) = &mut self.message_processor {
            processor.consume_scene_messages(scene_id)
        } else {
            // Fallback to legacy adapter-based consumption
            match &mut self.current_connection {
                CommsConnection::Connected(adapter) => adapter.consume_scene_messages(scene_id),
                _ => vec![],
            }
        }
    }

    fn broadcast_profile_version(&mut self) {
        let player_identity = DclGlobal::singleton().bind().get_player_identity();
        let player_identity_bind = player_identity.bind();

        if let Some(player_profile) = player_identity_bind.clone_profile() {
            let profile_version_packet = rfc4::Packet {
                message: Some(rfc4::packet::Message::ProfileVersion(
                    rfc4::AnnounceProfileVersion {
                        profile_version: player_profile.version,
                    },
                )),
                protocol_version: DEFAULT_PROTOCOL_VERSION,
            };

            // Send to main room if available
            if let Some(main_room) = &mut self.main_room {
                main_room.send_rfc4(profile_version_packet.clone(), false);
                tracing::debug!(
                    "📡 ProfileVersion broadcast to main room: version {}",
                    player_profile.version
                );
            }

            // Also send to scene room if available
            #[cfg(feature = "use_livekit")]
            if let Some(scene_room) = &mut self.scene_room {
                scene_room.send_rfc4(profile_version_packet.clone(), false);
                tracing::debug!(
                    "📡 ProfileVersion broadcast to scene room: version {}",
                    player_profile.version
                );
            }

            // Send through archipelago's adapter if available
            #[cfg(feature = "use_livekit")]
            if let CommsConnection::Archipelago(archipelago) = &mut self.current_connection {
                if let Some(adapter) = archipelago.adapter_as_mut() {
                    adapter.send_rfc4(profile_version_packet, false);
                    tracing::debug!(
                        "📡 ProfileVersion broadcast through archipelago: version {}",
                        player_profile.version
                    );
                }
            }
        }
    }

    fn announce_initial_profile(&mut self) {
        let player_identity = DclGlobal::singleton().bind().get_player_identity();
        let player_identity_bind = player_identity.bind();

        if let Some(player_profile) = player_identity_bind.clone_profile() {
            // Send ProfileResponse packet
            let profile_response_packet = rfc4::Packet {
                message: Some(rfc4::packet::Message::ProfileResponse(
                    rfc4::ProfileResponse {
                        serialized_profile: serde_json::to_string(&player_profile.content)
                            .unwrap_or_default(),
                        base_url: player_profile.base_url.clone(),
                    },
                )),
                protocol_version: DEFAULT_PROTOCOL_VERSION,
            };

            // Send ProfileVersion packet
            let profile_version_packet = rfc4::Packet {
                message: Some(rfc4::packet::Message::ProfileVersion(
                    rfc4::AnnounceProfileVersion {
                        profile_version: player_profile.version,
                    },
                )),
                protocol_version: DEFAULT_PROTOCOL_VERSION,
            };

            // Send to main room if available
            if let Some(main_room) = &mut self.main_room {
                main_room.send_rfc4(profile_response_packet.clone(), false);
                main_room.send_rfc4(profile_version_packet.clone(), false);
                tracing::debug!(
                    "📡 Initial profile announced to main room: version {}",
                    player_profile.version
                );
            }

            // Also send to scene room if available
            #[cfg(feature = "use_livekit")]
            if let Some(scene_room) = &mut self.scene_room {
                scene_room.send_rfc4(profile_response_packet.clone(), false);
                scene_room.send_rfc4(profile_version_packet.clone(), false);
                tracing::debug!(
                    "📡 Initial profile announced to scene room: version {}",
                    player_profile.version
                );
            }

            // Send through archipelago's adapter if available
            #[cfg(feature = "use_livekit")]
            if let CommsConnection::Archipelago(archipelago) = &mut self.current_connection {
                if let Some(adapter) = archipelago.adapter_as_mut() {
                    adapter.send_rfc4(profile_response_packet, false);
                    adapter.send_rfc4(profile_version_packet, false);
                    tracing::debug!(
                        "📡 Initial profile announced through archipelago: version {}",
                        player_profile.version
                    );
                }
            }
        }
    }
}

#[godot_api]
impl CommunicationManager {
    #[signal]
    fn chat_message(chats: VarArray);

    #[signal]
    fn on_adapter_changed(voice_chat_enabled: bool, new_adapter: GString);

    /// Signal emitted when disconnected from the server
    /// reason: 0 = DuplicateIdentity, 1 = RoomClosed, 2 = Kicked, 3 = Other
    #[signal]
    fn disconnected(reason: i32);

    /// Signal emitted with the result of a scene access check
    /// allowed: true if access is granted, false if banned
    /// error_message: non-empty if the check failed (network error, etc.)
    #[signal]
    fn scene_access_checked(scene_id: GString, allowed: bool, error_message: GString);

    #[func]
    fn broadcast_voice(&mut self, frame: PackedVector2Array) {
        let adapter = if let Some(main_room) = &mut self.main_room {
            match main_room {
                MainRoom::WebSocket(_) => None, // WebSocket doesn't support voice
                #[cfg(feature = "use_livekit")]
                MainRoom::LiveKit(livekit_room) => Some(livekit_room as &mut dyn Adapter),
            }
        } else {
            match &mut self.current_connection {
                CommsConnection::Connected(adapter) => Some(adapter.as_mut()),
                #[cfg(feature = "use_livekit")]
                CommsConnection::Archipelago(archipelago) => {
                    archipelago.adapter_as_mut().map(|a| a.as_mut())
                }
                _ => None,
            }
        };

        let Some(adapter) = adapter else {
            return;
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
    #[allow(clippy::too_many_arguments)]
    fn broadcast_movement(
        &mut self,
        compressed: bool,
        position: Vector3,
        rotation_y: f32,
        velocity: Vector3,
        walk: bool,
        run: bool,
        jog: bool,
        rise: bool,
        fall: bool,
        land: bool,
    ) -> bool {
        // Update archipelago position if connected via archipelago
        #[cfg(feature = "use_livekit")]
        if let CommsConnection::Archipelago(archipelago) = &mut self.current_connection {
            archipelago.update_position(position);
        }

        let velocity = Vector3::new(velocity.x, velocity.y, -velocity.z);

        let get_packet = || {
            if compressed {
                // Get elapsed time since start
                let time = self.start_time.elapsed().as_secs_f64();

                let movement = Movement::new(
                    position,
                    velocity,
                    self.realm_min_bounds,
                    self.realm_max_bounds,
                );

                // Determine move kind from parameters
                let move_kind = if run {
                    MoveKind::Run
                } else if jog {
                    MoveKind::Jog
                } else if walk {
                    MoveKind::Walk
                } else {
                    MoveKind::Idle
                };

                // Temporal::with_rotation_f32 expects radians (quantizes over 0..TAU)
                let temporal = Temporal::from_parts(
                    time,
                    false,
                    rotation_y, // radians from Godot
                    movement.velocity_tier(),
                    move_kind,
                    !fall && !rise,
                );

                let movement_compressed = MovementCompressed { temporal, movement };

                let movement_packet = rfc4::MovementCompressed {
                    temporal_data: i32::from_le_bytes(movement_compressed.temporal.into_bytes()),
                    movement_data: i64::from_le_bytes(movement_compressed.movement.into_bytes()),
                };

                rfc4::Packet {
                    message: Some(rfc4::packet::Message::MovementCompressed(movement_packet)),
                    protocol_version: DEFAULT_PROTOCOL_VERSION,
                }
            } else {
                // Create regular Movement packet with all required fields

                // Get elapsed time since start
                let timestamp = self.start_time.elapsed().as_secs_f32();

                // Calculate movement blend value based on velocity and movement type
                let movement_blend_value = if run {
                    3.0
                } else if jog {
                    2.0
                } else if walk {
                    1.0
                } else {
                    0.0
                };

                let movement_packet = rfc4::Movement {
                    timestamp,
                    position_x: position.x,
                    position_y: position.y,
                    position_z: -position.z,
                    velocity_x: velocity.x,
                    velocity_y: velocity.y,
                    velocity_z: velocity.z,
                    rotation_y,
                    movement_blend_value,
                    slide_blend_value: 0.0,
                    is_grounded: land,
                    is_jumping: rise,
                    is_long_jump: false,
                    is_long_fall: false,
                    is_falling: fall,
                    is_stunned: false,
                    is_instant: false,
                    is_emoting: self.is_emoting,
                };

                rfc4::Packet {
                    message: Some(rfc4::packet::Message::Movement(movement_packet)),
                    protocol_version: DEFAULT_PROTOCOL_VERSION,
                }
            }
        };

        // Send to main room if available
        let mut message_sent = if let Some(main_room) = &mut self.main_room {
            let sent = main_room.send_rfc4(get_packet(), true);
            if sent {
                tracing::debug!("📡 Movement sent to main room");
            }
            sent
        } else {
            false
        };

        // Also send to scene room if available (dual broadcasting)
        #[cfg(feature = "use_livekit")]
        if let Some(scene_room) = &mut self.scene_room {
            let scene_sent = scene_room.send_rfc4(get_packet(), true);
            message_sent = message_sent || scene_sent; // Consider successful if either main or scene room succeeded
            if scene_sent {
                tracing::debug!("📡 Movement also sent to scene room");
            }
        }

        // Also send through archipelago's adapter if available
        #[cfg(feature = "use_livekit")]
        if let CommsConnection::Archipelago(archipelago) = &mut self.current_connection {
            if let Some(adapter) = archipelago.adapter_as_mut() {
                let sent = adapter.send_rfc4(get_packet(), true);
                if sent {
                    tracing::debug!("📡 Movement also sent through archipelago");
                    message_sent = true;
                }
            }
        }

        if message_sent {
            self.last_position_broadcast_index += 1;
        }
        message_sent
    }

    #[func]
    fn broadcast_position_and_rotation(&mut self, position: Vector3, rotation: Quaternion) -> bool {
        // Update archipelago position if connected via archipelago
        #[cfg(feature = "use_livekit")]
        if let CommsConnection::Archipelago(archipelago) = &mut self.current_connection {
            archipelago.update_position(position);
        }

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
                protocol_version: DEFAULT_PROTOCOL_VERSION,
            }
        };

        // Send to main room if available
        let mut message_sent = if let Some(main_room) = &mut self.main_room {
            let sent = main_room.send_rfc4(get_packet(), true);
            if sent {
                tracing::debug!("📡 Position sent to main room");
            }
            sent
        } else {
            false
        };

        // Also send to scene room if available (dual broadcasting)
        #[cfg(feature = "use_livekit")]
        if let Some(scene_room) = &mut self.scene_room {
            let scene_sent = scene_room.send_rfc4(get_packet(), true);
            message_sent = message_sent || scene_sent; // Consider successful if either main or scene room succeeded
            if scene_sent {
                tracing::debug!("📡 Position also sent to scene room");
            }
        }

        // Also send through archipelago's adapter if available
        #[cfg(feature = "use_livekit")]
        if let CommsConnection::Archipelago(archipelago) = &mut self.current_connection {
            if let Some(adapter) = archipelago.adapter_as_mut() {
                let sent = adapter.send_rfc4(get_packet(), true);
                if sent {
                    tracing::debug!("📡 Position also sent through archipelago");
                    message_sent = true;
                }
            }
        }

        if message_sent {
            self.last_position_broadcast_index += 1;
        }
        message_sent
    }

    /// Called when the social blacklist changes to update the MessageProcessor cache
    #[func]
    pub fn on_blacklist_changed(&mut self) {
        if let Some(processor) = &mut self.message_processor {
            processor.refresh_blacklist_cache();
        }
    }

    #[func]
    fn send_chat(&mut self, text: GString) -> bool {
        let packet = rfc4::Packet {
            message: Some(rfc4::packet::Message::Chat(rfc4::Chat {
                message: text.to_string(),
                timestamp: self.start_time.elapsed().as_secs_f64(),
            })),
            protocol_version: DEFAULT_PROTOCOL_VERSION,
        };

        let mut sent = false;

        // Send to main room if available
        if let Some(main_room) = &mut self.main_room {
            sent = main_room.send_rfc4(packet.clone(), false) || sent;
        }

        // Also send to scene room if available
        #[cfg(feature = "use_livekit")]
        if let Some(scene_room) = &mut self.scene_room {
            sent = scene_room.send_rfc4(packet, false) || sent;
        }

        sent
    }

    #[func]
    pub fn send_emote(&mut self, emote_urn: GString) -> bool {
        let timestamp = godot::classes::Time::singleton().get_unix_time_from_system() * 1000.0;
        self.send_chat(GString::from(
            format!("␐{} {}", emote_urn, timestamp).as_str(),
        ));

        self.last_emote_incremental_id += 1;
        self.is_emoting = true;

        let packet = rfc4::Packet {
            message: Some(rfc4::packet::Message::PlayerEmote(rfc4::PlayerEmote {
                urn: emote_urn.to_string(),
                incremental_id: self.last_emote_incremental_id,
                timestamp: self.start_time.elapsed().as_secs_f32(),
            })),
            protocol_version: DEFAULT_PROTOCOL_VERSION,
        };

        let mut sent = false;

        // Send to main room if available
        if let Some(main_room) = &mut self.main_room {
            sent = main_room.send_rfc4(packet.clone(), false) || sent;
        }

        // Also send to scene room if available
        #[cfg(feature = "use_livekit")]
        if let Some(scene_room) = &mut self.scene_room {
            sent = scene_room.send_rfc4(packet, false) || sent;
        }

        sent
    }

    #[func]
    pub fn set_emoting(&mut self, emoting: bool) {
        self.is_emoting = emoting;
    }

    #[func]
    fn init_rs(&mut self) {
        DclGlobal::singleton()
            .bind()
            .get_realm()
            .connect("realm_changed", &self.base().callable("_on_realm_changed"));

        let mut player_identity = DclGlobal::singleton().bind().get_player_identity();
        player_identity.connect(
            "profile_changed",
            &self.base().callable("_on_profile_changed"),
        );

        // Connect to social blacklist changes
        let global = DclGlobal::singleton();
        let global_bind = global.bind();
        let mut social_blacklist = global_bind.social_blacklist.clone();
        social_blacklist.connect(
            "blacklist_changed",
            &self.base().callable("on_blacklist_changed"),
        );

        #[cfg(feature = "use_livekit")]
        {
            let mut scene_runner = DclGlobal::singleton().bind().get_scene_runner();
            scene_runner.connect(
                "on_change_scene_id",
                &self.base().callable("_on_change_scene_id"),
            );
        }

        #[cfg(feature = "use_livekit")]
        {
            let scene_runner = DclGlobal::singleton().bind().get_scene_runner();
            self._on_change_scene_id(scene_runner.bind().get_current_parcel_scene_id());
        }
    }

    #[func]
    fn _on_profile_changed(&mut self, _: Variant) {
        self.base_mut().call_deferred("_on_update_profile", &[]);
    }

    #[func]
    fn _on_realm_changed(&mut self) {
        self.base_mut()
            .call_deferred("_on_realm_changed_deferred", &[]);
    }

    fn _internal_get_comms_from_realm(&self) -> Option<(String, Option<GString>)> {
        let realm = DclGlobal::singleton().bind().get_realm();
        let realm_about = VarDictionary::from_variant(&realm.get("realm_about"));
        let comms = VarDictionary::from_variant(&realm_about.get(StringName::from("comms"))?);
        let comms_protocol = String::from_variant(&comms.get(StringName::from("protocol"))?);

        let fixed_adapter_raw = comms
            .get(StringName::from("fixedAdapter"))
            .map(|v| GString::from_variant(&v).to_string());
        let adapter_raw = comms
            .get(StringName::from("adapter"))
            .map(|v| GString::from_variant(&v).to_string());

        #[cfg(feature = "use_livekit")]
        let disable_archipelago = DISABLE_ARCHIPELAGO;
        #[cfg(not(feature = "use_livekit"))]
        let disable_archipelago = false;

        let comms_fixed_adapter = parse_comms_adapter_value(
            fixed_adapter_raw.as_deref(),
            adapter_raw.as_deref(),
            disable_archipelago,
            cfg!(feature = "use_livekit"),
        );

        tracing::debug!(
            "Comms protocol: {}, fixedAdapter: {:?}",
            comms_protocol,
            comms_fixed_adapter
        );

        Some((comms_protocol, comms_fixed_adapter.map(|s| s.to_godot())))
    }

    #[func]
    fn _on_realm_changed_deferred(&mut self) {
        // Skip automatic reconnection if blocked (e.g., after DuplicateIdentity)
        if self.block_auto_reconnect {
            tracing::warn!("Skipping automatic reconnection due to block_auto_reconnect flag");
            return;
        }

        self.clean();

        let comms = self._internal_get_comms_from_realm();
        if comms.is_none() {
            tracing::warn!("invalid comms from realm.");
            return;
        }

        let (comms_protocol, comms_fixed_adapter) = comms.unwrap();
        if comms_protocol != "v3" {
            tracing::warn!("Only protocol 'v3' is supported.");
            return;
        }

        if comms_fixed_adapter.is_none() {
            #[cfg(feature = "use_livekit")]
            if DISABLE_ARCHIPELAGO {
                // When archipelago is disabled, fall back to a direct LiveKit connection
                tracing::warn!(
                    "🔄 Archipelago disabled, attempting fallback to direct LiveKit connection"
                );
                // Try to create a direct LiveKit connection as fallback
                self.create_fallback_connection();
            } else {
                tracing::warn!("As far, only fixedAdapter is supported.");
            }
            #[cfg(not(feature = "use_livekit"))]
            tracing::warn!("As far, only fixedAdapter is supported.");
            return;
        }

        let comms_fixed_adapter_str = comms_fixed_adapter.unwrap().to_string();

        // If loading, save the adapter for reconnection after loading finishes
        if self.comms_on_hold {
            self.saved_adapter_for_resume = comms_fixed_adapter_str.to_godot();
            tracing::info!("Realm changed while comms on hold, saved adapter for resume");
            return;
        }

        self.change_adapter(comms_fixed_adapter_str.to_godot());
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
        self.current_connection_str
            .clone_from(&comms_fixed_adapter_str.to_godot());
        self.archipelago_profile_announced = false; // Reset flag when changing adapters
        self.block_auto_reconnect = false; // Reset block flag to allow this reconnection
        let avatar_scene = DclGlobal::singleton().bind().get_avatars();

        tracing::warn!(
            "🔌 Comms reconnection attempt - protocol: '{}', address: '{}'",
            protocol,
            comms_address
        );

        let current_ephemeral_auth_chain = player_identity
            .bind()
            .try_get_ephemeral_auth_chain()
            .expect("ephemeral auth chain needed to start a comms connection");

        let player_profile = player_identity.bind().clone_profile();

        match protocol {
            "ws-room" => {
                // Ensure shared message processor is created
                let processor_sender = self.ensure_message_processor();

                // Create WebSocket room with shared message processor
                let mut ws_room = WebSocketRoom::new(
                    comms_address,
                    format!("ws-room-{}", comms_address),
                    current_ephemeral_auth_chain,
                    player_profile,
                    avatar_scene,
                );
                ws_room.set_message_processor_sender(processor_sender);

                // Store the room - no need to change connection type
                self.main_room = Some(MainRoom::WebSocket(ws_room));

                // Announce initial profile to the room
                self.announce_initial_profile();
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
                    .get("realm_url")
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
                // Ensure shared message processor is created
                let processor_sender = self.ensure_message_processor();

                // Create LiveKit room with shared message processor
                // Main rooms use auto_subscribe: true (default) to automatically receive all peers
                let mut livekit_room = LivekitRoom::new(
                    comms_address.to_string(),
                    format!("livekit-{}", comms_address),
                );
                livekit_room.set_message_processor_sender(processor_sender);

                // Store the room - no need to change connection type
                self.main_room = Some(MainRoom::LiveKit(livekit_room));

                // Announce initial profile to the room
                self.announce_initial_profile();
            }

            #[cfg(not(feature = "use_livekit"))]
            "livekit" => {
                tracing::error!("livekit wasn't included in this build");
            }

            "offline" => {
                tracing::debug!("set offline");
            }
            #[cfg(feature = "use_livekit")]
            "archipelago" => {
                if DISABLE_ARCHIPELAGO {
                    tracing::debug!(
                        "⚠️  Archipelago connections are disabled (DISABLE_ARCHIPELAGO = true)"
                    );
                } else {
                    // Ensure we have a message processor
                    let processor_sender = self.ensure_message_processor();

                    let mut archipelago = ArchipelagoManager::new(
                        comms_address,
                        current_ephemeral_auth_chain.clone(),
                        player_profile,
                    );
                    archipelago.set_shared_processor_sender(processor_sender);

                    self.current_connection = CommsConnection::Archipelago(archipelago);
                }
            }
            _ => {
                tracing::debug!("unknown adapter {:?}", protocol);
            }
        }

        // Determine voice chat support based on available adapters
        self.voice_chat_enabled = if let Some(main_room) = &self.main_room {
            main_room.support_voice_chat()
        } else {
            match &self.current_connection {
                CommsConnection::Connected(adapter) => adapter.support_voice_chat(),
                #[cfg(feature = "use_livekit")]
                CommsConnection::Archipelago(archipelago) => {
                    if let Some(adapter) = archipelago.adapter() {
                        adapter.support_voice_chat()
                    } else {
                        // Only support voice if the feature is enabled
                        #[cfg(feature = "use_voice_chat")]
                        {
                            true
                        }
                        #[cfg(not(feature = "use_voice_chat"))]
                        {
                            false
                        }
                    }
                }
                _ => false,
            }
        };

        let voice_chat_enabled = self.voice_chat_enabled.to_variant();
        self.base_mut().emit_signal(
            "on_adapter_changed",
            &[voice_chat_enabled, comms_fixed_adapter_gstr.to_variant()],
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

        // Clean up shared message processor
        if let Some(processor) = &mut self.message_processor {
            processor.clean();
        }
        self.message_processor = None;

        // Clean up rooms
        if let Some(main_room) = &mut self.main_room {
            main_room.clean();
        }
        self.main_room = None;

        #[cfg(feature = "use_livekit")]
        if let Some(scene_room) = &mut self.scene_room {
            scene_room.clean();
        }
        #[cfg(feature = "use_livekit")]
        {
            self.scene_room = None;
            self.scene_room_reconnect_at = None;
        }
        self.current_scene_id = None;
        self.current_connection = CommsConnection::None;
        self.current_connection_str = GString::default();
        self.archipelago_profile_announced = false;
    }

    /// Disconnect all comms while a scene is loading.
    /// Saves the current adapter so we can reconnect after loading finishes.
    #[func]
    pub fn hold_comms(&mut self) {
        // Always save adapter if comms are connected, even if already on hold.
        // Handles the case where hold is called at startup (comms not ready)
        // and then again from _on_loading_started (comms now connected).
        if !self.current_connection_str.is_empty() {
            self.saved_adapter_for_resume = self.current_connection_str.clone();
            self.clean();
        }
        self.comms_on_hold = true;
        tracing::info!("Comms on hold (loading)");
    }

    /// Reconnect all comms after loading finishes.
    #[func]
    pub fn release_comms(&mut self) {
        if !self.comms_on_hold {
            return;
        }
        self.comms_on_hold = false;

        // Discard stale scene room connection requests that queued during the hold
        #[cfg(feature = "use_livekit")]
        while self.scene_room_connection_receiver.try_recv().is_ok() {}

        let adapter = std::mem::take(&mut self.saved_adapter_for_resume);
        if !adapter.is_empty() {
            tracing::info!("Comms resuming after loading");
            self.change_adapter(adapter);
        }

        // Reconnect the scene room — _on_change_scene_id saved current_scene_id
        // during the hold but deferred the actual connection.
        #[cfg(feature = "use_livekit")]
        if self.current_scene_id.is_some() {
            self.reconnect_scene_room();
        }
    }

    #[func]
    fn _on_update_profile(&mut self) {
        let dcl_player_identity = DclGlobal::singleton().bind().get_player_identity();
        let player_identity = dcl_player_identity.bind();

        let Some(player_profile) = player_identity.clone_profile() else {
            return;
        };
        // Update shared message processor if available
        if let Some(processor) = &mut self.message_processor {
            processor.change_profile(player_profile.clone());
        }

        // Also update adapters that need direct profile updates
        let profile_version = player_profile.version;
        match &mut self.current_connection {
            CommsConnection::Connected(adapter) => adapter.change_profile(player_profile.clone()),
            #[cfg(feature = "use_livekit")]
            CommsConnection::Archipelago(archipelago) => archipelago.change_profile(player_profile),
            _ => {}
        }

        // Immediately broadcast ProfileVersion when profile changes
        self.broadcast_profile_version();
        tracing::debug!(
            "📡 Profile changed - immediately broadcasting ProfileVersion: version {}",
            profile_version
        );
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
        self.current_connection_str.clone()
    }

    #[func]
    pub fn get_current_scene_room_id(&self) -> GString {
        self.current_scene_id.clone().unwrap_or_default()
    }

    #[func]
    pub fn is_connected_to_scene_room(&self) -> bool {
        #[cfg(feature = "use_livekit")]
        {
            self.scene_room.is_some()
        }
        #[cfg(not(feature = "use_livekit"))]
        {
            false
        }
    }

    #[cfg(feature = "use_livekit")]
    fn handle_scene_room_connection_request(&mut self, request: SceneRoomConnectionRequest) {
        match request {
            SceneRoomConnectionRequest::Banned { scene_id } => {
                // Handle banned signal from scene room connection/reconnection.
                // Only clean the scene room — keep main room alive so the user can navigate away.
                tracing::warn!(
                    "🚫 User is banned from scene '{}' — emitting kicked disconnect",
                    scene_id
                );
                if let Some(scene_room) = &mut self.scene_room {
                    scene_room.clean();
                }
                self.scene_room = None;
                self.scene_room_reconnect_at = None;
                self.current_scene_id = None;
                self.base_mut()
                    .emit_signal("disconnected", &[2i32.to_variant()]);
            }
            SceneRoomConnectionRequest::Connect {
                scene_id,
                livekit_url,
            } => {
                tracing::debug!(
                    "🔌 Processing scene room connection request for scene '{}' with URL: {}",
                    scene_id,
                    livekit_url
                );

                // Ensure we have a message processor (create one if needed)
                let processor_sender = self.ensure_message_processor();

                // Clean up existing scene room
                if let Some(scene_room) = &mut self.scene_room {
                    tracing::debug!("🧹 Cleaning up existing scene room");
                    scene_room.clean();
                }

                // Create new LiveKit room for the scene
                // Scene rooms use auto_subscribe: false to manually control subscriptions
                let room_id = format!("scene-{}", scene_id);
                tracing::debug!("🚀 Creating new scene room with ID: {}", room_id);

                let mut scene_room =
                    LivekitRoom::new_with_options(livekit_url.clone(), room_id, false);

                // Connect the scene room to the message processor
                scene_room.set_message_processor_sender(processor_sender);

                self.scene_room = Some(scene_room);
                self.scene_room_reconnect_at = None;

                // Announce initial profile to the scene room
                self.announce_initial_profile();

                tracing::debug!(
                    "✅ Scene room successfully created and connected to message processor"
                );
            }
        }
    }

    #[cfg(feature = "use_livekit")]
    #[func]
    pub fn _on_change_scene_id(&mut self, scene_id: i32) {
        use std::sync::Arc;

        use crate::{
            comms::consts::DISABLE_SCENE_ROOM,
            http_request::http_queue_requester::HttpQueueRequester,
        };

        let scene_runner = DclGlobal::singleton().bind().get_scene_runner();
        let scene_entity_id = scene_runner.bind().get_scene_entity_id(scene_id);

        if scene_entity_id.is_empty() {
            return; // ignore if empty
        }

        // Check if scene has actually changed
        if let Some(current_scene) = &self.current_scene_id {
            if current_scene == &scene_entity_id {
                return; // No change needed
            }
        }

        tracing::debug!("Scene changed to: {}", scene_entity_id);

        // Clean up existing scene room and cancel any pending reconnection
        if let Some(scene_room) = &mut self.scene_room {
            scene_room.clean();
        }
        self.scene_room = None;
        self.scene_room_reconnect_at = None;
        self.current_scene_id = Some(scene_entity_id.clone());

        // If loading is in progress, defer scene room creation until release
        if self.comms_on_hold {
            tracing::debug!(
                "Scene room on hold, deferring connection for: {}",
                scene_entity_id
            );
            return;
        }

        // Check if scene rooms are disabled
        if DISABLE_SCENE_ROOM {
            tracing::debug!("⚠️  Scene room connections are disabled (DISABLE_SCENE_ROOM = true)");
            return;
        }

        // Get player identity for signing
        let player_identity = DclGlobal::singleton().bind().get_player_identity();
        let player_identity_bind = player_identity.bind();

        let Some(ephemeral_auth_chain) = player_identity_bind.try_get_ephemeral_auth_chain() else {
            tracing::error!("No ephemeral auth chain available for scene room connection");
            return;
        };

        let _avatar_scene = DclGlobal::singleton().bind().get_avatars();
        let _player_profile = player_identity_bind.clone_profile();

        // Spawn async task to get scene adapter and connect
        let scene_entity_id = scene_entity_id.to_string();
        let realm = DclGlobal::singleton().bind().get_realm();
        let realm_name = realm.bind().get_realm_name().to_string();
        let http_requester: Arc<HttpQueueRequester> = DclGlobal::singleton()
            .bind()
            .get_http_requester()
            .bind()
            .get_http_queue_requester();
        let connection_sender = self.scene_room_connection_sender.clone();

        self.set_realm_bounds(
            realm.bind().get_realm_min_bounds(),
            realm.bind().get_realm_max_bounds(),
        );

        TokioRuntime::spawn(async move {
            tracing::debug!("Requesting scene adapter for scene: {}", scene_entity_id);
            match get_scene_adapter(
                http_requester,
                &scene_entity_id,
                &realm_name,
                &ephemeral_auth_chain,
            )
            .await
            {
                Ok(adapter_url) => {
                    tracing::debug!(
                        "✅ Got scene adapter URL for scene '{}': {}",
                        scene_entity_id,
                        adapter_url
                    );

                    // Parse the adapter URL to extract LiveKit connection details
                    if adapter_url.starts_with("livekit:") {
                        // Extract the actual LiveKit URL after "livekit:"
                        let livekit_url =
                            adapter_url.strip_prefix("livekit:").unwrap_or(&adapter_url);
                        tracing::debug!(
                            "🔗 Preparing to connect scene room to LiveKit: {}",
                            livekit_url
                        );

                        // Send connection request to main thread via channel
                        let request = SceneRoomConnectionRequest::Connect {
                            scene_id: scene_entity_id.clone(),
                            livekit_url: livekit_url.to_string(),
                        };

                        match connection_sender.send(request).await {
                            Ok(()) => {
                                tracing::debug!("📤 Scene room connection request sent to main thread for scene '{}'", scene_entity_id);
                            }
                            Err(e) => {
                                tracing::error!(
                                    "❌ Failed to send scene room connection request: {}",
                                    e
                                );
                            }
                        }
                    } else {
                        tracing::warn!("⚠️  Unsupported scene adapter type: {}", adapter_url);
                    }
                }
                Err(SceneAdapterError::Banned) => {
                    tracing::warn!(
                        "🚫 Scene adapter denied: user is banned from scene '{}'",
                        scene_entity_id
                    );
                    let _ = connection_sender
                        .send(SceneRoomConnectionRequest::Banned {
                            scene_id: scene_entity_id,
                        })
                        .await;
                }
                Err(SceneAdapterError::Other(e)) => {
                    tracing::error!(
                        "❌ Failed to get scene adapter for scene '{}': {}",
                        scene_entity_id,
                        e
                    );
                }
            }
        });
    }

    #[func]
    pub fn set_realm_bounds(&mut self, min_bounds: Vector2i, max_bounds: Vector2i) {
        if let Some(processor) = self.message_processor.as_mut() {
            self.realm_min_bounds = min_bounds;
            self.realm_max_bounds = max_bounds;
            processor.set_realm_bounds(min_bounds, max_bounds);
            tracing::debug!(
                "🌐 Realm bounds updated: min=({}, {}), max=({}, {})",
                min_bounds.x,
                min_bounds.y,
                max_bounds.x,
                max_bounds.y
            );
        } else {
            tracing::warn!("⚠️  Cannot set realm bounds: MessageProcessor not initialized");
        }
    }

    #[func]
    pub fn set_livekit_debug(&mut self, enabled: bool) {
        self.livekit_debug = enabled;
        if !enabled {
            // Clear all avatar room debug labels
            let avatar_scene = DclGlobal::singleton().bind().get_avatars();
            if let Some(processor) = &self.message_processor {
                for (address, _) in processor.get_peer_room_info() {
                    let address_str = format!("{:#x}", address);
                    let mut avatar_scene_ref = avatar_scene.clone();
                    avatar_scene_ref
                        .bind_mut()
                        .set_avatar_room_debug_by_address(
                            address_str.to_godot(),
                            GString::default(),
                        );
                }
            }
        }
    }

    #[func]
    pub fn get_livekit_debug(&self) -> bool {
        self.livekit_debug
    }

    #[func]
    pub fn get_debug_room_info(&self) -> VarDictionary {
        let mut dict = VarDictionary::new();
        dict.set(
            "adapter".to_variant(),
            self.current_connection_str.to_variant(),
        );

        // Main room / archipelago status
        let main_connected = self.main_room.is_some()
            || matches!(&self.current_connection, CommsConnection::Connected(_))
            || {
                #[cfg(feature = "use_livekit")]
                {
                    matches!(&self.current_connection, CommsConnection::Archipelago(_))
                }
                #[cfg(not(feature = "use_livekit"))]
                {
                    false
                }
            };
        dict.set("main_connected".to_variant(), main_connected.to_variant());

        // Scene room status
        let scene_room_id = self.current_scene_id.clone().unwrap_or_default();
        dict.set("scene_room".to_variant(), scene_room_id.to_variant());
        dict.set(
            "comms_on_hold".to_variant(),
            self.comms_on_hold.to_variant(),
        );
        #[cfg(feature = "use_livekit")]
        {
            let scene_connected = self.scene_room.is_some();
            dict.set("scene_connected".to_variant(), scene_connected.to_variant());
        }
        #[cfg(not(feature = "use_livekit"))]
        {
            dict.set("scene_connected".to_variant(), false.to_variant());
        }
        dict
    }

    #[func]
    pub fn get_debug_peer_rooms(&self) -> VarArray {
        let mut arr = VarArray::new();
        if let Some(processor) = &self.message_processor {
            for (address, rooms) in processor.get_peer_room_info() {
                let mut dict = VarDictionary::new();
                let address_str = format!("{:#x}", address);
                dict.set("address".to_variant(), address_str.to_variant());
                dict.set("rooms".to_variant(), rooms.to_variant());
                arr.push(&dict.to_variant());
            }
        }
        arr
    }

    /// Check if the current user has access to a scene (not banned).
    /// Emits `scene_access_checked(scene_id, allowed, error_message)` with the result.
    #[func]
    pub fn check_scene_access(&self, scene_id: GString, realm_name: GString) {
        let scene_id_str = scene_id.to_string();
        let realm_name_str = realm_name.to_string();

        let player_identity = DclGlobal::singleton().bind().get_player_identity();
        let player_identity_bind = player_identity.bind();

        let Some(ephemeral_auth_chain) = player_identity_bind.try_get_ephemeral_auth_chain() else {
            tracing::warn!("No ephemeral auth chain for scene access check");
            let sender = self.scene_access_sender.clone();
            let sid = scene_id_str.clone();
            TokioRuntime::spawn(async move {
                let _ = sender
                    .send((sid, true, "No auth chain available".to_string()))
                    .await;
            });
            return;
        };

        let http_requester: Arc<HttpQueueRequester> = DclGlobal::singleton()
            .bind()
            .get_http_requester()
            .bind()
            .get_http_queue_requester();
        let sender = self.scene_access_sender.clone();

        TokioRuntime::spawn(async move {
            let result = check_gatekeeper_access(
                http_requester,
                &scene_id_str,
                &realm_name_str,
                &ephemeral_auth_chain,
            )
            .await;

            match result {
                Ok(allowed) => {
                    let _ = sender.send((scene_id_str, allowed, String::new())).await;
                }
                Err(e) => {
                    tracing::warn!("Scene access check failed: {}", e);
                    let _ = sender.send((scene_id_str, true, e)).await;
                }
            }
        });
    }
}

/// Check if the user has access to a scene via the gatekeeper.
/// Returns Ok(true) if allowed, Ok(false) if banned (HTTP 403), Err for other failures.
async fn check_gatekeeper_access(
    http_requester: Arc<HttpQueueRequester>,
    scene_id: &str,
    realm_name: &str,
    ephemeral_auth_chain: &crate::auth::ephemeral_auth_chain::EphemeralAuthChain,
) -> Result<bool, String> {
    use crate::{
        comms::consts::{gatekeeper_url, gatekeeper_url_local},
        http_request::request_response::{RequestOption, ResponseType},
    };

    let gatekeeper = if scene_id.starts_with("b64-") {
        gatekeeper_url_local()
    } else {
        gatekeeper_url()
    };

    let request_body = serde_json::json!({
        "sceneId": scene_id,
        "realmName": realm_name
    });
    let metadata_json_string = request_body.to_string();

    let uri = gatekeeper
        .parse::<http::Uri>()
        .map_err(|e| format!("Invalid gatekeeper URL: {}", e))?;
    let method = http::Method::POST;

    let headers =
        wallet::sign_request(method.as_str(), &uri, ephemeral_auth_chain, request_body).await;

    let request_option = RequestOption::new(
        0,
        uri.to_string(),
        method,
        ResponseType::AsJson,
        Some(metadata_json_string.as_bytes().to_vec()),
        Some(headers.into_iter().collect()),
        None,
    );

    let response = http_requester
        .request(request_option, 0)
        .await
        .map_err(|e| format!("Request failed: {}", e.error_message))?;

    if response.status_code == http::StatusCode::FORBIDDEN {
        tracing::info!(
            "Scene access denied (403) for scene '{}' in realm '{}'",
            scene_id,
            realm_name
        );
        return Ok(false);
    }

    if !response.status_code.is_success() {
        return Err(format!("HTTP error: {}", response.status_code));
    }

    Ok(true)
}

#[cfg(feature = "use_livekit")]
async fn get_scene_adapter(
    http_requester: Arc<HttpQueueRequester>,
    scene_id: &str,
    realm_name: &str,
    ephemeral_auth_chain: &crate::auth::ephemeral_auth_chain::EphemeralAuthChain,
) -> Result<String, SceneAdapterError> {
    // Create the request body

    use crate::{
        comms::consts::{gatekeeper_url, gatekeeper_url_local},
        http_request::request_response::{RequestOption, ResponseEnum, ResponseType},
    };

    // Use preview gatekeeper for local scenes (b64- prefix indicates local preview)
    let gatekeeper = if scene_id.starts_with("b64-") {
        gatekeeper_url_local()
    } else {
        gatekeeper_url()
    };

    let request_body = serde_json::json!({
        "sceneId": scene_id,
        "realmName": realm_name
    });
    let metadata_json_string = request_body.to_string();

    tracing::debug!("🔄 Making scene adapter request to: {}", gatekeeper);
    tracing::debug!("📋 Request body: {}", metadata_json_string);

    // Create URI
    let uri = gatekeeper
        .parse::<http::Uri>()
        .map_err(|e| SceneAdapterError::Other(format!("Invalid gatekeeper URL: {}", e)))?;
    let method = http::Method::POST;

    // Sign the request
    tracing::debug!("🔐 Signing request with ephemeral auth chain");
    let headers = wallet::sign_request(
        method.as_str(),
        &uri,
        ephemeral_auth_chain,
        request_body, // Pass the serde_json::Value directly, not the string
    )
    .await;

    tracing::debug!("📝 Generated {} authentication headers", headers.len());

    let request_option = RequestOption::new(
        0,
        uri.to_string(),
        method,
        ResponseType::AsJson,
        Some(metadata_json_string.as_bytes().to_vec()),
        Some(headers.into_iter().collect()),
        None,
    );

    let response = http_requester
        .request(request_option, 0)
        .await
        .map_err(|e| SceneAdapterError::Other(format!("Request failed: {}", e.error_message)))?;

    tracing::debug!(
        "📡 Received HTTP response with status: {}",
        response.status_code
    );

    if response.status_code == http::StatusCode::FORBIDDEN {
        tracing::info!(
            "Scene adapter denied (403) for scene '{}' in realm '{}' — user is banned",
            scene_id,
            realm_name
        );
        return Err(SceneAdapterError::Banned);
    }

    if !response.status_code.is_success() {
        tracing::error!(
            "❌ HTTP request failed with status: {}",
            response.status_code
        );
        return Err(SceneAdapterError::Other(format!(
            "HTTP error: {}",
            response.status_code
        )));
    }

    // Extract response data
    let response_data = response
        .response_data
        .map_err(|e| SceneAdapterError::Other(format!("Response data error: {}", e)))?;

    // Parse the response based on type
    tracing::debug!("🔍 Parsing gatekeeper response");
    let gatekeeper_response: GatekeeperResponse = match response_data {
        ResponseEnum::String(text) => {
            tracing::debug!("📄 Response as string: {}", text);
            serde_json::from_str(&text)
                .map_err(|e| SceneAdapterError::Other(format!("JSON parse error: {}", e)))?
        }
        ResponseEnum::Json(json_result) => {
            let json_value = json_result
                .map_err(|e| SceneAdapterError::Other(format!("JSON result error: {}", e)))?;
            tracing::debug!("📊 Response as JSON: {}", json_value);
            serde_json::from_value(json_value)
                .map_err(|e| SceneAdapterError::Other(format!("JSON value parse error: {}", e)))?
        }
        ResponseEnum::Bytes(bytes) => {
            let text = String::from_utf8(bytes)
                .map_err(|e| SceneAdapterError::Other(format!("Invalid UTF-8: {}", e)))?;
            tracing::debug!("📄 Response as bytes->string: {}", text);
            serde_json::from_str(&text)
                .map_err(|e| SceneAdapterError::Other(format!("JSON parse error: {}", e)))?
        }
        _ => {
            return Err(SceneAdapterError::Other(
                "Unexpected response type".to_string(),
            ))
        }
    };

    tracing::debug!(
        "✅ Successfully parsed gatekeeper response: adapter = '{}'",
        gatekeeper_response.adapter
    );
    Ok(gatekeeper_response.adapter)
}

fn get_chat_array(chats: Vec<(H160, rfc4::Chat)>) -> VarArray {
    let mut chats_variant_array = VarArray::new();
    for (address, chat) in chats {
        let mut chat_arr = VarArray::new();
        let address = format!("{:#x}", address);
        chat_arr.push(&address.to_variant());
        chat_arr.push(&chat.timestamp.to_variant());
        chat_arr.push(&chat.message.to_variant());

        chats_variant_array.push(&chat_arr.to_variant());
    }
    chats_variant_array
}

/// Parse the comms adapter from realm about data.
///
/// Takes the raw `adapter` or `fixedAdapter` value from the about endpoint's comms section
/// and resolves it to the final adapter string that should be passed to `change_adapter`.
///
/// Returns `None` if the adapter value is not recognized.
///
/// The `fixed_adapter` parameter corresponds to the `fixedAdapter` key.
/// The `adapter` parameter corresponds to the `adapter` key.
/// `fixed_adapter` takes priority over `adapter` when both are present.
fn parse_comms_adapter_value(
    fixed_adapter: Option<&str>,
    adapter: Option<&str>,
    disable_archipelago: bool,
    livekit_enabled: bool,
) -> Option<String> {
    // fixedAdapter takes priority
    let raw = if let Some(fa) = fixed_adapter {
        Some(fa.to_string())
    } else if let Some(a) = adapter {
        if let Some(archipelago_url) = a.strip_prefix("archipelago:") {
            if !livekit_enabled {
                tracing::debug!(
                    "⚠️  Archipelago URL detected but LiveKit feature is not enabled: {}",
                    a
                );
                return None;
            }
            if disable_archipelago {
                tracing::debug!(
                    "⚠️  Archipelago URL detected but ignored due to DISABLE_ARCHIPELAGO flag: {}",
                    a
                );
                return None;
            }
            // Strip "archipelago:" prefix, keep the rest as the adapter value
            Some(archipelago_url.to_string())
        } else if a.starts_with("fixed-adapter:") {
            // World about endpoints return adapter = "fixed-adapter:signed-login:..."
            Some(a.to_string())
        } else {
            None
        }
    } else {
        None
    };

    // Strip "fixed-adapter:" prefix if present
    raw.map(|s| s.strip_prefix("fixed-adapter:").unwrap_or(&s).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================
    // Tests for parse_comms_adapter_value
    // ==========================================

    #[test]
    fn test_fixed_adapter_direct_livekit() {
        // Preview scenes use fixedAdapter directly
        let result = parse_comms_adapter_value(
            Some("livekit:wss://preview.decentraland.org/rooms/test"),
            None,
            false,
            true,
        );
        assert_eq!(
            result,
            Some("livekit:wss://preview.decentraland.org/rooms/test".to_string())
        );
    }

    #[test]
    fn test_fixed_adapter_with_prefix() {
        // fixedAdapter with "fixed-adapter:" prefix should be stripped
        let result = parse_comms_adapter_value(
            Some("fixed-adapter:signed-login:https://worlds.decentraland.org/comms"),
            None,
            false,
            true,
        );
        assert_eq!(
            result,
            Some("signed-login:https://worlds.decentraland.org/comms".to_string())
        );
    }

    #[test]
    fn test_fixed_adapter_takes_priority_over_adapter() {
        let result = parse_comms_adapter_value(
            Some("livekit:wss://preview.decentraland.org/rooms/test"),
            Some("archipelago:archipelago:wss://archipelago.decentraland.org/ws"),
            false,
            true,
        );
        assert_eq!(
            result,
            Some("livekit:wss://preview.decentraland.org/rooms/test".to_string())
        );
    }

    #[test]
    fn test_adapter_archipelago_genesis_city() {
        // Genesis City: adapter = "archipelago:archipelago:wss://..."
        let result = parse_comms_adapter_value(
            None,
            Some("archipelago:archipelago:wss://archipelago-ws-connector.decentraland.org/ws"),
            false,
            true,
        );
        assert_eq!(
            result,
            Some("archipelago:wss://archipelago-ws-connector.decentraland.org/ws".to_string())
        );
    }

    #[test]
    fn test_adapter_archipelago_disabled() {
        let result = parse_comms_adapter_value(
            None,
            Some("archipelago:archipelago:wss://archipelago.decentraland.org/ws"),
            true, // disable_archipelago
            true,
        );
        assert_eq!(result, None);
    }

    #[test]
    fn test_adapter_archipelago_no_livekit() {
        let result = parse_comms_adapter_value(
            None,
            Some("archipelago:archipelago:wss://archipelago.decentraland.org/ws"),
            false,
            false, // livekit not enabled
        );
        assert_eq!(result, None);
    }

    // ==========================================
    // BUG REPRODUCTION: Issue #1719
    // World adapter "fixed-adapter:signed-login:..." in the `adapter` field
    // is not recognized, causing commsAdapter to remain empty.
    // ==========================================

    #[test]
    fn test_world_adapter_fixed_adapter_signed_login() {
        // World about returns: adapter = "fixed-adapter:signed-login:https://worlds-content-server..."
        // This MUST be recognized and stripped to "signed-login:https://..."
        let result = parse_comms_adapter_value(
            None,
            Some("fixed-adapter:signed-login:https://worlds-content-server.decentraland.org/worlds/aesironline.dcl.eth/comms"),
            false,
            true,
        );
        assert_eq!(
            result,
            Some("signed-login:https://worlds-content-server.decentraland.org/worlds/aesironline.dcl.eth/comms".to_string())
        );
    }

    #[test]
    fn test_world_adapter_fixed_adapter_livekit() {
        // Some worlds might return: adapter = "fixed-adapter:livekit:wss://..."
        let result = parse_comms_adapter_value(
            None,
            Some("fixed-adapter:livekit:wss://livekit.decentraland.org/rooms/world-room"),
            false,
            true,
        );
        assert_eq!(
            result,
            Some("livekit:wss://livekit.decentraland.org/rooms/world-room".to_string())
        );
    }

    #[test]
    fn test_no_comms_data() {
        let result = parse_comms_adapter_value(None, None, false, true);
        assert_eq!(result, None);
    }

    #[test]
    fn test_adapter_unknown_protocol_ignored() {
        // Unknown adapter values (not archipelago, not fixed-adapter) should be None
        let result = parse_comms_adapter_value(None, Some("unknown:something"), false, true);
        assert_eq!(result, None);
    }
}
