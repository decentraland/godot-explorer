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

#[derive(Debug)]
struct Peer {
    alias: u32,
    profile: Option<UserProfile>,
    announced_version: Option<u32>,
    protocol_version: u32,
    last_activity: Instant,
}

struct ProfileUpdate {
    address: H160,
    peer_alias: u32,
    profile: UserProfile,
}

pub struct MessageProcessor {
    // Message channel for receiving messages from multiple rooms
    message_receiver: mpsc::Receiver<IncomingMessage>,
    message_sender: mpsc::Sender<IncomingMessage>,
    
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
        let (profile_update_sender, profile_update_receiver) = mpsc::channel(100);
        
        Self {
            message_receiver,
            message_sender,
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

        if !inactive_peers.is_empty() {
            let mut avatar_scene_ref = self.avatars.clone();
            let mut avatar_scene = avatar_scene_ref.bind_mut();
            
            for address in inactive_peers {
                if let Some(peer) = self.peer_identities.remove(&address) {
                    tracing::info!("Removing inactive avatar {:#x} (alias: {})", address, peer.alias);
                    avatar_scene.remove_avatar(peer.alias);
                }
            }
        }
        
        // Periodic profile requests
        if self.last_profile_request_sent.elapsed().as_secs_f32() > 10.0 {
            self.last_profile_request_sent = Instant::now();
            // TODO: Implement profile request broadcasting to all rooms
        }
        
        true
    }
    
    fn process_message(&mut self, message: IncomingMessage) {
        // Handle peer creation/updates first
        let peer_alias = if let Some(peer) = self.peer_identities.get_mut(&message.address) {
            // Update existing peer
            if let MessageType::Rfc4(rfc4_msg) = &message.message {
                peer.protocol_version = rfc4_msg.protocol_version;
            }
            peer.last_activity = Instant::now();
            peer.alias
        } else {
            // Check if there's an existing peer with the same address (reconnection case)
            if let Some(existing_peer) = self.peer_identities.remove(&message.address) {
                tracing::info!("Removing old peer {:#x} (alias: {}) due to reconnection", message.address, existing_peer.alias);
                
                // Brief borrow to remove old avatar
                {
                    let mut avatar_scene_ref = self.avatars.clone();
                    let mut avatar_scene = avatar_scene_ref.bind_mut();
                    avatar_scene.remove_avatar(existing_peer.alias);
                }
            }

            self.peer_alias_counter += 1;
            let new_alias = self.peer_alias_counter;
            
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
                self.handle_rfc4_message(rfc4_msg.message.clone(), peer_alias, message.address);
            }
        }
    }
    
    fn handle_rfc4_message(
        &mut self,
        message: rfc4::packet::Message,
        peer_alias: u32,
        address: H160,
    ) {
        match message {
            rfc4::packet::Message::Position(position) => {
                tracing::info!(
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
                tracing::info!(
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

                tracing::info!(
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
                tracing::info!(
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

                    // Fetch from lambda server
                    tracing::info!(
                        "comms > requesting profile from lambda for {:#x}",
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
                tracing::info!(
                    "Received ProfileRequest from {:#x}: {:?}",
                    address,
                    profile_request
                );

                // TODO: Respond with profile if it's for our player
                // This will need to communicate back to the originating room
            }
            rfc4::packet::Message::ProfileResponse(profile_response) => {
                tracing::info!(
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
        // TODO: Broadcast profile version to all rooms
    }
    
    pub fn clean(&mut self) {
        self.peer_identities.clear();
    }
}