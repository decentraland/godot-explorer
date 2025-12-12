use std::{
    collections::{HashMap, HashSet, VecDeque},
    time::{Duration, Instant},
};

use ethers_core::types::H160;
use godot::prelude::{GString, Gd};
use std::cmp::Ordering;
use tokio::sync::mpsc;

use crate::{
    avatars::avatar_scene::AvatarScene,
    comms::{
        consts::{
            DEFAULT_PROTOCOL_VERSION, INACTIVE_PEER_THRESHOLD_SECS, MAX_CHAT_MESSAGES,
            MAX_CHAT_MESSAGE_SIZE, MAX_SCENE_IDS, MAX_SCENE_MESSAGES_PER_SCENE,
            MESSAGE_CHANNEL_SIZE, OUTGOING_CHANNEL_SIZE, PROFILE_REQUEST_INTERVAL_SECS,
            PROFILE_UPDATE_CHANNEL_SIZE,
        },
        profile::{SerializedProfile, UserProfile},
    },
    content::profile::{prepare_request_requirements, request_lambda_profile},
    dcl::components::proto_components::kernel::comms::rfc4,
    godot_classes::dcl_social_blacklist::DclSocialBlacklist,
    scene_runner::tokio_runtime::TokioRuntime,
};

use super::movement_compressed::MovementCompressed;

/// Represents an incoming message from a communication room
#[derive(Debug, Clone)]
pub struct IncomingMessage {
    pub message: MessageType,
    pub address: H160,
    pub room_id: String, // To identify which room the message came from
}

/// Reason for disconnection from the server
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisconnectReason {
    /// Another participant with the same identity has joined the room
    DuplicateIdentity,
    /// The room was closed
    RoomClosed,
    /// Participant was removed/kicked from the server
    Kicked,
    /// Other disconnection reasons (server shutdown, signal close, etc.)
    Other,
}

/// Types of messages that can be received from peers
#[derive(Debug, Clone)]
pub enum MessageType {
    Rfc4(Rfc4Message),
    InitVoice(VoiceInitData),
    VoiceFrame(VoiceFrameData),
    PeerJoined,                     // Peer joined a room
    PeerLeft,                       // Peer left a room
    Disconnected(DisconnectReason), // Disconnected from the server
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

/// Represents an outgoing message to be sent to communication rooms
#[derive(Debug, Clone)]
pub struct OutgoingMessage {
    pub packet: rfc4::Packet,
    pub unreliable: bool,
}

#[derive(Debug)]
struct Peer {
    alias: u32,
    profile: Option<UserProfile>,
    announced_version: Option<u32>,
    protocol_version: u32,
    last_activity: Instant,
    room_activity: HashMap<String, Instant>, // Track last activity per room
    profile_fetch_attempted: bool,           // Track if we already tried to fetch this profile
    profile_fetch_failures: u8,              // Count consecutive failures
    profile_fetch_banned_until: Option<Instant>, // Ban fetching until this time
}

struct ProfileUpdate {
    address: H160,
    peer_alias: u32,
    profile: UserProfile,
}

struct ProfileFetchFailure {
    address: H160,
    announced_version: u32,
}

/// Central message processor that handles all incoming and outgoing messages
/// from multiple communication rooms (WebSocket, LiveKit, etc.)
///
/// This processor:
/// - Manages peer lifecycle across multiple rooms
/// - Handles avatar creation/removal based on peer activity
/// - Processes RFC4 protocol messages (movement, chat, profiles)
/// - Manages voice chat data
/// - Prevents memory leaks with bounded queues
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
    #[allow(dead_code)]
    last_profile_response_sent: Instant,

    // Chat and scene messages (bounded to prevent memory exhaustion)
    chats: VecDeque<(H160, rfc4::Chat)>,
    incoming_scene_messages: HashMap<String, VecDeque<(H160, Vec<u8>)>>,

    // Track last chat timestamp per sender to filter duplicates
    last_chat_timestamps: HashMap<H160, f64>,

    // Profile updates from async tasks
    profile_update_receiver: mpsc::Receiver<ProfileUpdate>,
    profile_update_sender: mpsc::Sender<ProfileUpdate>,

