use std::{collections::HashMap, sync::Arc, time::Instant};

use ethers_core::types::H160;
use godot::prelude::{GString, Gd};
use tokio::sync::mpsc;

use crate::{
    avatars::avatar_scene::AvatarScene,
    comms::profile::{SerializedProfile, UserProfile},
    content::profile::prepare_request_requirements,
    dcl::components::proto_components::kernel::comms::rfc4,
    scene_runner::tokio_runtime::TokioRuntime,
    content::profile::request_lambda_profile,
};

use super::movement_compressed::MovementCompressed;

#[derive(Debug, Clone)]
pub struct IncomingMessage {
    pub message: MessageType,
    pub address: H160,
    pub room_id: String, // To identify which room the message came from
}

#[derive(Debug, Clone)]
pub enum MessageType {
    Rfc4(Rfc4Message),
    InitVoice(VoiceInitData),
    VoiceFrame(VoiceFrameData),
    PeerJoined,     // Peer joined a room
    PeerLeft,       // Peer left a room
}

#[derive(Debug, Clone)]
pub struct Rfc4Message {
    pub message: rfc4::packet::Message,
    pub protocol_version: u32,
}

#[derive(Debug, Clone)]
pub struct VoiceInitData {
    pub sample_rate: u32,
    pub num_channels: u32,
    pub samples_per_channel: u32,
}

#[derive(Debug, Clone)]
pub struct VoiceFrameData {
    pub data: Vec<i16>,
}

#[derive(Debug, Clone)]
pub struct OutgoingMessage {
    pub packet: rfc4::Packet,
    pub unreliable: bool,
    pub target_room: Option<String>, // None means broadcast to all rooms
}

#[derive(Debug)]
struct Peer {
    alias: u32,
    profile: Option<UserProfile>,
    announced_version: Option<u32>,
    protocol_version: u32,
    last_activity: Instant,
    room_activity: HashMap<String, Instant>,  // Track last activity per room
}

struct ProfileUpdate {
    address: H160,
    peer_alias: u32,
    profile: UserProfile,
}

pub struct MessageProcessor {
    // Message channels for receiving messages from multiple rooms
    message_receiver: mpsc::Receiver<IncomingMessage>,
    message_sender: mpsc::Sender<IncomingMessage>,
    
    // Outgoing message channel for sending responses back to rooms
    outgoing_receiver: mpsc::Receiver<OutgoingMessage>,
    outgoing_sender: mpsc::Sender<OutgoingMessage>,
    
    // Avatar management
    avatars: Gd<AvatarScene>,
    peer_identities: HashMap<H160, Peer>,
    peer_alias_counter: u32,
    
    // Player info
    player_address: H160,
    player_profile: Option<UserProfile>,
    
    // Timing
    last_profile_request_sent: Instant,
    last_profile_response_sent: Instant,
    
    // Chat and scene messages
    chats: Vec<(H160, rfc4::Chat)>,
    incoming_scene_messages: HashMap<String, Vec<(H160, Vec<u8>)>>,
    
    // Profile updates from async tasks
    profile_update_receiver: mpsc::Receiver<ProfileUpdate>,
    profile_update_sender: mpsc::Sender<ProfileUpdate>,
}

impl MessageProcessor {
    pub fn new(
        player_address: H160,
        player_profile: Option<UserProfile>,
        avatars: Gd<AvatarScene>,
    ) -> Self {
        let (message_sender, message_receiver) = mpsc::channel(1000);
        let (outgoing_sender, outgoing_receiver) = mpsc::channel(1000);
        let (profile_update_sender, profile_update_receiver) = mpsc::channel(100);
        
        Self {
            message_receiver,
            message_sender,
            outgoing_receiver,
            outgoing_sender,
            avatars,
            peer_identities: HashMap::new(),
            peer_alias_counter: 0,
            player_address,
            player_profile,
            last_profile_request_sent: Instant::now(),
            last_profile_response_sent: Instant::now(),
            chats: Vec::new(),
            incoming_scene_messages: HashMap::new(),
            profile_update_receiver,
            profile_update_sender,
        }
    }
    
