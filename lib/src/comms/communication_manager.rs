use ethers_core::types::H160;
use godot::prelude::*;
use http::Uri;
#[cfg(feature = "use_livekit")]
use std::sync::Arc;
use std::time::Instant;

use crate::{
    comms::{adapter::{movement_compressed::MoveKind, ws_room::WebSocketRoom, message_processor::MessageProcessor}, signed_login::SignedLoginMeta},
    dcl::components::proto_components::kernel::comms::rfc4,
    godot_classes::dcl_global::DclGlobal,
    auth::wallet,
    scene_runner::tokio_runtime::TokioRuntime,
};
use tokio::sync::mpsc;
use serde::{Deserialize, Serialize};

use super::{
    adapter::adapter_trait::Adapter,
    signed_login::{SignedLogin, SignedLoginPollStatus},
};

use crate::comms::adapter::movement_compressed::{MovementCompressed, Temporal, Movement};

const GATEKEEPER_URL: &str = "https://comms-gatekeeper.decentraland.org/get-scene-adapter";

// Temporary flags for testing different connection scenarios
const DISABLE_ARCHIPELAGO: bool = false;
const DISABLE_SCENE_ROOM: bool = true;

#[derive(Serialize, Deserialize)]
pub struct GatekeeperResponse {
    adapter: String,
}

#[derive(Debug)]
pub struct SceneRoomConnectionRequest {
    pub scene_id: String,
    pub livekit_url: String,
}

#[allow(clippy::large_enum_variant)]
enum MainRoom {
    WebSocket(WebSocketRoom),
    #[cfg(feature = "use_livekit")]
    LiveKit(LivekitRoom),
}