    // Profile fetch failures from async tasks
    profile_failure_receiver: mpsc::Receiver<ProfileFetchFailure>,
    profile_failure_sender: mpsc::Sender<ProfileFetchFailure>,

    // Configurable realm bounds for movement compression
    realm_min: godot::prelude::Vector2i,
    realm_max: godot::prelude::Vector2i,

    // Social blacklist for blocked/muted filtering
    social_blacklist: Option<Gd<DclSocialBlacklist>>,

    // Cached blocked/muted sets for performance (updated when social_blacklist changes)
    cached_blocked: HashSet<H160>,
    cached_muted: HashSet<H160>,

    // Disconnect reason if disconnected from the server, along with the room_id
    disconnect_reason: Option<(DisconnectReason, String)>,
}

fn compare_f64(a: &f64, b: &f64) -> Ordering {
    match (a.is_nan(), b.is_nan()) {
        (true, true) => Ordering::Equal, // NaN == NaN for sorting purposes
        (true, false) => Ordering::Greater, // NaN sorts last
        (false, true) => Ordering::Less,
        (false, false) => {
            // Use total_cmp for consistent ordering (handles -0.0 vs 0.0)
            a.total_cmp(b)
        }
    }
}

impl MessageProcessor {
    /// Creates a new MessageProcessor instance
    ///
    /// # Arguments
    /// * `player_address` - The Ethereum address of the local player
    /// * `player_profile` - The player's profile (optional)
    /// * `avatars` - Reference to the avatar scene for managing avatar visuals
    pub fn new(
        player_address: H160,
        player_profile: Option<UserProfile>,
        avatars: Gd<AvatarScene>,
    ) -> Self {
        let (message_sender, message_receiver) = mpsc::channel(MESSAGE_CHANNEL_SIZE);
        let (outgoing_sender, outgoing_receiver) = mpsc::channel(OUTGOING_CHANNEL_SIZE);
        let (profile_update_sender, profile_update_receiver) =
            mpsc::channel(PROFILE_UPDATE_CHANNEL_SIZE);
        let (profile_failure_sender, profile_failure_receiver) =
            mpsc::channel(PROFILE_UPDATE_CHANNEL_SIZE);

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
            chats: VecDeque::new(),
            incoming_scene_messages: HashMap::new(),
            last_chat_timestamps: HashMap::new(),
            profile_update_receiver,
            profile_update_sender,
            profile_failure_receiver,
            profile_failure_sender,
            // Default realm bounds
            realm_min: godot::prelude::Vector2i::new(-150, -150),
            realm_max: godot::prelude::Vector2i::new(163, 158),
            social_blacklist: None,
            cached_blocked: HashSet::new(),
            cached_muted: HashSet::new(),
            disconnect_reason: None,
        }
    }

    /// Sets the social blacklist reference for filtering blocked/muted users
    pub fn set_social_blacklist(&mut self, blacklist: Gd<DclSocialBlacklist>) {
        // Update cached sets when blacklist changes
        let blacklist_bind = blacklist.bind();
        self.cached_blocked
            .clone_from(blacklist_bind.get_blocked_set());

        // Merge blocked users into muted cache (blocked users are also muted)
        self.cached_muted.clone_from(blacklist_bind.get_muted_set());
        self.cached_muted.extend(blacklist_bind.get_blocked_set());
        drop(blacklist_bind);

        self.social_blacklist = Some(blacklist);
    }

    /// Updates the cached blocked/muted sets from the social blacklist
    pub fn refresh_blacklist_cache(&mut self) {
        if let Some(blacklist) = &self.social_blacklist {
            let blacklist_bind = blacklist.bind();
            let new_blocked = blacklist_bind.get_blocked_set().clone();
            let new_muted = blacklist_bind.get_muted_set().clone();

            // Find newly blocked addresses (in new set but not in old set)
            let newly_blocked: Vec<H160> = new_blocked
                .difference(&self.cached_blocked)
                .cloned()
                .collect();

            // Find newly unblocked addresses (in old set but not in new set)
            let newly_unblocked: Vec<H160> = self
                .cached_blocked
                .difference(&new_blocked)
                .cloned()
                .collect();

            // Update the cached sets
            self.cached_blocked.clone_from(&new_blocked);
            // Merge blocked users into muted cache (blocked users are also muted)
            self.cached_muted = new_muted;
            self.cached_muted.extend(&new_blocked);

            // Hide avatars for newly blocked users
            if !newly_blocked.is_empty() {
                let mut avatar_scene_ref = self.avatars.clone();
                let mut avatar_scene = avatar_scene_ref.bind_mut();

                for blocked_address in newly_blocked {
                    if let Some(peer) = self.peer_identities.get(&blocked_address) {
                        tracing::info!(
                            "🚫 Hiding avatar for blocked user {:#x} (alias: {})",
                            blocked_address,
                            peer.alias
                        );
                        avatar_scene.set_avatar_blocked(peer.alias, true);
                    }
                }
            }

            // Show avatars for newly unblocked users
            if !newly_unblocked.is_empty() {
                let mut avatar_scene_ref = self.avatars.clone();
                let mut avatar_scene = avatar_scene_ref.bind_mut();

                for unblocked_address in newly_unblocked {
                    if let Some(peer) = self.peer_identities.get(&unblocked_address) {
                        tracing::info!(
                            "✅ Showing avatar for unblocked user {:#x} (alias: {})",
                            unblocked_address,
                            peer.alias
                        );
                        avatar_scene.set_avatar_blocked(peer.alias, false);
                    }
                }
            }
        }
    }

    /// Returns a sender channel that rooms can use to send messages to this processor
    ///
    /// Rooms should use this sender to forward all incoming messages for centralized processing
    pub fn get_message_sender(&self) -> mpsc::Sender<IncomingMessage> {
        self.message_sender.clone()
    }

    /// Sets the realm bounds for movement compression
    ///
    /// These bounds define the coordinate space for quantizing movement data.
    /// The default values are (-150, -150) to (163, 158).
    ///
    /// # Arguments
    /// * `min` - The minimum x,y coordinates of the realm
    /// * `max` - The maximum x,y coordinates of the realm
    pub fn set_realm_bounds(
        &mut self,
        min: godot::prelude::Vector2i,
        max: godot::prelude::Vector2i,
    ) {
        self.realm_min = min;
        self.realm_max = max;
        tracing::info!("Updated realm bounds: min={:?}, max={:?}", min, max);
    }

    /// Consumes and returns all pending outgoing messages
    ///
    /// CommunicationManager should call this regularly to retrieve messages
    /// that need to be sent through the appropriate rooms
    pub fn consume_outgoing_messages(&mut self) -> Vec<OutgoingMessage> {
        let mut messages = Vec::new();
        while let Ok(message) = self.outgoing_receiver.try_recv() {
            messages.push(message);
        }
        messages
    }

    /// Checks if there was a disconnection and returns the reason along with the room_id
    /// Clears the reason after returning it
    ///
    /// CommunicationManager should call this regularly to check for disconnection
    pub fn consume_disconnect_reason(&mut self) -> Option<(DisconnectReason, String)> {
        self.disconnect_reason.take()
    }

    /// Processes all pending messages and performs periodic maintenance
    ///
    /// This should be called regularly (e.g., every frame) to:
    /// - Process incoming messages from all rooms
    /// - Handle profile updates from async tasks
    /// - Clean up inactive peers
    ///
    /// Returns true if processing should continue, false if fatal error
    pub fn poll(&mut self) -> bool {
        // Handle profile updates from async tasks
        while let Ok(update) = self.profile_update_receiver.try_recv() {
            tracing::debug!(
                "Received profile update for {:#x}: version {}",
                update.address,
                update.profile.version
            );

            // Brief borrow scope for avatar update
            {
                let mut avatar_scene_ref = self.avatars.clone();
                let mut avatar_scene = avatar_scene_ref.bind_mut();
                avatar_scene.update_avatar_by_alias(update.peer_alias, &update.profile);
            }

            if let Some(peer) = self.peer_identities.get_mut(&update.address) {
                peer.profile = Some(update.profile);
                peer.profile_fetch_attempted = false; // Reset so we can fetch again if needed
                peer.profile_fetch_failures = 0; // Reset failure count on success
                peer.profile_fetch_banned_until = None; // Clear any ban
            }
        }

        // Handle profile fetch failures
        while let Ok(failure) = self.profile_failure_receiver.try_recv() {
            if let Some(peer) = self.peer_identities.get_mut(&failure.address) {
                peer.profile_fetch_failures += 1;
                peer.profile_fetch_attempted = false; // Allow retry

                if peer.profile_fetch_failures >= 2 {
                    // Ban profile fetching for 30 seconds after 2 failures
                    peer.profile_fetch_banned_until =
                        Some(Instant::now() + Duration::from_secs(30));
                    tracing::warn!(
                        "Banning profile fetch for {:#x} for 30 seconds after {} failures (announced version {} not available)",
                        failure.address,
                        peer.profile_fetch_failures,
                        failure.announced_version
                    );
                }
            }
        }

        // Process incoming messages
        while let Ok(message) = self.message_receiver.try_recv() {
            self.process_message(message);
        }

        // Remove inactive avatars (only if inactive in ALL rooms)
        // With proper lifecycle events, we can use a longer timeout as a safety net
        let inactive_threshold = std::time::Duration::from_secs(INACTIVE_PEER_THRESHOLD_SECS);
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
                    tracing::debug!(
                        "⏰ Peer {:#x} (alias: {}) timed out in room '{}' (safety cleanup)",
                        address,
                        peer.alias,
                        room
                    );
                }

                // If peer has no active rooms left AND has been inactive, remove it
                if peer.room_activity.is_empty()
                    && peer.last_activity.elapsed() > inactive_threshold
                {
                    tracing::info!(
                        "⏰ Peer {:#x} (alias: {}) has no active rooms and timed out - removing",
                        address,
                        peer.alias
                    );
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
                    tracing::info!(
                        "🗑️ Removed inactive peer {:#x} (alias: {})",
                        address,
                        peer.alias
                    );
                    avatar_scene.remove_avatar(peer.alias);

                    // Clean up chat timestamp tracking for removed peer
                    self.last_chat_timestamps.remove(&address);
                }
            }
        }

        // Periodic profile requests
        if self.last_profile_request_sent.elapsed().as_secs_f32() > PROFILE_REQUEST_INTERVAL_SECS {
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
                tracing::debug!(
                    "📨 Existing peer {:#x} (alias: {}) now also seen in room '{}'",
                    message.address,
                    peer.alias,
                    message.room_id
                );
            } else {
                tracing::debug!(
                    "📨 Message from {:#x} via room '{}' (existing peer, alias: {})",
                    message.address,
                    message.room_id,
                    peer.alias
                );
            }

            // Update activity for this specific room
            peer.room_activity
                .insert(message.room_id.clone(), Instant::now());
            peer.last_activity = Instant::now();

            if let MessageType::Rfc4(rfc4_msg) = &message.message {
                peer.protocol_version = rfc4_msg.protocol_version;
            }
            peer.alias
        } else {
            // Create new peer only if it doesn't exist
            self.peer_alias_counter += 1;
            let new_alias = self.peer_alias_counter;

            tracing::info!(
                "🆕 Creating new peer {:#x} from room '{}' with alias: {}",
                message.address,
                message.room_id,
                new_alias
            );

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
                        DEFAULT_PROTOCOL_VERSION
                    },
                    last_activity: Instant::now(),
                    room_activity,
                    profile_fetch_attempted: false,
                    profile_fetch_failures: 0,
                    profile_fetch_banned_until: None,
                },
            );

            // Brief borrow to add new avatar
            {
                let mut avatar_scene_ref = self.avatars.clone();
                let mut avatar_scene = avatar_scene_ref.bind_mut();
                avatar_scene
                    .add_avatar(new_alias, GString::from(format!("{:#x}", message.address)));

                // If the user is blocked, hide the avatar immediately
                if self.cached_blocked.contains(&message.address) {
                    tracing::info!(
                        "🚫 New peer {:#x} (alias: {}) is blocked, hiding avatar",
                        message.address,
                        new_alias
                    );
                    avatar_scene.set_avatar_blocked(new_alias, true);
                }
            }

            // Send initial profile request to the room where this message came from
            let request_packet = rfc4::Packet {
                message: Some(rfc4::packet::Message::ProfileRequest(
                    rfc4::ProfileRequest {
                        address: format!("{:#x}", message.address),
                        profile_version: 0, // Request any version
                    },
                )),
                protocol_version: DEFAULT_PROTOCOL_VERSION,
            };

            let outgoing = OutgoingMessage {
                packet: request_packet,
                unreliable: false,
            };

            if let Err(e) = self.outgoing_sender.try_send(outgoing) {
                tracing::warn!("Failed to queue initial ProfileRequest for new peer: {}", e);
            } else {
                tracing::debug!(
                    "📤 Sending initial ProfileRequest for new peer {:#x}",
                    message.address
                );
            }

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
                // Check if user is muted for voice (using cached set for O(1) lookup)
                // Note: cached_muted includes both muted AND blocked users
                if self.cached_muted.contains(&message.address) {
                    return; // muted/blocked - ignore voice frames
                }

                // If all the frame.data is less than 10, we skip the frame
                if voice_frame.data.iter().all(|&c| c.abs() < 10) {
                    return;
                }

                let frame = godot::prelude::PackedVector2Array::from_iter(
                    voice_frame.data.iter().map(|c| {
                        let val = (*c as f32) / (i16::MAX as f32);
                        godot::prelude::Vector2 { x: val, y: val }
                    }),
                );

                let mut avatar_scene_ref = self.avatars.clone();
                let mut avatar_scene = avatar_scene_ref.bind_mut();
                avatar_scene.push_voice_frame(peer_alias, frame);
            }
            MessageType::Rfc4(rfc4_msg) => {
                // Handle RFC4 messages
                self.handle_rfc4_message(rfc4_msg.message.clone(), peer_alias, message.address);
            }
            MessageType::PeerJoined => {
                // Peer joined event - ensure peer exists and update room activity
                tracing::debug!(
                    "👋 Peer {:#x} joined room '{}' (alias: {})",
                    message.address,
                    room_id,
                    peer_alias
                );
            }
            MessageType::PeerLeft => {
                // Handle peer leaving a room
                self.handle_peer_left(message.address, room_id);
            }
            MessageType::Disconnected(reason) => {
                // Set disconnect_reason if not already set (first disconnect wins)
                // Any room disconnect (scene or archipelago) should be reported
                if self.disconnect_reason.is_none() {
                    self.disconnect_reason = Some((*reason, room_id));
                }
            }
        }
    }

    fn handle_peer_left(&mut self, address: H160, room_id: String) {
        if let Some(peer) = self.peer_identities.get_mut(&address) {
            peer.room_activity.remove(&room_id);
            tracing::debug!(
                "👋 Peer {:#x} (alias: {}) left room '{}'",
                address,
                peer.alias,
                room_id
            );

            // If peer has no more active rooms, remove it
            if peer.room_activity.is_empty() {
                let alias = peer.alias;
                self.peer_identities.remove(&address);
                tracing::info!(
                    "🗑️  Removing peer {:#x} (alias: {}) - no longer in any rooms",
                    address,
                    alias
                );

                // Remove avatar
                let mut avatar_scene_ref = self.avatars.clone();
                let mut avatar_scene = avatar_scene_ref.bind_mut();
                avatar_scene.remove_avatar(alias);

                // Clean up chat timestamp tracking for removed peer
                self.last_chat_timestamps.remove(&address);
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
                tracing::debug!(
                    "Received Position from {:#x}: pos({}, {}, {}), rot({}, {}, {}, {})",
                    address,
                    position.position_x,
                    position.position_y,
                    position.position_z,
                    position.rotation_x,
                    position.rotation_y,
                    position.rotation_z,
                    position.rotation_w
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

                // Get position from compressed movement with configured realm bounds
                let pos = movement.position(self.realm_min, self.realm_max);
                let velocity = movement.velocity();
                let rotation_rad = -movement.temporal.rotation_f32();
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

                // Check if user is muted for chat (using cached set for O(1) lookup)
                // Note: cached_muted includes both muted AND blocked users
                if self.cached_muted.contains(&address) {
                    tracing::info!("Ignoring muted {:#x}", address);
                    return; // muted/blocked - ignore chat messages
                }

                // Check for duplicate messages based on timestamp
                // Check if we've seen a recent message from this sender
                if let Some(&last_timestamp) = self.last_chat_timestamps.get(&address) {
                    // If the new timestamp is older or within tolerance of the last one, it's a duplicate
                    if compare_f64(&chat.timestamp, &last_timestamp) != Ordering::Greater {
                        tracing::info!(
                            "Discarding duplicate chat from {:#x}: timestamp {} <= {} (last + tolerance)",
                            address,
                            chat.timestamp,
                            last_timestamp
                        );
                        return;
                    }
                }

                // Update the last timestamp for this sender
                self.last_chat_timestamps.insert(address, chat.timestamp);

                // Enforce bounded queue for chat messages
                if self.chats.len() >= MAX_CHAT_MESSAGES {
                    let dropped = self.chats.pop_front();
                    if let Some((addr, _)) = dropped {
                        tracing::warn!("Chat queue full, dropping oldest message from {:#x}", addr);
                    }
                }
                let chat = if chat.message.len() > MAX_CHAT_MESSAGE_SIZE {
                    rfc4::Chat {
                        message: format!("{}...", &chat.message[..MAX_CHAT_MESSAGE_SIZE]),
                        timestamp: chat.timestamp,
                    }
                } else {
                    chat
                };
                self.chats.push_back((address, chat));
            }
            rfc4::packet::Message::ProfileVersion(announce_profile_version) => {
                tracing::debug!(
                    "Received ProfileVersion from {:#x}: version {}",
                    address,
                    announce_profile_version.profile_version
                );

                let announced_version = announce_profile_version.profile_version;

                // Get current version and update peer
                let (current_version, peer_alias_for_async) = if let Some(peer) =
                    self.peer_identities.get_mut(&address)
                {
                    let current_version = peer.profile.as_ref().map(|p| p.version).unwrap_or(0);

                    // If announcing a different version than before, reset failure tracking
                    if peer.announced_version != Some(announced_version) {
                        peer.profile_fetch_failures = 0;
                        peer.profile_fetch_banned_until = None;
                        peer.profile_fetch_attempted = false;
                        tracing::debug!(
                                "New profile version announced for {:#x}: {} (was {:?}), resetting failure tracking",
                                address,
                                announced_version,
                                peer.announced_version
                            );
                    }

                    peer.announced_version = Some(announced_version);
                    (current_version, peer_alias)
                } else {
                    (0, peer_alias)
                };

                // Check if profile fetch is banned
                let is_banned = if let Some(peer) = self.peer_identities.get(&address) {
                    if let Some(banned_until) = peer.profile_fetch_banned_until {
                        if Instant::now() < banned_until {
                            tracing::debug!(
                                "Profile fetch for {:#x} is banned for {} more seconds",
                                address,
                                (banned_until - Instant::now()).as_secs()
                            );
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                };

                // If the announced version is newer than what we have AND we haven't tried to fetch it yet AND not banned
                if announced_version > current_version
                    && !self
                        .peer_identities
                        .get(&address)
                        .is_some_and(|p| p.profile_fetch_attempted)
                    && !is_banned
                {
                    tracing::info!(
                        "Requesting newer profile from {:#x}: announced={}, current={}",
                        address,
                        announced_version,
                        current_version
                    );

                    // Mark that we're attempting to fetch this profile
                    if let Some(peer) = self.peer_identities.get_mut(&address) {
                        peer.profile_fetch_attempted = true;
                    }

                    // First, try to fetch from lambda server
                    tracing::debug!("comms > requesting profile from lambda for {:#x}", address);

                    let profile_sender = self.profile_update_sender.clone();
                    let profile_failure_sender = self.profile_failure_sender.clone();
                    let outgoing_sender = self.outgoing_sender.clone();
                    let announced_version_for_retry = announced_version;
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
                            tracing::debug!(
                                "Fetched profile from lambda for {:#x}: version {}",
                                address,
                                profile.version
                            );
                            // Check if the fetched version matches what was announced
                            let version_mismatch = profile.version < announced_version_for_retry;
                            if version_mismatch {
                                tracing::warn!(
                                    "Profile version mismatch for {:#x}: announced={}, fetched={}",
                                    address,
                                    announced_version_for_retry,
                                    profile.version
                                );
                                // Send failure notification
                                let _ = profile_failure_sender
                                    .send(ProfileFetchFailure {
                                        address,
                                        announced_version: announced_version_for_retry,
                                    })
                                    .await;
                            }

                            if let Err(e) = profile_sender
                                .send(ProfileUpdate {
                                    address,
                                    peer_alias: peer_alias_for_async,
                                    profile,
                                })
                                .await
                            {
                                tracing::error!("Failed to send profile update: {}", e);
                            }
                        } else {
                            tracing::error!(
                                "fetch profile lambda > failed to fetch profile from lambda for {:#x}: {:?}",
                                address,
                                result
                            );

                            // Lambda fetch failed, likely a guest user - send ProfileRequest to peer
                            tracing::info!(
                                "Profile not found on lambda for {:#x}, sending ProfileRequest to peer (likely guest user)",
                                address
                            );

                            let request_packet = rfc4::Packet {
                                message: Some(rfc4::packet::Message::ProfileRequest(
                                    rfc4::ProfileRequest {
                                        address: format!("{:#x}", address),
                                        profile_version: announced_version_for_retry,
                                    },
                                )),
                                protocol_version: DEFAULT_PROTOCOL_VERSION,
                            };

                            let outgoing = OutgoingMessage {
                                packet: request_packet,
                                unreliable: false,
                            };

                            if let Err(e) = outgoing_sender.try_send(outgoing) {
                                tracing::warn!(
                                    "Failed to queue ProfileRequest after lambda failure: {}",
                                    e
                                );
                            } else {
                                tracing::debug!(
                                    "📤 Sending ProfileRequest for {:#x} (version {}) after lambda failure",
                                    address,
                                    announced_version_for_retry
                                );
                            }

                            // Send failure notification
                            let _ = profile_failure_sender
                                .send(ProfileFetchFailure {
                                    address,
                                    announced_version: announced_version_for_retry,
                                })
                                .await;
                        }
                    });
                }
            }
            rfc4::packet::Message::ProfileRequest(profile_request) => {
                tracing::debug!(
                    "Received ProfileRequest from {:#x} for address {}",
                    address,
                    profile_request.address
                );

                // Parse the requested address
                if let Ok(requested_address) = profile_request.address.parse::<H160>() {
                    // First check if they're requesting our player's profile
                    if requested_address == self.player_address {
                        if let Some(player_profile) = &self.player_profile {
                            let serialized_profile = serde_json::to_string(&player_profile.content)
                                .unwrap_or_else(|_| "{}".to_string());

                            let response_packet = rfc4::Packet {
                                message: Some(rfc4::packet::Message::ProfileResponse(
                                    rfc4::ProfileResponse {
                                        serialized_profile,
                                        base_url: player_profile.base_url.clone(),
                                    },
                                )),
                                protocol_version: DEFAULT_PROTOCOL_VERSION,
                            };

                            // Send response back to the requesting room
                            let outgoing = OutgoingMessage {
                                packet: response_packet,
                                unreliable: false,
                            };

                            if let Err(e) = self.outgoing_sender.try_send(outgoing) {
                                tracing::warn!("Failed to queue ProfileResponse: {}", e);
                            } else {
                                tracing::debug!("📤 Sending ProfileResponse to {:#x}", address);
                            }
                        } else {
                            tracing::debug!(
                                "ProfileRequest for our address but no profile available"
                            );
                        }
                    }
                } else {
                    tracing::warn!(
                        "Invalid address in ProfileRequest: {}",
                        profile_request.address
                    );
                }
            }
            rfc4::packet::Message::ProfileResponse(profile_response) => {
                tracing::debug!("Received ProfileResponse from {:#x}", address);

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

                // Parse the eth_address from the profile to determine who this profile belongs to
                let profile_address = match serialized_profile.eth_address.parse::<H160>() {
                    Ok(addr) => addr,
                    Err(e) => {
                        tracing::error!(
                            "Invalid eth_address in ProfileResponse: {} - error: {}",
                            serialized_profile.eth_address,
                            e
                        );
                        return;
                    }
                };

                tracing::info!(
                    "ProfileResponse from {:#x} contains profile for {:#x} (version {})",
                    address,
                    profile_address,
                    incoming_version
                );

                // Update the profile for the address specified IN the profile, not the sender
                if let Some(peer) = self.peer_identities.get_mut(&profile_address) {
                    let current_version = peer.profile.as_ref().map(|p| p.version).unwrap_or(0);

                    if incoming_version <= current_version {
                        tracing::debug!(
                            "Ignoring ProfileResponse for {:#x}: version {} <= current {}",
                            profile_address,
                            incoming_version,
                            current_version
                        );
                        return;
                    }

                    let profile = UserProfile {
                        version: incoming_version,
                        content: serialized_profile.clone(),
                        base_url: profile_response.base_url.clone(),
                    };

                    let mut avatar_scene_ref = self.avatars.clone();
                    let mut avatar_scene = avatar_scene_ref.bind_mut();
                    // Use the peer's alias for the address in the profile
                    avatar_scene.update_avatar_by_alias(peer.alias, &profile);
                    peer.profile = Some(profile);
                    peer.profile_fetch_attempted = false; // Reset so we can fetch again if needed

                    tracing::info!(
                        "Updated profile for {:#x} (alias {}) to version {}",
                        profile_address,
                        peer.alias,
                        incoming_version
                    );
                } else {
                    tracing::debug!(
                        "Received ProfileResponse for unknown peer {:#x}",
                        profile_address
                    );
                }
            }
            rfc4::packet::Message::Scene(scene) => {
                // Limit the number of scene IDs we track
                if !self.incoming_scene_messages.contains_key(&scene.scene_id)
                    && self.incoming_scene_messages.len() >= MAX_SCENE_IDS
                {
                    // Remove the oldest scene ID (arbitrary choice - could use LRU)
                    if let Some(oldest_key) = self.incoming_scene_messages.keys().next().cloned() {
                        self.incoming_scene_messages.remove(&oldest_key);
                        tracing::warn!(
                            "Scene message map full, dropped messages for scene: {}",
                            oldest_key
                        );
                    }
                }

                let entry = self
                    .incoming_scene_messages
                    .entry(scene.scene_id.clone())
                    .or_default();

                // Enforce bounded queue per scene
                if entry.len() >= MAX_SCENE_MESSAGES_PER_SCENE {
                    let dropped = entry.pop_front();
                    if let Some((addr, _)) = dropped {
                        tracing::warn!(
                            "Scene {} message queue full, dropping oldest message from {:#x}",
                            scene.scene_id,
                            addr
                        );
                    }
                }
                entry.push_back((address, scene.data));
            }
            rfc4::packet::Message::Voice(_voice) => {}
            rfc4::packet::Message::PlayerEmote(player_emote) => {
                tracing::debug!(
                    "Received PlayerEmote from {:#x}: {:?}",
                    address,
                    player_emote
                );

                // Let avatar_scene handle emotes
                let mut avatar_scene_ref = self.avatars.clone();
                let mut avatar_scene = avatar_scene_ref.bind_mut();
                avatar_scene.play_emote(peer_alias, player_emote.incremental_id, &player_emote.urn);
            }
            rfc4::packet::Message::SceneEmote(_) => {
                tracing::warn!("Not implemented: SceneEmote handling in message_processor");
            }
        }
    }

    pub fn consume_chats(&mut self) -> Vec<(H160, rfc4::Chat)> {
        self.chats.drain(..).collect()
    }

    pub fn consume_scene_messages(&mut self, scene_id: &str) -> Vec<(H160, Vec<u8>)> {
        if let Some(messages) = self.incoming_scene_messages.get_mut(scene_id) {
            messages.drain(..).collect()
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
        self.last_chat_timestamps.clear();
        // Clean up all avatars when disconnected
        let mut avatar_scene_ref = self.avatars.clone();
        avatar_scene_ref.bind_mut().clean();
    }
}