    /// Get a sender for rooms to send messages to this processor
    pub fn get_message_sender(&self) -> mpsc::Sender<IncomingMessage> {
        self.message_sender.clone()
    }
    
    /// Consume outgoing messages that need to be sent by rooms
    pub fn consume_outgoing_messages(&mut self) -> Vec<OutgoingMessage> {
        let mut messages = Vec::new();
        while let Ok(message) = self.outgoing_receiver.try_recv() {
            messages.push(message);
        }
        messages
    }
    
    /// Process all pending messages and return true if should continue
    pub fn poll(&mut self) -> bool {
        // Handle profile updates from async tasks
        while let Ok(update) = self.profile_update_receiver.try_recv() {
            tracing::warn!(
                "comms > received profile update for {:#x}: {:?}",
                update.address,
                update.profile
            );
            
            // Brief borrow scope for avatar update
            {
                let mut avatar_scene_ref = self.avatars.clone();
                let mut avatar_scene = avatar_scene_ref.bind_mut();
                avatar_scene.update_avatar_by_alias(update.peer_alias, &update.profile);
            }
            
            if let Some(peer) = self.peer_identities.get_mut(&update.address) {
                peer.profile = Some(update.profile);
            }
        }
        
        // Process incoming messages
        while let Ok(message) = self.message_receiver.try_recv() {
            self.process_message(message);
        }
        
        // Remove inactive avatars (only if inactive in ALL rooms)
        // With proper lifecycle events, we can use a longer timeout as a safety net
        let inactive_threshold = std::time::Duration::from_secs(5);
        let mut peers_to_update: Vec<(H160, Vec<String>)> = Vec::new();
        
        // First pass: identify which rooms are inactive for each peer
        for (address, peer) in self.peer_identities.iter_mut() {
            let mut inactive_rooms = Vec::new();
            
            // Check each room the peer has been seen in
            let rooms_to_check: Vec<String> = peer.room_activity.keys().cloned().collect();
            for room_id in rooms_to_check {
                if let Some(&last_seen) = peer.room_activity.get(&room_id) {
                    if last_seen.elapsed() > inactive_threshold {
                        inactive_rooms.push(room_id);
                    }
                }
            }
            
            if !inactive_rooms.is_empty() {
                peers_to_update.push((*address, inactive_rooms));
            }
        }
        
        // Second pass: remove inactive rooms and check if peer should be removed
        let mut peers_to_remove = Vec::new();
        for (address, inactive_rooms) in peers_to_update {
            if let Some(peer) = self.peer_identities.get_mut(&address) {
                // Remove inactive rooms
                for room in &inactive_rooms {
                    peer.room_activity.remove(room);
                    tracing::debug!("⏰ Peer {:#x} (alias: {}) timed out in room '{}' (safety cleanup)", 
                        address, peer.alias, room);
                }
                
                // If peer has no active rooms left AND has been inactive, remove it
                if peer.room_activity.is_empty() && peer.last_activity.elapsed() > inactive_threshold {
                    tracing::info!("⏰ Peer {:#x} (alias: {}) has no active rooms and timed out - removing", 
                        address, peer.alias);
                    peers_to_remove.push(address);
                }
            }
        }
        
        // Remove peers that have no active rooms and timed out
        if !peers_to_remove.is_empty() {
            let mut avatar_scene_ref = self.avatars.clone();
            let mut avatar_scene = avatar_scene_ref.bind_mut();
            
            for address in peers_to_remove {
                if let Some(peer) = self.peer_identities.remove(&address) {
                    tracing::info!("🗑️ Removed inactive peer {:#x} (alias: {})", 
                        address, peer.alias);
                    avatar_scene.remove_avatar(peer.alias);
                }
            }
        }
        
        // Periodic profile requests
        if self.last_profile_request_sent.elapsed().as_secs_f32() > 10.0 {
            self.last_profile_request_sent = Instant::now();
            // NOTE: ProfileVersion broadcasting is now handled at CommunicationManager level
        }
        
        true
    }
    