impl MainRoom {
    fn poll(&mut self) {
        match self {
            MainRoom::WebSocket(ws_room) => {
                ws_room.poll();
            }
            #[cfg(feature = "use_livekit")]
            MainRoom::LiveKit(livekit_room) => {
                livekit_room.poll();
            }
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
use crate::{comms::adapter::{archipelago::ArchipelagoManager, livekit::LivekitRoom}, dcl::SceneId, http_request::http_queue_requester::HttpQueueRequester};

#[allow(clippy::large_enum_variant)]
enum CommsConnection {
    None,
    WaitingForIdentity(String),
    SignedLogin(SignedLogin),
    #[cfg(feature = "use_livekit")]
    Archipelago(ArchipelagoManager),
    Connected(Box<dyn Adapter>),
}

#[derive(GodotClass)]
#[class(base=Node)]
pub struct CommunicationManager {
    current_connection: CommsConnection,
    current_connection_str: GString,
    last_position_broadcast_index: u64,
    voice_chat_enabled: bool,
    start_time: Instant,
    
    // Shared message processor for all adapters
    message_processor: Option<MessageProcessor>,
    
    // Store active rooms
    main_room: Option<MainRoom>,
    #[cfg(feature = "use_livekit")]
    scene_room: Option<LivekitRoom>,
    current_scene_id: Option<GString>,

    // Channel for scene room connection requests from async tasks
    #[cfg(feature = "use_livekit")]
    scene_room_connection_receiver: mpsc::Receiver<SceneRoomConnectionRequest>,
    #[cfg(feature = "use_livekit")]
    scene_room_connection_sender: mpsc::Sender<SceneRoomConnectionRequest>,

    base: Base<Node>,
}

#[godot_api]
impl INode for CommunicationManager {
    fn init(base: Base<Node>) -> Self {
        #[cfg(feature = "use_livekit")]
        let (scene_room_connection_sender, scene_room_connection_receiver) = mpsc::channel(10);
        
        CommunicationManager {
            current_connection: CommsConnection::None,
            current_connection_str: GString::default(),
            last_position_broadcast_index: 0,
            voice_chat_enabled: false,
            start_time: Instant::now(),
            message_processor: None,
            main_room: None,
            #[cfg(feature = "use_livekit")]
            scene_room: None,
            current_scene_id: None,
            #[cfg(feature = "use_livekit")]
            scene_room_connection_receiver,
            #[cfg(feature = "use_livekit")]
            scene_room_connection_sender,
            base,
        }
    }

    fn ready(&mut self) {
        self.base_mut().call_deferred("init_rs".into(), &[]);
    }

    fn process(&mut self, _dt: f64) {
        // Handle scene room connection requests from async tasks
        #[cfg(feature = "use_livekit")]
        while let Ok(request) = self.scene_room_connection_receiver.try_recv() {
            self.handle_scene_room_connection_request(request);
        }

        match &mut self.current_connection {
            CommsConnection::None => {}
            CommsConnection::WaitingForIdentity(adapter_url) => {
                let player_identity = DclGlobal::singleton().bind().get_player_identity();

                if player_identity.bind().try_get_address().is_some() {
                    let var = adapter_url.to_variant();
                    self.base_mut()
                        .call_deferred("change_adapter".into(), &[var]);
                }
            }
            CommsConnection::SignedLogin(signed_login) => match signed_login.poll() {
                SignedLoginPollStatus::Pending => {}
                SignedLoginPollStatus::Complete(response) => {
                    self.change_adapter(response.fixed_adapter.unwrap_or("offline".into()).into());
                }
                SignedLoginPollStatus::Error(e) => {
                    tracing::info!("Error in signed login: {:?}", e);
                    self.current_connection = CommsConnection::None;
                }
            },
            #[cfg(feature = "use_livekit")]
            CommsConnection::Archipelago(archipelago) => {
                archipelago.poll();
                let chats = archipelago.consume_chats();

                if !chats.is_empty() {
                    let chats_variant_array = get_chat_array(chats);
                    self.base_mut().emit_signal(
                        "chat_message".into(),
                        &[chats_variant_array.to_variant()],
                    );
                }
            }
            CommsConnection::Connected(adapter) => {
                let adapter = adapter.as_mut();
                let adapter_polling_ok = adapter.poll();
                let chats = adapter.consume_chats();

                if !chats.is_empty() {
                    let chats_variant_array = get_chat_array(chats);
                    self.base_mut()
                        .emit_signal("chat_message".into(), &[chats_variant_array.to_variant()]);
                }

                if !adapter_polling_ok {
                    self.current_connection = CommsConnection::None;
                }
            }
        }

        // Poll the shared message processor (if active)
        if let Some(processor) = &mut self.message_processor {
            let processor_polling_ok = processor.poll();
            let chats = processor.consume_chats();

            if !chats.is_empty() {
                let chats_variant_array = get_chat_array(chats);
                self.base_mut()
                    .emit_signal("chat_message".into(), &[chats_variant_array.to_variant()]);
            }

            if !processor_polling_ok {
                // Reset the message processor if it fails
                self.message_processor = None;
            }
        }

        // Poll main room (if active)
        if let Some(main_room) = &mut self.main_room {
            main_room.poll();
        }
        
        // Poll scene room (if active)
        #[cfg(feature = "use_livekit")]
        if let Some(scene_room) = &mut self.scene_room {
            scene_room.poll();
        }
    }
}

impl CommunicationManager {
    fn create_fallback_connection(&mut self) {
        tracing::info!("🔧 Creating fallback MessageProcessor for scene room support");
        
        // Ensure we have a MessageProcessor for scene rooms to work
        let _ = self.ensure_message_processor();
        
        // Set voice chat to false since we don't have a main connection with voice support
        self.voice_chat_enabled = false;
        
        let voice_chat_enabled = self.voice_chat_enabled.to_variant();
        self.base_mut().emit_signal(
            "on_adapter_changed".into(),
            &[voice_chat_enabled, "fallback".to_variant()],
        );
        
        tracing::info!("✅ Fallback connection established - scene rooms will work");
    }

    fn ensure_message_processor(&mut self) -> mpsc::Sender<crate::comms::adapter::message_processor::IncomingMessage> {
        if self.message_processor.is_none() {
            let player_identity = DclGlobal::singleton().bind().get_player_identity();
            let player_identity_bind = player_identity.bind();
            let player_address = player_identity_bind.get_address();
            let player_profile = player_identity_bind.clone_profile();
            let avatar_scene = DclGlobal::singleton().bind().get_avatars();

            let processor = MessageProcessor::new(
                player_address,
                player_profile,
                avatar_scene,
            );
            let sender = processor.get_message_sender();
            self.message_processor = Some(processor);
            sender
        } else {
            self.message_processor.as_ref().unwrap().get_message_sender()
        }
    }

    pub fn send_scene_message(&mut self, scene_id: String, data: Vec<u8>) {
        let scene_message = rfc4::Packet {
            message: Some(rfc4::packet::Message::Scene(rfc4::Scene { scene_id, data })),
            protocol_version: 100,
        };
        // Send to main room if available
        if let Some(main_room) = &mut self.main_room {
            main_room.send_rfc4(scene_message.clone(), true);
        }
        
        // Also send to scene room if available
        #[cfg(feature = "use_livekit")]
        if let Some(scene_room) = &mut self.scene_room {
            scene_room.send_rfc4(scene_message, true);
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
                #[cfg(feature = "use_livekit")]
                CommsConnection::Archipelago(archipelago) => {
                    archipelago.consume_scene_messages(scene_id)
                }
                _ => vec![],
            }
        }
    }
}

#[godot_api]
impl CommunicationManager {
    #[signal]
    fn chat_message(chats: VariantArray) {}

    #[signal]
    fn on_adapter_changed(voice_chat_enabled: bool, new_adapter: GString) {}

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
    fn broadcast_movement(&mut self, compressed: bool, position: Vector3, rotation_y: f32, velocity: Vector3, walk: bool, run: bool, jog: bool, rise: bool, fall: bool, land: bool) -> bool {
        // Update archipelago position if connected via archipelago
        if let CommsConnection::Archipelago(archipelago) = &mut self.current_connection {
            archipelago.update_position(position);
        }
        
        let rotation_y = rotation_y.to_degrees();

        let get_packet = || {
            if compressed {
                // Create MovementCompressed packet using the pattern from the other engine
                
                // Get elapsed time since start
                let time = self.start_time.elapsed().as_secs_f64();
                
                // Get realm bounds - using default values for now, you may want to get actual bounds
                let realm_bounds = (godot::prelude::Vector2i::new(-150, -150), godot::prelude::Vector2i::new(150, 150));
                
                let movement = Movement::new(
                    position,
                    velocity,
                    realm_bounds.0,
                    realm_bounds.1,
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
                
                // For temporal data, we need to determine these values based on game state
                let temporal = Temporal::from_parts(
                    time,
                    false, // is_emote - determine from game state
                    rotation_y,
                    movement.velocity_tier(),
                    move_kind,
                    !fall && !rise, // is_grounded - not grounded if falling or rising
                );
                
                let movement_compressed = MovementCompressed { temporal, movement };

                let movement_packet = rfc4::MovementCompressed {
                    temporal_data: i32::from_le_bytes(movement_compressed.temporal.into_bytes()),
                    movement_data: i64::from_le_bytes(movement_compressed.movement.into_bytes()),
                };
                
                rfc4::Packet {
                    message: Some(rfc4::packet::Message::MovementCompressed(
                        movement_packet
                    )),
                    protocol_version: 100,
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
                    rotation_y: -rotation_y,
                    movement_blend_value,
                    slide_blend_value: 0.0,
                    is_grounded: land,
                    is_jumping: rise,
                    is_long_jump: false,
                    is_long_fall: false,
                    is_falling: fall,
                    is_stunned: false,
                };

                //tracing::info!("Movement packet: {:?}", movement_packet);

                rfc4::Packet {
                    message: Some(rfc4::packet::Message::Movement(movement_packet)),
                    protocol_version: 100,
                }
            }
        };

        // Send to main room if available
        let mut message_sent = if let Some(main_room) = &mut self.main_room {
            main_room.send_rfc4(get_packet(), true)
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

        if message_sent {
            self.last_position_broadcast_index += 1;
        }
        message_sent
    }

    #[func]
    fn broadcast_position_and_rotation(&mut self, position: Vector3, rotation: Quaternion) -> bool {
        // Update archipelago position if connected via archipelago
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
                protocol_version: 100,
            }
        };

        // Send to main room if available
        let mut message_sent = if let Some(main_room) = &mut self.main_room {
            main_room.send_rfc4(get_packet(), true)
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

        if message_sent {
            self.last_position_broadcast_index += 1;
        }
        message_sent
    }

    #[func]
    fn send_chat(&mut self, text: GString) -> bool {
        let packet = rfc4::Packet {
            message: Some(rfc4::packet::Message::Chat(rfc4::Chat {
                message: text.to_string(),
                timestamp: self.start_time.elapsed().as_secs_f64(),
            })),
            protocol_version: 100,
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
    fn init_rs(&mut self) {
        DclGlobal::singleton().bind().get_realm().connect(
            "realm_changed".into(),
            self.base().callable("_on_realm_changed"),
        );

        let mut player_identity = DclGlobal::singleton().bind().get_player_identity();
        player_identity.connect(
            "profile_changed".into(),
            self.base().callable("_on_profile_changed"),
        );

        let mut scene_runner = DclGlobal::singleton().bind().get_scene_runner();
        scene_runner.connect(
            "on_change_scene_id".into(),
            self.base().callable("_on_change_scene_id"),
        );
    }

    #[func]
    fn _on_profile_changed(&mut self, _: Variant) {
        self.base_mut()
            .call_deferred("_on_update_profile".into(), &[]);
    }

    #[func]
    fn _on_realm_changed(&mut self) {
        self.base_mut()
            .call_deferred("_on_realm_changed_deferred".into(), &[]);
    }

    fn _internal_get_comms_from_realm(&self) -> Option<(String, Option<GString>)> {
        let realm = DclGlobal::singleton().bind().get_realm();
        let realm_about = Dictionary::from_variant(&realm.get("realm_about".into()));
        let comms = Dictionary::from_variant(&realm_about.get(StringName::from("comms"))?);
        let comms_protocol = String::from_variant(&comms.get(StringName::from("protocol"))?);

        let comms_fixed_adapter = if comms.contains_key("fixedAdapter") {
            comms
                .get(StringName::from("fixedAdapter"))
                .map(|v| GString::from_variant(&v))
        } else if comms.contains_key("adapter") {
            if let Some(temp) = comms
                .get(StringName::from("adapter"))
                .map(|v| GString::from_variant(&v).to_string())
            {
                if temp.starts_with("fixed-adapter:") {
                    Some(temp.replace("fixed-adapter:", "").into())
                } else if temp.starts_with("archipelago:") {
                    if DISABLE_ARCHIPELAGO {
                        tracing::info!("⚠️  Archipelago URL detected but ignored due to DISABLE_ARCHIPELAGO flag: {}", temp);
                        None
                    } else {
                        Some(temp.to_string()[12..].into())
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        Some((comms_protocol, comms_fixed_adapter))
    }

    #[func]
    fn _on_realm_changed_deferred(&mut self) {
        self.clean();

        let comms = self._internal_get_comms_from_realm();
        if comms.is_none() {
            tracing::info!("invalid comms from realm.");
            return;
        }

        let (comms_protocol, comms_fixed_adapter) = comms.unwrap();
        if comms_protocol != "v3" {
            tracing::info!("Only protocol 'v3' is supported.");
            return;
        }

        if comms_fixed_adapter.is_none() {
            if DISABLE_ARCHIPELAGO {
                // When archipelago is disabled, fall back to a direct LiveKit connection
                tracing::info!("🔄 Archipelago disabled, attempting fallback to direct LiveKit connection");
                // Try to create a direct LiveKit connection as fallback
                self.create_fallback_connection();
            } else {
                tracing::info!("As far, only fixedAdapter is supported.");
            }
            return;
        }

        let comms_fixed_adapter_str = comms_fixed_adapter.unwrap().to_string();
        self.change_adapter(comms_fixed_adapter_str.into());
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
        let avatar_scene = DclGlobal::singleton().bind().get_avatars();

        tracing::info!("change_adapter to protocol {protocol} and address {comms_address}");

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
                    .get("realm_url".into())
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
                let mut livekit_room = LivekitRoom::new(
                    comms_address.to_string(),
                    format!("livekit-{}", comms_address),
                );
                livekit_room.set_message_processor_sender(processor_sender);
                
                // Store the room - no need to change connection type
                self.main_room = Some(MainRoom::LiveKit(livekit_room));
            }

            #[cfg(not(feature = "use_livekit"))]
            "livekit" => {
                tracing::error!("livekit wasn't included in this build");
            }

            "offline" => {
                tracing::info!("set offline");
            }
            #[cfg(feature = "use_livekit")]
            "archipelago" => {
                if DISABLE_ARCHIPELAGO {
                    tracing::info!("⚠️  Archipelago connections are disabled (DISABLE_ARCHIPELAGO = true)");
                } else {
                    self.current_connection = CommsConnection::Archipelago(ArchipelagoManager::new(
                        comms_address,
                        current_ephemeral_auth_chain.clone(),
                        player_profile,
                        avatar_scene,
                    ));
                }
            }
            _ => {
                tracing::info!("unknown adapter {:?}", protocol);
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
                        true // Archipelago inherently supports voice when LiveKit is available
                    }
                }
                _ => false,
            }
        };

        let voice_chat_enabled = self.voice_chat_enabled.to_variant();
        self.base_mut().emit_signal(
            "on_adapter_changed".into(),
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
        }
        self.current_scene_id = None;
        self.current_connection = CommsConnection::None;
        self.current_connection_str = GString::default();
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
        match &mut self.current_connection {
            CommsConnection::Connected(adapter) => adapter.change_profile(player_profile),
            #[cfg(feature = "use_livekit")]
            CommsConnection::Archipelago(archipelago) => archipelago.change_profile(player_profile),
            _ => {}
        }
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
        GString::from(self.current_connection_str.clone())
    }

    
    #[cfg(feature = "use_livekit")]
    fn handle_scene_room_connection_request(&mut self, request: SceneRoomConnectionRequest) {
        tracing::info!("🔌 Processing scene room connection request for scene '{}' with URL: {}", request.scene_id, request.livekit_url);
        
        // Try to create scene room using shared message processor
        if self.message_processor.is_some() {
            // Clean up existing scene room
            if let Some(scene_room) = &mut self.scene_room {
                tracing::info!("🧹 Cleaning up existing scene room");
                scene_room.clean();
            }
            
            // Create new LiveKit room for the scene
            let room_id = format!("scene-{}", request.scene_id);
            tracing::info!("🚀 Creating new scene room with ID: {}", room_id);
            
            let mut scene_room = LivekitRoom::new(request.livekit_url.clone(), room_id);
            
            // Connect the scene room to the shared message processor
            let processor_sender = self.message_processor.as_ref().unwrap().get_message_sender();
            scene_room.set_message_processor_sender(processor_sender);
            
            self.scene_room = Some(scene_room);
            
            // Check if we're in fallback mode (no main room)
            if self.main_room.is_none() && matches!(&self.current_connection, CommsConnection::None) {
                tracing::info!("✅ Scene room successfully created and connected to fallback message processor (archipelago disabled)");
            } else {
                tracing::info!("✅ Scene room successfully created and connected to shared message processor");
            }
        } else {
            // Check if archipelago has a message processor we can use
            if let CommsConnection::Archipelago(archipelago) = &mut self.current_connection {
                if let Some(processor_sender) = archipelago.get_message_processor_sender() {
                    // Clean up existing scene room
                    if let Some(scene_room) = &mut self.scene_room {
                        tracing::info!("🧹 Cleaning up existing scene room");
                        scene_room.clean();
                    }
                    
                    // Create new LiveKit room for the scene
                    let room_id = format!("scene-{}", request.scene_id);
                    tracing::info!("🚀 Creating new scene room with ID: {} (archipelago mode)", room_id);
                    
                    let mut scene_room = LivekitRoom::new(request.livekit_url.clone(), room_id);
                    scene_room.set_message_processor_sender(processor_sender);
                    self.scene_room = Some(scene_room);
                    tracing::info!("✅ Scene room successfully created and connected to archipelago message processor");
                } else {
                    tracing::warn!("⚠️  Cannot create scene room: Archipelago message processor not ready");
                }
            } else {
                tracing::info!("📝 No message processor available yet, scene room will be created when connection is established");
            }
        }
    }

    #[cfg(feature = "use_livekit")]
    #[func]
    pub fn _on_change_scene_id(&mut self, scene_id: i32) {
        use std::sync::Arc;

        use crate::http_request::http_queue_requester::HttpQueueRequester;

        let scene_runner = DclGlobal::singleton().bind().get_scene_runner();
        let scene_entity_id = scene_runner.bind().get_scene_entity_id(scene_id);
        
        // Check if scene has actually changed
        if let Some(current_scene) = &self.current_scene_id {
            if current_scene == &scene_entity_id {
                return; // No change needed
            }
        }
        
        tracing::info!("Scene changed to: {}", scene_entity_id);
        
        // Clean up existing scene room
        if let Some(scene_room) = &mut self.scene_room {
            scene_room.clean();
        }
        self.scene_room = None;
        self.current_scene_id = Some(scene_entity_id.clone());
        
        // Check if scene rooms are disabled
        if DISABLE_SCENE_ROOM {
            tracing::info!("⚠️  Scene room connections are disabled (DISABLE_SCENE_ROOM = true)");
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
        let realm_name = DclGlobal::singleton().bind().get_realm().get("realm_name".into()).to_string();
        let http_requester: Arc<HttpQueueRequester> = DclGlobal::singleton()
            .bind()
            .get_http_requester()
            .bind()
            .get_http_queue_requester();
        let connection_sender = self.scene_room_connection_sender.clone();
        TokioRuntime::spawn(async move {
            tracing::info!("Requesting scene adapter for scene: {}", scene_entity_id);
            match get_scene_adapter(http_requester, &scene_entity_id, &realm_name, &ephemeral_auth_chain).await {
                Ok(adapter_url) => {
                    tracing::info!("✅ Got scene adapter URL for scene '{}': {}", scene_entity_id, adapter_url);
                    
                    // Parse the adapter URL to extract LiveKit connection details
                    if adapter_url.starts_with("livekit:") {
                        // Extract the actual LiveKit URL after "livekit:"
                        let livekit_url = &adapter_url[8..]; // Remove "livekit:" prefix
                        tracing::info!("🔗 Preparing to connect scene room to LiveKit: {}", livekit_url);
                        
                        // Send connection request to main thread via channel
                        let request = SceneRoomConnectionRequest {
                            scene_id: scene_entity_id.clone(),
                            livekit_url: livekit_url.to_string(),
                        };
                        
                        match connection_sender.send(request).await {
                            Ok(()) => {
                                tracing::info!("📤 Scene room connection request sent to main thread for scene '{}'", scene_entity_id);
                            },
                            Err(e) => {
                                tracing::error!("❌ Failed to send scene room connection request: {}", e);
                            }
                        }
                    } else {
                        tracing::warn!("⚠️  Unsupported scene adapter type: {}", adapter_url);
                    }
                },
                Err(e) => {
                    tracing::error!("❌ Failed to get scene adapter for scene '{}': {}", scene_entity_id, e);
                }
            }
        });
    }
    
}

#[cfg(feature = "use_livekit")]
async fn get_scene_adapter(
    http_requester: Arc<HttpQueueRequester>,
    scene_id: &str,
    realm_name: &str,
    ephemeral_auth_chain: &crate::auth::ephemeral_auth_chain::EphemeralAuthChain,
) -> Result<String, String> {
    
    // Create the request body

    use crate::http_request::request_response::{RequestOption, ResponseEnum, ResponseType};
    let request_body = serde_json::json!({
        "sceneId": scene_id,
        "realmName": realm_name
    });
    let metadata_json_string = request_body.to_string();
    
    tracing::info!("🔄 Making scene adapter request to: {}", GATEKEEPER_URL);
    tracing::info!("📋 Request body: {}", metadata_json_string);
    
    // Create URI
    let uri = http::Uri::from_static(GATEKEEPER_URL);
    let method = http::Method::POST;
    
    // Sign the request
    tracing::info!("🔐 Signing request with ephemeral auth chain");
    let headers = wallet::sign_request(
        method.as_str(),
        &uri,
        ephemeral_auth_chain,
        request_body,  // Pass the serde_json::Value directly, not the string
    )
    .await;
    
    tracing::info!("📝 Generated {} authentication headers", headers.len());

    let request_option = RequestOption::new(
        0,
        uri.to_string(),
        method,
        ResponseType::AsJson,
        Some(metadata_json_string.as_bytes().to_vec()),
        Some(headers.into_iter().collect()),
        None,
    );
    
    let response = http_requester.request(request_option, 0).await
        .map_err(|e| format!("Request failed: {}", e.error_message))?;

    tracing::info!("📡 Received HTTP response with status: {}", response.status_code);

    if !response.status_code.is_success() {
        tracing::error!("❌ HTTP request failed with status: {}", response.status_code);
        return Err(format!("HTTP error: {}", response.status_code));
    }

    // Extract response data
    let response_data = response.response_data
        .map_err(|e| format!("Response data error: {}", e))?;

    // Parse the response based on type
    tracing::info!("🔍 Parsing gatekeeper response");
    let gatekeeper_response: GatekeeperResponse = match response_data {
        ResponseEnum::String(text) => {
            tracing::info!("📄 Response as string: {}", text);
            serde_json::from_str(&text).map_err(|e| format!("JSON parse error: {}", e))?
        },
        ResponseEnum::Json(json_result) => {
            let json_value = json_result.map_err(|e| format!("JSON result error: {}", e))?; // Extract the Result first
            tracing::info!("📊 Response as JSON: {}", json_value);
            serde_json::from_value(json_value).map_err(|e| format!("JSON value parse error: {}", e))?
        },
        ResponseEnum::Bytes(bytes) => {
            let text = String::from_utf8(bytes)
                .map_err(|e| format!("Invalid UTF-8: {}", e))?;
            tracing::info!("📄 Response as bytes->string: {}", text);
            serde_json::from_str(&text).map_err(|e| format!("JSON parse error: {}", e))?
        },
        _ => return Err("Unexpected response type".to_string()),
    };

    tracing::info!("✅ Successfully parsed gatekeeper response: adapter = '{}'", gatekeeper_response.adapter);
    Ok(gatekeeper_response.adapter)
}

fn get_chat_array(chats: Vec<(H160, rfc4::Chat)>) -> VariantArray {
    let mut chats_variant_array = VariantArray::new();
    for (address, chat) in chats {
        let mut chat_arr = VariantArray::new();
        let address = format!("{:#x}", address);
        chat_arr.push(address.to_variant());
        chat_arr.push(chat.timestamp.to_variant());
        chat_arr.push(chat.message.to_variant());

        chats_variant_array.push(chat_arr.to_variant());
    }
    chats_variant_array
}