    fn process_message(&mut self, message: IncomingMessage) {
        let room_id = message.room_id.clone(); // Extract room_id for later use
        
        // Handle peer creation/updates first
        let peer_alias = if let Some(peer) = self.peer_identities.get_mut(&message.address) {
            // Update existing peer - check if this is from a new room
            if !peer.room_activity.contains_key(&message.room_id) {
                tracing::info!("📨 Existing peer {:#x} (alias: {}) now also seen in room '{}'", 
                    message.address, peer.alias, message.room_id);
            } else {
                tracing::debug!("📨 Message from {:#x} via room '{}' (existing peer, alias: {})", 
                    message.address, message.room_id, peer.alias);
            }
            
            // Update activity for this specific room
            peer.room_activity.insert(message.room_id.clone(), Instant::now());
            peer.last_activity = Instant::now();
            
            if let MessageType::Rfc4(rfc4_msg) = &message.message {
                peer.protocol_version = rfc4_msg.protocol_version;
            }
            peer.alias
        } else {
            // Create new peer only if it doesn't exist
            self.peer_alias_counter += 1;
            let new_alias = self.peer_alias_counter;
            
            tracing::info!("🆕 Creating new peer {:#x} from room '{}' with alias: {}", 
                message.address, message.room_id, new_alias);
            
            let mut room_activity = HashMap::new();
            room_activity.insert(message.room_id.clone(), Instant::now());
            
            self.peer_identities.insert(
                message.address,
                Peer {
                    alias: new_alias,
                    profile: None,
                    announced_version: None,
                    protocol_version: if let MessageType::Rfc4(rfc4_msg) = &message.message {
                        rfc4_msg.protocol_version
                    } else {
                        100
                    },
                    last_activity: Instant::now(),
                    room_activity,
                },
            );
            
            // Brief borrow to add new avatar
            {
                let mut avatar_scene_ref = self.avatars.clone();
                let mut avatar_scene = avatar_scene_ref.bind_mut();
                avatar_scene.add_avatar(
                    new_alias,
                    GString::from(format!("{:#x}", message.address)),
                );
            }

            // TODO: Send profile request to the room where this message came from
            new_alias
        };
        
        // Handle non-RFC4 messages that need avatar_scene
        match &message.message {
            MessageType::InitVoice(voice_init) => {
                let mut avatar_scene_ref = self.avatars.clone();
                let mut avatar_scene = avatar_scene_ref.bind_mut();
                avatar_scene.spawn_voice_channel(
                    peer_alias,
                    voice_init.sample_rate,
                    voice_init.num_channels,
                    voice_init.samples_per_channel,
                );
            }
            MessageType::VoiceFrame(voice_frame) => {
                // If all the frame.data is less than 10, we skip the frame
                if voice_frame.data.iter().all(|&c| c.abs() < 10) {
                    return;
                }

                let frame = godot::prelude::PackedVector2Array::from_iter(voice_frame.data.iter().map(|c| {
                    let val = (*c as f32) / (i16::MAX as f32);
                    godot::prelude::Vector2 { x: val, y: val }
                }));

                let mut avatar_scene_ref = self.avatars.clone();
                let mut avatar_scene = avatar_scene_ref.bind_mut();
                avatar_scene.push_voice_frame(peer_alias, frame);
            }
            MessageType::Rfc4(rfc4_msg) => {
                // Handle RFC4 messages
                self.handle_rfc4_message(rfc4_msg.message.clone(), peer_alias, message.address, room_id.clone());
            }
            MessageType::PeerJoined => {
                // Peer joined event - ensure peer exists and update room activity
                tracing::info!("👋 Peer {:#x} joined room '{}' (alias: {})", 
                    message.address, room_id, peer_alias);
            }
            MessageType::PeerLeft => {
                // Handle peer leaving a room
                self.handle_peer_left(message.address, room_id);
            }
        }
    }
    
    fn handle_peer_left(&mut self, address: H160, room_id: String) {
        if let Some(peer) = self.peer_identities.get_mut(&address) {
            peer.room_activity.remove(&room_id);
            tracing::info!("👋 Peer {:#x} (alias: {}) left room '{}'", 
                address, peer.alias, room_id);
            
            // If peer has no more active rooms, remove it
            if peer.room_activity.is_empty() {
                let alias = peer.alias;
                self.peer_identities.remove(&address);
                tracing::info!("🗑️  Removing peer {:#x} (alias: {}) - no longer in any rooms", 
                    address, alias);
                
                // Remove avatar
                let mut avatar_scene_ref = self.avatars.clone();
                let mut avatar_scene = avatar_scene_ref.bind_mut();
                avatar_scene.remove_avatar(alias);
            }
        }
    }
    
    fn handle_rfc4_message(
        &mut self,
        message: rfc4::packet::Message,
        peer_alias: u32,
        address: H160,
        room_id: String,
    ) {
        match message {
            rfc4::packet::Message::Position(position) => {
                tracing::debug!(
                    "Received Position from {:#x}: pos({}, {}, {}), rot({}, {}, {}, {})", 
                    address,
                    position.position_x, position.position_y, position.position_z,
                    position.rotation_x, position.rotation_y, position.rotation_z, position.rotation_w
                );
                
                // Let avatar_scene handle timestamp validation
                let mut avatar_scene_ref = self.avatars.clone();
                let mut avatar_scene = avatar_scene_ref.bind_mut();
                avatar_scene.update_avatar_transform_with_rfc4_position(peer_alias, &position);
            }
            rfc4::packet::Message::Movement(movement) => {
                tracing::debug!(
                    "Received Movement from {:#x}: timestamp({}) pos({}, {}, {}), rot_y({}), vel({}, {}, {}) blend({}), slide_blend({})", 
                    address,
                    movement.timestamp,
                    movement.position_x, movement.position_y, movement.position_z,
                    movement.rotation_y,
                    movement.velocity_x, movement.velocity_y, movement.velocity_z,
                    movement.movement_blend_value,
                    movement.slide_blend_value,
                );

                // Let avatar_scene handle timestamp validation
                let mut avatar_scene_ref = self.avatars.clone();
                let mut avatar_scene = avatar_scene_ref.bind_mut();
                avatar_scene.update_avatar_transform_with_movement(peer_alias, &movement);
            }
            rfc4::packet::Message::MovementCompressed(movement_compressed) => {
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

                tracing::debug!(
                    "Received MovementCompressed from {:#x}: pos({}, {}, {}), rot_rad({}), vel({}, {}, {}), timestamp({})", 
                    address,
                    pos.x, pos.y, -pos.z,
                    rotation_rad,
                    velocity.x, velocity.y, velocity.z,
                    timestamp
                );

                // Let avatar_scene handle timestamp validation
                let mut avatar_scene_ref = self.avatars.clone();
                let mut avatar_scene = avatar_scene_ref.bind_mut();
                avatar_scene.update_avatar_transform_with_movement_compressed(
                    peer_alias,
                    pos,
                    rotation_rad,
                    timestamp,
                );
            }
            rfc4::packet::Message::Chat(chat) => {
                tracing::info!("Received Chat from {:#x}: {:?}", address, chat);
                self.chats.push((address, chat));
            }
            rfc4::packet::Message::ProfileVersion(announce_profile_version) => {
                tracing::debug!(
                    "Received ProfileVersion from {:#x}: {:?}",
                    address,
                    announce_profile_version
                );

                let announced_version = announce_profile_version.profile_version;
                
                // Get current version and update peer
                let (current_version, peer_alias_for_async) = if let Some(peer) = self.peer_identities.get_mut(&address) {
                    let current_version = peer.profile.as_ref().map(|p| p.version).unwrap_or(0);
                    peer.announced_version = Some(announced_version);
                    (current_version, peer.alias)
                } else {
                    (0, peer_alias)
                };

                // If the announced version is newer than what we have, request the profile
                if announced_version > current_version {
                    tracing::info!(
                        "Requesting newer profile from {:#x}: announced={}, current={}",
                        address,
                        announced_version,
                        current_version
                    );

                    // First, try sending a ProfileRequest to the peer directly
                    let request_packet = rfc4::Packet {
                        message: Some(rfc4::packet::Message::ProfileRequest(rfc4::ProfileRequest {
                            address: format!("{:#x}", address),
                            profile_version: announced_version,
                        })),
                        protocol_version: 100,
                    };

                    let outgoing = OutgoingMessage {
                        packet: request_packet,
                        unreliable: false,
                        target_room: Some(room_id.clone()),
                    };

                    if let Err(e) = self.outgoing_sender.try_send(outgoing) {
                        tracing::warn!("Failed to queue ProfileRequest: {}", e);
                    } else {
                        tracing::info!("📤 Sending ProfileRequest to {:#x} via room '{}'", address, room_id);
                    }

                    // Also fetch from lambda server as fallback
                    tracing::info!(
                        "comms > also requesting profile from lambda for {:#x} as fallback",
                        address
                    );
                    
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
                                peer_alias: peer_alias_for_async,
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
            rfc4::packet::Message::ProfileRequest(profile_request) => {
                tracing::debug!(
                    "Received ProfileRequest from {:#x}: {:?}",
                    address,
                    profile_request
                );

                // Check if they're requesting our player's profile
                if let Some(requested_address) = profile_request.address.parse::<H160>().ok() {
                    if requested_address == self.player_address {
                        // They're requesting our profile - send ProfileResponse
                        if let Some(player_profile) = &self.player_profile {
                            let serialized_profile = serde_json::to_string(&player_profile.content)
                                .unwrap_or_else(|_| "{}".to_string());
                            
                            let response_packet = rfc4::Packet {
                                message: Some(rfc4::packet::Message::ProfileResponse(rfc4::ProfileResponse {
                                    serialized_profile,
                                    base_url: player_profile.base_url.clone(),
                                })),
                                protocol_version: 100,
                            };

                            // Send response back to the requesting room
                            let outgoing = OutgoingMessage {
                                packet: response_packet,
                                unreliable: false,
                                target_room: Some(room_id.clone()),
                            };

                            if let Err(e) = self.outgoing_sender.try_send(outgoing) {
                                tracing::warn!("Failed to queue ProfileResponse: {}", e);
                            } else {
                                tracing::info!("📤 Sending ProfileResponse to {:#x} via room '{}'", address, room_id);
                            }
                        } else {
                            tracing::warn!("ProfileRequest for our address but no profile available");
                        }
                    }
                } else {
                    tracing::warn!("Invalid address in ProfileRequest: {}", profile_request.address);
                }
            }
            rfc4::packet::Message::ProfileResponse(profile_response) => {
                tracing::debug!(
                    "Received ProfileResponse from {:#x}: {:?}",
                    address,
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
                            return;
                        }
                    };

                let incoming_version = serialized_profile.version as u32;
                
                // Check and update peer profile
                if let Some(peer) = self.peer_identities.get_mut(&address) {
                    let current_version = peer.profile.as_ref().map(|p| p.version).unwrap_or(0);

                    if incoming_version <= current_version {
                        return;
                    }

                    let profile = UserProfile {
                        version: incoming_version,
                        content: serialized_profile.clone(),
                        base_url: profile_response.base_url.clone(),
                    };

                    let mut avatar_scene_ref = self.avatars.clone();
                    let mut avatar_scene = avatar_scene_ref.bind_mut();
                    avatar_scene.update_avatar_by_alias(peer_alias, &profile);
                    peer.profile = Some(profile);
                }
            }
            rfc4::packet::Message::Scene(scene) => {
                let entry = self
                    .incoming_scene_messages
                    .entry(scene.scene_id)
                    .or_default();

                // TODO: should we limit the size of the queue or accumulated bytes?
                entry.push((address, scene.data));
            }
            rfc4::packet::Message::Voice(_voice) => {}
            _ => {
                tracing::debug!("comms > unhandled rfc4 message");
            }
        }
    }
    
    pub fn consume_chats(&mut self) -> Vec<(H160, rfc4::Chat)> {
        std::mem::take(&mut self.chats)
    }
    
    pub fn consume_scene_messages(&mut self, scene_id: &str) -> Vec<(H160, Vec<u8>)> {
        if let Some(messages) = self.incoming_scene_messages.get_mut(scene_id) {
            std::mem::take(messages)
        } else {
            Vec::new()
        }
    }
    
    pub fn change_profile(&mut self, new_profile: UserProfile) {
        self.player_profile = Some(new_profile);
        // NOTE: ProfileVersion broadcasting is now handled at CommunicationManager level
    }
    
    pub fn clean(&mut self) {
        self.peer_identities.clear();
    }
}