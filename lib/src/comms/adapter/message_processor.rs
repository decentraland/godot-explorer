use std::{
    collections::{HashMap, HashSet, VecDeque},
    time::{Duration, Instant},
};

use ethers_core::types::H160;
use godot::prelude::{GString, Gd, ToGodot};
use std::cmp::Ordering;
use tokio::sync::mpsc;

use crate::{
    auth::wallet::AsH160,
    avatars::avatar_scene::AvatarScene,
    comms::{
        consts::{
            truncate_utf8_safe, DEFAULT_PROTOCOL_VERSION, INACTIVE_PEER_THRESHOLD_SECS,
            MAX_CHAT_MESSAGES, MAX_CHAT_MESSAGE_SIZE, MAX_SCENE_IDS, MAX_SCENE_MESSAGES_PER_SCENE,
            MESSAGE_CHANNEL_SIZE, OUTGOING_CHANNEL_SIZE, PROFILE_REQUEST_INTERVAL_SECS,
            PROFILE_UPDATE_CHANNEL_SIZE, PULSE_ROOM_ID,
        },
        profile::{SerializedProfile, UserProfile},
    },
    content::profile::{
        prepare_request_requirements, request_lambda_profile, request_registry_profile,
    },
    dcl::components::proto_components::kernel::comms::rfc4,
    godot_classes::{dcl_global::DclGlobal, dcl_social_blacklist::DclSocialBlacklist},
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
    InitVideo(VideoInitData),
    VideoFrame(VideoFrameData),
    InitStreamerAudio(StreamerAudioInitData),
    StreamerAudioFrame(StreamerAudioFrameData),
    VideoTrackEnded(String), // Video track ended/unsubscribed (sid)
    VideoTrackMuted { sid: String, muted: bool },
    ActiveSpeakersChanged(Vec<String>), // Ordered speaker identities, loudest first
    PeerJoined,                         // Peer joined a room
    PeerLeft,                           // Peer left a room
    Disconnected(DisconnectReason),     // Disconnected from the server
    PeerMetadata(String),               // Peer metadata (e.g., version info for staging/dev builds)
    RoomMetadataChanged(String),        // Room metadata changed (e.g., ban list update)
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

/// LiveKit video track source, mapped from `livekit::track::TrackSource`.
/// Screenshare ranks above cameras in stream selection (see `best_video_track`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoTrackSourceKind {
    Screenshare,
    Camera,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct VideoInitData {
    pub width: u32,
    pub height: u32,
    /// LiveKit track sid — the registry key for this video stream.
    pub sid: String,
    /// LiveKit participant identity that published the track.
    pub identity: String,
    pub source: VideoTrackSourceKind,
    pub muted: bool,
}

#[derive(Debug, Clone)]
pub struct VideoFrameData {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub sid: String,
}

// Streamer audio data (separate from voice chat - for video player audio)
#[derive(Debug, Clone)]
pub struct StreamerAudioInitData {
    pub sample_rate: u32,
    pub num_channels: u32,
    pub samples_per_channel: u32,
}

#[derive(Debug, Clone)]
pub struct StreamerAudioFrameData {
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
    peer_version: Option<String>,            // Client version for staging/dev builds
    lambdas_endpoint: Option<String>, // Peer's lambda URL from LiveKit metadata (lambdasEndpoint)
    last_movement_timestamp: f32,     // Dedup: last movement timestamp received
    last_emote_incremental_id: u32,   // Dedup: last emote incremental ID received
    /// Transport-preference gate: true while this peer is a live member of the "pulse" room
    /// (set on any pulse-bridged message, cleared by a pulse PeerLeft). While set, this peer's
    /// movement/emotes from LiveKit rooms are DISCARDED — never merged: LiveKit timestamps are
    /// the sender's clock and Pulse timestamps are the server tick, so comparing them starves
    /// one source permanently. On either flip both dedup layers are reset for the same reason.
    pulse_live: bool,
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

struct VideoTrackInfo {
    /// LiveKit participant identity that published this track.
    identity: String,
    /// Wallet address when the identity is one (used for the block-list check).
    /// Streamer identities (`stream:…`, `presentation-bot:…`, `…-streamer`) have none.
    identity_h160: Option<H160>,
    source: VideoTrackSourceKind,
    muted: bool,
    #[allow(dead_code)]
    width: u32,
    #[allow(dead_code)]
    height: u32,
    #[allow(dead_code)]
    last_frame_time: Instant,
    /// Monotonic arrival index — "first available" tie-breaker in selection.
    order: u64,
}

/// Participants whose identity starts with this prefix are authoritative video
/// sources (slides / playback bots) and always hold the stream.
const PRESENTATION_BOT_IDENTITY_PREFIX: &str = "presentation-bot:";

/// Minimum hold before the stream may follow a new active speaker, preventing
/// flicker during rapid speaker changes (mirrors unity-explorer).
const MIN_SPEAKER_HOLD: Duration = Duration::from_millis(1500);

/// Pick the video track to route to `livekit-video://` players. Mirror of
/// unity-explorer `LivekitPlayer.BestFollowCandidate`/`BestInitialVideoKey`:
/// presentation bot → unmuted screen share → dominant active speaker with a
/// video track (only once the hold elapsed; the current speaker keeps it) →
/// keep the current track → first available.
fn best_video_track(
    tracks: &HashMap<String, VideoTrackInfo>,
    current_sid: Option<&str>,
    active_speakers: &[String],
    hold_elapsed: bool,
) -> Option<String> {
    let first_by_order = |mut candidates: Vec<(&String, &VideoTrackInfo)>| -> Option<String> {
        candidates.sort_by_key(|(_, info)| info.order);
        candidates.first().map(|(sid, _)| (*sid).clone())
    };

    let bots: Vec<_> = tracks
        .iter()
        .filter(|(_, info)| info.identity.starts_with(PRESENTATION_BOT_IDENTITY_PREFIX))
        .collect();
    if let Some(sid) = first_by_order(bots) {
        return Some(sid);
    }

    // A muted (paused) share falls through so video follows speakers until it resumes.
    let shares: Vec<_> = tracks
        .iter()
        .filter(|(_, info)| info.source == VideoTrackSourceKind::Screenshare && !info.muted)
        .collect();
    if let Some(sid) = first_by_order(shares) {
        return Some(sid);
    }

    let current_identity = current_sid
        .and_then(|sid| tracks.get(sid))
        .map(|info| info.identity.as_str());
    if hold_elapsed {
        for speaker in active_speakers {
            if Some(speaker.as_str()) == current_identity {
                break; // the current source is the dominant speaker — keep it
            }
            let speaker_tracks: Vec<_> = tracks
                .iter()
                .filter(|(_, info)| info.identity == *speaker)
                .collect();
            if let Some(sid) = first_by_order(speaker_tracks) {
                return Some(sid);
            }
        }
    }

    if let Some(sid) = current_sid {
        if tracks.contains_key(sid) {
            return Some(sid.to_string());
        }
    }

    first_by_order(tracks.iter().collect())
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

    // Video track management — keyed by LiveKit track sid. Only the selected
    // track's frames are forwarded to the scenes' livekit video players.
    active_video_tracks: HashMap<String, VideoTrackInfo>,
    video_track_order_counter: u64,
    selected_video_sid: Option<String>,
    // Ordered speaker identities (loudest first) from the latest
    // ActiveSpeakersChanged event of any connected room.
    active_speakers: Vec<String>,
    video_switched_at: Instant,

    // Disconnect reason if disconnected from the server, along with the room_id
    disconnect_reason: Option<(DisconnectReason, String)>,

    // Set to true when room metadata indicates the local player is banned
    room_metadata_banned: bool,
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
            active_video_tracks: HashMap::new(),
            video_track_order_counter: 0,
            selected_video_sid: None,
            active_speakers: Vec::new(),
            video_switched_at: Instant::now(),
            disconnect_reason: None,
            room_metadata_banned: false,
        }
    }

    /// Which rfc4 messages the transport-preference gate applies to: exactly the avatar-sync
    /// slice that rides Pulse (movement in its three encodings, and emotes). Everything else
    /// either never rides Pulse (chat, scene, profile request/response) or is idempotent
    /// (profile-version announcements) and flows from both transports untouched.
    fn is_gated_by_pulse_preference(message: &rfc4::packet::Message) -> bool {
        matches!(
            message,
            rfc4::packet::Message::Position(_)
                | rfc4::packet::Message::Movement(_)
                | rfc4::packet::Message::MovementCompressed(_)
                | rfc4::packet::Message::PlayerEmote(_)
        )
    }

    /// Compares two lambdas endpoints ignoring trailing-slash style — Godot
    /// publishes `…/lambdas/` in its LiveKit metadata while Unity publishes
    /// `…/lambdas`, and the slash-only mismatch must not make the realm's own
    /// endpoint look like a different catalyst.
    fn is_same_lambda_endpoint(a: &str, b: &str) -> bool {
        a.trim_end_matches('/') == b.trim_end_matches('/')
    }

    /// Validates a peer-advertised `lambdasEndpoint` (untrusted LiveKit
    /// metadata). Anything that isn't a plausible http(s) URL is discarded so
    /// the profile fetch keeps using the realm's own lambda endpoint instead.
    fn sanitize_lambdas_endpoint(endpoint: &str) -> Option<&str> {
        let trimmed = endpoint.trim();
        let host_and_path = trimmed
            .strip_prefix("https://")
            .or_else(|| trimmed.strip_prefix("http://"))?;
        if host_and_path.is_empty()
            || host_and_path.starts_with('/')
            || trimmed.chars().any(char::is_whitespace)
        {
            return None;
        }
        Some(trimmed)
    }

    /// Returns true if the address looks like a real player (non-synthetic Ethereum address).
    /// Synthetic addresses (like H160::from_low_u64_be(1) for the auth server) are non-player.
    fn is_player_address(address: H160) -> bool {
        // Addresses in the first 0xff range are Ethereum precompiles / synthetic,
        // not real players. Real Ethereum addresses are derived from public keys
        // and are effectively random 160-bit values.
        address > H160::from_low_u64_be(0xff)
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
                        tracing::debug!(
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
                        tracing::debug!(
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
        tracing::debug!("Updated realm bounds: min={:?}, max={:?}", min, max);
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

    /// Returns true (and resets) if room metadata indicated the local player was banned.
    pub fn consume_room_metadata_banned(&mut self) -> bool {
        let banned = self.room_metadata_banned;
        self.room_metadata_banned = false;
        banned
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
            let mut fetch_state: Option<(u32, i32, bool)> = None;
            if let Some(peer) = self.peer_identities.get_mut(&failure.address) {
                peer.profile_fetch_failures += 1;
                peer.profile_fetch_attempted = false; // Allow retry

                if peer.profile_fetch_failures >= 2 {
                    // Ban profile fetching for 30 seconds after 2 failures
                    peer.profile_fetch_banned_until =
                        Some(Instant::now() + Duration::from_secs(30));
                    tracing::debug!(
                        "Banning profile fetch for {:#x} for 30 seconds after {} failures (announced version {} not available)",
                        failure.address,
                        peer.profile_fetch_failures,
                        failure.announced_version
                    );
                }
                fetch_state = Some((
                    peer.alias,
                    peer.profile_fetch_failures as i32,
                    peer.profile_fetch_banned_until.is_some(),
                ));
            }
            // Surface the state to the avatar's nameplate (dev shows "Loading"/"Failed").
            if let Some((alias, failures, banned)) = fetch_state {
                let mut avatar_scene_ref = self.avatars.clone();
                let mut avatar_scene = avatar_scene_ref.bind_mut();
                avatar_scene.set_avatar_profile_fetch_state(alias, failures, banned);
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

            // Check each room the peer has been seen in.
            // "pulse" is exempt: its server stops sending deltas for static distant peers by
            // design (interest-management tiers), so activity is not a liveness signal there.
            // Membership is authoritative instead — reliable PlayerJoined/PlayerLeft, plus the
            // synthetic PeerLeft flood PulseRoom emits on any teardown.
            let rooms_to_check: Vec<String> = peer.room_activity.keys().cloned().collect();
            for room_id in rooms_to_check {
                if room_id == PULSE_ROOM_ID {
                    continue;
                }
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
                    tracing::debug!(
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
                    tracing::debug!(
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

    /// Handle media messages (video/audio from streamers) that don't need peer lifecycle.
    /// These use synthetic addresses (H160::zero()) and must bypass the player address check.
    fn process_media_message(&mut self, message: IncomingMessage) {
        match message.message {
            MessageType::InitVideo(video_init) => {
                tracing::debug!(
                    "InitVideo track {} from '{}' ({:?}): {}x{}",
                    video_init.sid,
                    video_init.identity,
                    video_init.source,
                    video_init.width,
                    video_init.height
                );

                let order = self.video_track_order_counter;
                self.video_track_order_counter += 1;
                self.active_video_tracks.insert(
                    video_init.sid,
                    VideoTrackInfo {
                        identity_h160: video_init.identity.as_str().as_h160(),
                        identity: video_init.identity,
                        source: video_init.source,
                        muted: video_init.muted,
                        width: video_init.width,
                        height: video_init.height,
                        last_frame_time: Instant::now(),
                        order,
                    },
                );
                self.reselect_video_track();
            }
            MessageType::VideoFrame(video_frame) => {
                self.reselect_video_track(); // cheap; applies speaker-hold expiry
                let Some(track_info) = self.active_video_tracks.get_mut(&video_frame.sid) else {
                    // Frames can race ahead of InitVideo or trail VideoTrackEnded.
                    return;
                };
                track_info.last_frame_time = Instant::now();

                // Filter blocked users (wallet-identity publishers only)
                if let Some(address) = track_info.identity_h160 {
                    if self.cached_blocked.contains(&address) {
                        return;
                    }
                }

                // Forward only the selected track to the scenes' livekit video players
                if self.selected_video_sid.as_deref() != Some(video_frame.sid.as_str()) {
                    return;
                }
                use crate::godot_classes::dcl_global::DclGlobal;
                let mut scene_runner = DclGlobal::singleton().bind().scene_runner.clone();
                let mut scene_runner = scene_runner.bind_mut();

                for (_, scene) in scene_runner.get_all_scenes_mut().iter_mut() {
                    scene.process_livekit_video_frame(
                        video_frame.width,
                        video_frame.height,
                        &video_frame.data,
                    );
                }
            }
            MessageType::VideoTrackEnded(sid) => {
                if self.active_video_tracks.remove(&sid).is_some() {
                    tracing::debug!("Video track {} ended", sid);
                }
                if self.selected_video_sid.as_deref() == Some(sid.as_str()) {
                    self.selected_video_sid = None;
                }
                self.reselect_video_track();
            }
            MessageType::VideoTrackMuted { sid, muted } => {
                if let Some(track_info) = self.active_video_tracks.get_mut(&sid) {
                    track_info.muted = muted;
                    self.reselect_video_track();
                }
            }
            MessageType::ActiveSpeakersChanged(speakers) => {
                self.active_speakers = speakers;
                self.reselect_video_track();
            }
            MessageType::InitStreamerAudio(audio_init) => {
                tracing::debug!(
                    "InitStreamerAudio: sample_rate={}, channels={}, samples_per_channel={}",
                    audio_init.sample_rate,
                    audio_init.num_channels,
                    audio_init.samples_per_channel
                );

                // Forward to all scenes to initialize their video player audio
                use crate::godot_classes::dcl_global::DclGlobal;
                let mut scene_runner = DclGlobal::singleton().bind().scene_runner.clone();
                let mut scene_runner = scene_runner.bind_mut();

                for (_, scene) in scene_runner.get_all_scenes_mut().iter_mut() {
                    scene.init_livekit_audio(
                        audio_init.sample_rate,
                        audio_init.num_channels,
                        audio_init.samples_per_channel,
                    );
                }
            }
            MessageType::StreamerAudioFrame(audio_frame) => {
                // Convert i16 audio data to PackedVector2Array (same as voice chat)
                let frame = godot::prelude::PackedVector2Array::from_iter(
                    audio_frame.data.iter().map(|c| {
                        let val = (*c as f32) / (i16::MAX as f32);
                        godot::prelude::Vector2 { x: val, y: val }
                    }),
                );

                // Forward to all scenes
                use crate::godot_classes::dcl_global::DclGlobal;
                let mut scene_runner = DclGlobal::singleton().bind().scene_runner.clone();
                let mut scene_runner = scene_runner.bind_mut();

                for (_, scene) in scene_runner.get_all_scenes_mut().iter_mut() {
                    scene.process_livekit_audio_frame(frame.clone());
                }
            }
            _ => {} // Other message types are not media messages
        }
    }

    /// Re-run stream selection and log when the routed source changes. Switching
    /// resets the speaker-hold timer (mirrors unity-explorer's debounce).
    fn reselect_video_track(&mut self) {
        let hold_elapsed = self.video_switched_at.elapsed() >= MIN_SPEAKER_HOLD;
        let next = best_video_track(
            &self.active_video_tracks,
            self.selected_video_sid.as_deref(),
            &self.active_speakers,
            hold_elapsed,
        );
        if next != self.selected_video_sid {
            tracing::debug!(
                "🎬 livekit video source: {:?} → {:?}",
                self.selected_video_sid,
                next
            );
            self.selected_video_sid = next;
            self.video_switched_at = Instant::now();
        }
    }

    /// Handle non-player participant messages (e.g., "authoritative-server").
    /// Matching bevy's NonPlayerUpdate path: no avatar, no profile, only Scene messages.
    fn process_non_player_message(&mut self, message: IncomingMessage) {
        if let MessageType::Rfc4(rfc4_msg) = message.message {
            if let rfc4::packet::Message::Scene(scene) = rfc4_msg.message {
                tracing::debug!(
                    "📨 Non-player Scene message received for scene '{}' ({} bytes)",
                    scene.scene_id,
                    scene.data.len()
                );

                // Limit the number of scene IDs we track
                if !self.incoming_scene_messages.contains_key(&scene.scene_id)
                    && self.incoming_scene_messages.len() >= MAX_SCENE_IDS
                {
                    if let Some(oldest_key) = self.incoming_scene_messages.keys().next().cloned() {
                        self.incoming_scene_messages.remove(&oldest_key);
                    }
                }

                let entry = self
                    .incoming_scene_messages
                    .entry(scene.scene_id.clone())
                    .or_default();

                if entry.len() >= MAX_SCENE_MESSAGES_PER_SCENE {
                    entry.pop_front();
                }
                entry.push_back((message.address, scene.data));
            } else {
                tracing::debug!(
                    "📨 Non-player non-Scene message ignored from {:#x} (room '{}')",
                    message.address,
                    message.room_id
                );
            }
        }
    }

    fn process_message(&mut self, message: IncomingMessage) {
        // Skip messages from ourselves (can happen if local participant events leak through)
        if message.address == self.player_address {
            return;
        }

        // Room-level events (synthetic H160::zero() address) — handle before peer checks
        if let MessageType::RoomMetadataChanged(ref metadata) = message.message {
            self.handle_room_metadata_changed(metadata);
            return;
        }
        if let MessageType::Disconnected(reason) = &message.message {
            // Set disconnect_reason if not already set (first disconnect wins).
            // Routed here because room-level events use H160::zero() and would
            // otherwise be filtered out by the is_player_address check below.
            if self.disconnect_reason.is_none() {
                self.disconnect_reason = Some((*reason, message.room_id));
            }
            return;
        }

        // Media messages (video/audio from streamers) use synthetic addresses (H160::zero())
        // and must bypass the player address check — they don't need peer lifecycle management.
        match &message.message {
            MessageType::InitVideo(_)
            | MessageType::VideoFrame(_)
            | MessageType::InitStreamerAudio(_)
            | MessageType::StreamerAudioFrame(_)
            | MessageType::VideoTrackEnded(_)
            | MessageType::VideoTrackMuted { .. }
            | MessageType::ActiveSpeakersChanged(_) => {
                self.process_media_message(message);
                return;
            }
            _ => {}
        }

        // Non-player participants (like "authoritative-server" with synthetic address)
        // bypass the full peer lifecycle — no avatar, no profile, only Scene messages.
        // This matches bevy's NonPlayerUpdate path.
        if !Self::is_player_address(message.address) {
            self.process_non_player_message(message);
            return;
        }

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

            tracing::debug!(
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
                    peer_version: None,
                    lambdas_endpoint: None,
                    last_movement_timestamp: f32::NEG_INFINITY,
                    last_emote_incremental_id: 0,
                    pulse_live: false,
                },
            );

            // Brief borrow to add new avatar
            {
                let mut avatar_scene_ref = self.avatars.clone();
                let mut avatar_scene = avatar_scene_ref.bind_mut();
                avatar_scene
                    .add_avatar(new_alias, GString::from(&format!("{:#x}", message.address)));

                // If the user is blocked, hide the avatar immediately
                if self.cached_blocked.contains(&message.address) {
                    tracing::debug!(
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

        // Any pulse-bridged message (except the departure itself) marks the peer as live on
        // Pulse, engaging the transport-preference gate; the flip resets the dedup layers.
        if room_id == PULSE_ROOM_ID && !matches!(message.message, MessageType::PeerLeft) {
            self.set_peer_pulse_live(message.address, true);
        }

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
            // Media messages (InitVideo, VideoFrame, InitStreamerAudio, StreamerAudioFrame)
            // are handled early in process_message() via process_media_message() before
            // the peer lifecycle check, so they never reach this match block.
            MessageType::InitVideo(_)
            | MessageType::VideoFrame(_)
            | MessageType::InitStreamerAudio(_)
            | MessageType::StreamerAudioFrame(_)
            | MessageType::VideoTrackEnded(_)
            | MessageType::VideoTrackMuted { .. }
            | MessageType::ActiveSpeakersChanged(_) => {
                unreachable!("Media messages are handled before peer lifecycle check");
            }
            MessageType::Rfc4(rfc4_msg) => {
                // Handle RFC4 messages
                self.handle_rfc4_message(
                    rfc4_msg.message.clone(),
                    peer_alias,
                    message.address,
                    &room_id,
                );
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
            MessageType::Disconnected(_) => {
                // Handled earlier in process_message() as a room-level event before
                // the per-peer routing — this arm is unreachable.
            }
            MessageType::PeerMetadata(ref metadata) => {
                // Parse metadata JSON to extract version and catalyst info
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(metadata) {
                    if let Some(version) = json.get("dcl_version").and_then(|v| v.as_str()) {
                        tracing::debug!(
                            "Received version metadata from {:#x}: {}",
                            message.address,
                            version
                        );
                        if let Some(peer) = self.peer_identities.get_mut(&message.address) {
                            peer.peer_version = Some(version.to_string());
                            // Only show version label for non-production builds
                            if !DclGlobal::is_production() {
                                let mut avatar_scene_ref = self.avatars.clone();
                                let address_str = format!("{:#x}", message.address);
                                avatar_scene_ref.call(
                                    "set_avatar_version_by_address",
                                    &[
                                        GString::from(&address_str).to_variant(),
                                        GString::from(version).to_variant(),
                                    ],
                                );
                            }
                        }
                    }
                    if let Some(endpoint) = json
                        .get("lambdasEndpoint")
                        .and_then(|v| v.as_str())
                        .and_then(Self::sanitize_lambdas_endpoint)
                    {
                        if let Some(peer) = self.peer_identities.get_mut(&message.address) {
                            if peer.lambdas_endpoint.as_deref() != Some(endpoint) {
                                tracing::debug!(
                                    "Peer {:#x} lambdas endpoint: {}",
                                    message.address,
                                    endpoint
                                );
                                peer.lambdas_endpoint = Some(endpoint.to_string());
                                // Reset fetch state so we retry with the new endpoint
                                peer.profile_fetch_attempted = false;
                                peer.profile_fetch_failures = 0;
                                peer.profile_fetch_banned_until = None;
                            }
                        }
                    }
                }
            }
            // Handled via early return at the top of process_message()
            MessageType::RoomMetadataChanged(_) => {}
        }
    }

    /// Parse room metadata JSON for `bannedAddresses` and check if the local
    /// player is in the list.  Metadata format (from comms-gatekeeper):
    /// `{"bannedAddresses": ["0xabc...", "0xdef..."]}`
    fn handle_room_metadata_changed(&mut self, metadata: &str) {
        let Ok(json) = serde_json::from_str::<serde_json::Value>(metadata) else {
            return;
        };
        let Some(banned) = json.get("bannedAddresses").and_then(|v| v.as_array()) else {
            return;
        };

        let local_addr = format!("{:#x}", self.player_address);
        let local_addr_no_prefix = &local_addr[2..]; // strip "0x"
        let is_banned = banned.iter().any(|v| {
            v.as_str().is_some_and(|s| {
                s.eq_ignore_ascii_case(&local_addr) || s.eq_ignore_ascii_case(local_addr_no_prefix)
            })
        });

        if is_banned {
            tracing::warn!(
                "Room metadata indicates local player {:#x} is banned",
                self.player_address
            );
            self.room_metadata_banned = true;
        }
    }

    /// Flip a peer's transport preference. On EITHER flip direction, both dedup layers (the
    /// per-peer fields here and the per-alias state in AvatarScene) reset: LiveKit and Pulse
    /// timestamps come from incomparable clocks, so stale dedup state from the previous source
    /// would permanently starve the new one (see `Peer::pulse_live`).
    fn set_peer_pulse_live(&mut self, address: H160, live: bool) {
        let Some(peer) = self.peer_identities.get_mut(&address) else {
            return;
        };
        if peer.pulse_live == live {
            return;
        }
        peer.pulse_live = live;
        peer.last_movement_timestamp = f32::NEG_INFINITY;
        peer.last_emote_incremental_id = 0;
        let alias = peer.alias;
        tracing::debug!(
            "🔀 Peer {:#x} (alias: {}) transport preference → {}",
            address,
            alias,
            if live { "pulse" } else { "livekit" }
        );
        let mut avatar_scene_ref = self.avatars.clone();
        avatar_scene_ref.bind_mut().reset_movement_dedup(alias);
    }

    fn handle_peer_left(&mut self, address: H160, room_id: String) {
        // Leaving the pulse room (a real PlayerLeft or PulseRoom's teardown flood) hands the
        // peer back to LiveKit-driven rendering within this same frame.
        if room_id == PULSE_ROOM_ID {
            self.set_peer_pulse_live(address, false);
        }
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
                tracing::debug!(
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

                // Clean up video tracks published under this peer's identity
                self.active_video_tracks
                    .retain(|_, info| info.identity_h160 != Some(address));
                if let Some(sid) = self.selected_video_sid.clone() {
                    if !self.active_video_tracks.contains_key(&sid) {
                        self.selected_video_sid = None;
                        self.reselect_video_track();
                    }
                }
            }
        }
    }

    fn handle_rfc4_message(
        &mut self,
        message: rfc4::packet::Message,
        peer_alias: u32,
        address: H160,
        room_id: &str,
    ) {
        // Transport-preference gate: while a peer is live on Pulse, its avatar-sync messages
        // from LiveKit rooms are discarded outright (see `Peer::pulse_live` for why merging is
        // impossible). Chat/Scene/ProfileRequest/Response never ride Pulse and ProfileVersion
        // is idempotent — none of those are gated.
        if room_id != PULSE_ROOM_ID
            && Self::is_gated_by_pulse_preference(&message)
            && self
                .peer_identities
                .get(&address)
                .is_some_and(|peer| peer.pulse_live)
        {
            tracing::trace!(
                "🔀 Discarding LiveKit avatar-sync message from pulse-live peer {:#x} (room '{}')",
                address,
                room_id
            );
            return;
        }

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
                // Deduplicate: skip if timestamp is not newer (dual-room broadcasting).
                // NOT for the pulse room: its movements are already strictly ordered by the
                // decoder's per-subject sequence window, and its timestamps are the server
                // tick in f32 seconds — at large server uptimes (>~2^20 s) the f32 ULP
                // exceeds the 100 ms packet interval, so consecutive updates quantize equal
                // and a `<=` check here would silently drop them.
                if room_id != PULSE_ROOM_ID {
                    if let Some(peer) = self.peer_identities.get_mut(&address) {
                        if movement.timestamp <= peer.last_movement_timestamp {
                            tracing::debug!(
                                "Discarding duplicate Movement from {:#x}: timestamp {} <= {}",
                                address,
                                movement.timestamp,
                                peer.last_movement_timestamp
                            );
                            return;
                        }
                        peer.last_movement_timestamp = movement.timestamp;
                    }
                }

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

                // Deduplicate: skip if timestamp is not newer (dual-room broadcasting)
                let timestamp = movement.temporal.timestamp_f32();
                if let Some(peer) = self.peer_identities.get_mut(&address) {
                    if timestamp <= peer.last_movement_timestamp {
                        tracing::debug!(
                            "Discarding duplicate MovementCompressed from {:#x}: timestamp {} <= {}",
                            address,
                            timestamp,
                            peer.last_movement_timestamp
                        );
                        return;
                    }
                    peer.last_movement_timestamp = timestamp;
                }

                // Get position from compressed movement with configured realm bounds
                let pos = movement.position(self.realm_min, self.realm_max);
                let velocity = movement.velocity();
                let rotation_rad = movement.temporal.rotation_f32();

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
                tracing::debug!("Received Chat from {:#x}: {:?}", address, chat);

                // Check if user is muted for chat (using cached set for O(1) lookup)
                // Note: cached_muted includes both muted AND blocked users
                if self.cached_muted.contains(&address) {
                    tracing::debug!("Ignoring muted {:#x}", address);
                    return; // muted/blocked - ignore chat messages
                }

                // Check for duplicate messages based on timestamp
                // Check if we've seen a recent message from this sender
                if let Some(&last_timestamp) = self.last_chat_timestamps.get(&address) {
                    // If the new timestamp is older or within tolerance of the last one, it's a duplicate
                    if compare_f64(&chat.timestamp, &last_timestamp) != Ordering::Greater {
                        tracing::debug!(
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
                        message: format!(
                            "{}...",
                            truncate_utf8_safe(&chat.message, MAX_CHAT_MESSAGE_SIZE)
                        ),
                        ..chat
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

                // Deduplicate: skip if same version already announced and fetch attempted (dual-room broadcasting)
                if let Some(peer) = self.peer_identities.get(&address) {
                    if peer.announced_version == Some(announced_version)
                        && peer.profile_fetch_attempted
                    {
                        tracing::debug!(
                            "Discarding duplicate ProfileVersion from {:#x}: version {} already being processed",
                            address,
                            announced_version
                        );
                        return;
                    }
                }

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
                    tracing::debug!(
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

                    // Use the peer's lambdasEndpoint (from LiveKit metadata) as the primary
                    // fetch target — goes directly to the catalyst the peer deployed to,
                    // avoiding cross-catalyst propagation delays (issue #1856).
                    let peer_lambdas_endpoint = self
                        .peer_identities
                        .get(&address)
                        .and_then(|p| p.lambdas_endpoint.clone());

                    TokioRuntime::spawn(async move {
                        // Unity CatalystRetryPolicy: 6 retries, base 2s, multiplier 2x, capped at 60s
                        // Retry if profile is None OR fetched version < announced version (version mismatch)
                        const MAX_RETRIES: u32 = 6;
                        const BASE_DELAY_MS: u64 = 2000;
                        const BACKOFF_MULTIPLIER: u64 = 2;
                        const MAX_DELAY_MS: u64 = 60_000;

                        let version_ok = |r: &Result<UserProfile, _>| matches!(r, Ok(p) if p.version >= announced_version_for_retry);

                        // Determine fetch endpoint: peer's lambdas endpoint if available, else realm lambda
                        let mut fetch_endpoint = match peer_lambdas_endpoint.as_deref() {
                            Some(endpoint)
                                if !Self::is_same_lambda_endpoint(
                                    endpoint,
                                    &lamda_server_base_url,
                                ) =>
                            {
                                endpoint.to_string()
                            }
                            _ => lamda_server_base_url.clone(),
                        };

                        // Step 1: fetch with Unity-matching retry policy (exponential backoff)
                        let mut result = Err(anyhow::anyhow!("not started"));
                        for attempt in 0..=MAX_RETRIES {
                            if attempt > 0 {
                                let delay = (BASE_DELAY_MS * BACKOFF_MULTIPLIER.pow(attempt - 1))
                                    .min(MAX_DELAY_MS);
                                tracing::debug!(
                                    "profile fetch retry {}/{} for {:#x} in {}ms",
                                    attempt,
                                    MAX_RETRIES,
                                    address,
                                    delay
                                );
                                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                            }
                            result = request_lambda_profile(
                                address,
                                fetch_endpoint.as_str(),
                                profile_base_url.as_str(),
                                http_requester.clone(),
                            )
                            .await;
                            if version_ok(&result) {
                                break;
                            }
                            // A hard error from the peer-advertised endpoint (bad metadata,
                            // unreachable catalyst) must not burn the whole retry chain:
                            // switch to the realm endpoint for the remaining attempts and
                            // try it right away. An Ok-but-stale response keeps retrying
                            // the same endpoint — that's catalyst propagation lag, not a
                            // broken URL.
                            if result.is_err()
                                && !Self::is_same_lambda_endpoint(
                                    &fetch_endpoint,
                                    &lamda_server_base_url,
                                )
                            {
                                tracing::debug!(
                                    "peer lambdas endpoint {} failed for {:#x}, falling back to realm endpoint {}",
                                    fetch_endpoint,
                                    address,
                                    lamda_server_base_url
                                );
                                fetch_endpoint = lamda_server_base_url.clone();
                                result = request_lambda_profile(
                                    address,
                                    fetch_endpoint.as_str(),
                                    profile_base_url.as_str(),
                                    http_requester.clone(),
                                )
                                .await;
                                if version_ok(&result) {
                                    break;
                                }
                            }
                        }

                        // Step 2: asset-bundle-registry fallback (for legacy clients without lambdasEndpoint)
                        let result = if version_ok(&result) {
                            result
                        } else {
                            tracing::debug!("Trying asset-bundle-registry for {:#x}", address);
                            request_registry_profile(
                                address,
                                profile_base_url.as_str(),
                                http_requester.clone(),
                            )
                            .await
                        };

                        // Step 3: realm lambda fallback (if peer endpoint != realm and registry also failed)
                        let result = if version_ok(&result) {
                            result
                        } else if !Self::is_same_lambda_endpoint(
                            &fetch_endpoint,
                            &lamda_server_base_url,
                        ) {
                            tracing::debug!(
                                "Falling back to realm lambda for {:#x}: {}",
                                address,
                                lamda_server_base_url
                            );
                            request_lambda_profile(
                                address,
                                lamda_server_base_url.as_str(),
                                profile_base_url.as_str(),
                                http_requester,
                            )
                            .await
                        } else {
                            result
                        };

                        if let Ok(profile) = result {
                            tracing::debug!(
                                "Fetched profile from lambda for {:#x}: version {}",
                                address,
                                profile.version
                            );
                            // Check if the fetched version matches what was announced
                            let version_mismatch = profile.version < announced_version_for_retry;
                            if version_mismatch {
                                tracing::debug!(
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
                            tracing::debug!(
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
                                tracing::debug!(
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
                    tracing::debug!(
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

                tracing::debug!(
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

                    tracing::debug!(
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
                        tracing::debug!(
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
                        tracing::debug!(
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
                // A stop signal ends the looping emote. Handled BEFORE the incremental-id
                // dedup: a stop must neither depend on nor affect id ordering (Pulse stops
                // carry no meaningful id; Unity's LiveKit stops reuse the start's id).
                if player_emote.is_stopping == Some(true) {
                    tracing::debug!("Received PlayerEmote stop from {:#x}", address);
                    let mut avatar_scene_ref = self.avatars.clone();
                    avatar_scene_ref.bind_mut().stop_emote(peer_alias);
                    return;
                }

                // Deduplicate: skip if incremental_id is not newer (dual-room broadcasting)
                if let Some(peer) = self.peer_identities.get_mut(&address) {
                    if player_emote.incremental_id <= peer.last_emote_incremental_id {
                        tracing::debug!(
                            "Discarding duplicate PlayerEmote from {:#x}: id {} <= {}",
                            address,
                            player_emote.incremental_id,
                            peer.last_emote_incremental_id
                        );
                        return;
                    }
                    peer.last_emote_incremental_id = player_emote.incremental_id;
                }

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
            rfc4::packet::Message::LookAtPosition(_)
            | rfc4::packet::Message::Reaction(_)
            | rfc4::packet::Message::ChatReaction(_) => {}
        }
    }

    pub fn consume_chats(&mut self) -> Vec<(H160, rfc4::Chat)> {
        self.chats.drain(..).collect()
    }

    pub fn consume_scene_messages(&mut self, scene_id: &str) -> Vec<(H160, Vec<u8>)> {
        if let Some(messages) = self.incoming_scene_messages.get_mut(scene_id) {
            let result: Vec<_> = messages.drain(..).collect();
            if !result.is_empty() {
                tracing::debug!(
                    "📤 consume_scene_messages: delivering {} messages for scene '{}'",
                    result.len(),
                    scene_id
                );
            }
            result
        } else {
            if !self.incoming_scene_messages.is_empty() {
                tracing::debug!(
                    "📤 consume_scene_messages: scene '{}' not found, available keys: {:?}",
                    scene_id,
                    self.incoming_scene_messages.keys().collect::<Vec<_>>()
                );
            }
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

    /// Returns room connectivity info for each peer.
    /// Each entry is (address, room_description, name) where room_description lists
    /// every room the peer is seen in, joined with " + " — e.g. "PULSE + SCENE +
    /// ARCHIPELAGO", "SCENE + ARCHIPELAGO", "PULSE", or "NONE". A trailing '*' on
    /// PULSE marks that Pulse is the source currently driving the avatar
    /// (transport-preference gate). `name` is the profile display name, empty while
    /// the profile hasn't been resolved yet.
    pub fn get_peer_room_info(&self) -> Vec<(H160, String, String)> {
        let mut result = Vec::new();
        for (address, peer) in &self.peer_identities {
            let mut has_scene = false;
            let mut has_archipelago = false;
            let mut has_pulse = false;
            for room_id in peer.room_activity.keys() {
                if room_id.starts_with("scene-") {
                    has_scene = true;
                } else if room_id == PULSE_ROOM_ID {
                    has_pulse = true;
                } else {
                    has_archipelago = true;
                }
            }
            let mut parts: Vec<&str> = Vec::new();
            if has_pulse {
                // Mark which source is actually driving the avatar right now.
                parts.push(if peer.pulse_live { "PULSE*" } else { "PULSE" });
            }
            if has_scene {
                parts.push("SCENE");
            }
            if has_archipelago {
                parts.push("ARCHIPELAGO");
            }
            let room_desc = if parts.is_empty() {
                "NONE".to_string()
            } else {
                parts.join(" + ")
            };
            let name = peer
                .profile
                .as_ref()
                .map(|profile| profile.content.name.clone())
                .unwrap_or_default();
            result.push((*address, room_desc, name));
        }
        result
    }
}

// MessageProcessor itself needs a live Godot engine (Gd<AvatarScene>), so the full
// interleaved pulse/livekit sequences are covered by the cross-client QA matrix; what IS
// engine-free — the gate's message classification — is pinned here.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pulse_preference_gates_exactly_the_avatar_sync_slice() {
        use rfc4::packet::Message;

        // Gated: the avatar-sync slice that rides Pulse.
        assert!(MessageProcessor::is_gated_by_pulse_preference(
            &Message::Movement(rfc4::Movement::default())
        ));
        assert!(MessageProcessor::is_gated_by_pulse_preference(
            &Message::MovementCompressed(rfc4::MovementCompressed::default())
        ));
        assert!(MessageProcessor::is_gated_by_pulse_preference(
            &Message::Position(rfc4::Position::default())
        ));
        assert!(MessageProcessor::is_gated_by_pulse_preference(
            &Message::PlayerEmote(rfc4::PlayerEmote::default())
        ));

        // Never gated: doesn't ride Pulse, or idempotent.
        assert!(!MessageProcessor::is_gated_by_pulse_preference(
            &Message::Chat(rfc4::Chat::default())
        ));
        assert!(!MessageProcessor::is_gated_by_pulse_preference(
            &Message::Scene(rfc4::Scene::default())
        ));
        assert!(!MessageProcessor::is_gated_by_pulse_preference(
            &Message::ProfileVersion(rfc4::AnnounceProfileVersion::default())
        ));
        assert!(!MessageProcessor::is_gated_by_pulse_preference(
            &Message::ProfileRequest(rfc4::ProfileRequest::default())
        ));
        assert!(!MessageProcessor::is_gated_by_pulse_preference(
            &Message::ProfileResponse(rfc4::ProfileResponse::default())
        ));
    }

    #[test]
    fn lambda_endpoint_comparison_ignores_trailing_slash_style() {
        // Godot metadata carries `…/lambdas/`, Unity metadata carries `…/lambdas` —
        // same catalyst, must compare equal so the realm endpoint isn't treated
        // as a different fetch target.
        assert!(MessageProcessor::is_same_lambda_endpoint(
            "https://peer.decentraland.org/lambdas",
            "https://peer.decentraland.org/lambdas/"
        ));
        assert!(MessageProcessor::is_same_lambda_endpoint(
            "https://peer.decentraland.org/lambdas/",
            "https://peer.decentraland.org/lambdas/"
        ));
        assert!(!MessageProcessor::is_same_lambda_endpoint(
            "https://peer.decentraland.org/lambdas",
            "https://peer-ec2.decentraland.org/lambdas/"
        ));
    }

    #[test]
    fn sanitize_lambdas_endpoint_accepts_only_plausible_http_urls() {
        assert_eq!(
            MessageProcessor::sanitize_lambdas_endpoint(" https://peer.decentraland.org/lambdas "),
            Some("https://peer.decentraland.org/lambdas")
        );
        assert_eq!(
            MessageProcessor::sanitize_lambdas_endpoint("http://localhost:7070/lambdas/"),
            Some("http://localhost:7070/lambdas/")
        );
        assert_eq!(MessageProcessor::sanitize_lambdas_endpoint(""), None);
        assert_eq!(
            MessageProcessor::sanitize_lambdas_endpoint("not a url"),
            None
        );
        assert_eq!(
            MessageProcessor::sanitize_lambdas_endpoint("ftp://peer.decentraland.org/lambdas"),
            None
        );
        assert_eq!(
            MessageProcessor::sanitize_lambdas_endpoint("https://"),
            None
        );
        assert_eq!(
            MessageProcessor::sanitize_lambdas_endpoint("https:///lambdas"),
            None
        );
        assert_eq!(
            MessageProcessor::sanitize_lambdas_endpoint("https://peer.decentraland.org/lam bdas"),
            None
        );
    }

    // --- best_video_track: mirror of unity-explorer LivekitPlayer selection ---

    fn track(
        identity: &str,
        source: VideoTrackSourceKind,
        muted: bool,
        order: u64,
    ) -> VideoTrackInfo {
        VideoTrackInfo {
            identity: identity.to_string(),
            identity_h160: identity.as_h160(),
            source,
            muted,
            width: 640,
            height: 360,
            last_frame_time: Instant::now(),
            order,
        }
    }

    fn tracks(entries: Vec<(&str, VideoTrackInfo)>) -> HashMap<String, VideoTrackInfo> {
        entries
            .into_iter()
            .map(|(sid, info)| (sid.to_string(), info))
            .collect()
    }

    #[test]
    fn presentation_bot_outranks_screenshare_and_camera() {
        let map = tracks(vec![
            (
                "cam",
                track("0xaaaa", VideoTrackSourceKind::Camera, false, 0),
            ),
            (
                "share",
                track("stream:p:1", VideoTrackSourceKind::Screenshare, false, 1),
            ),
            (
                "bot",
                track("presentation-bot:r", VideoTrackSourceKind::Camera, false, 2),
            ),
        ]);
        assert_eq!(
            best_video_track(&map, None, &[], true),
            Some("bot".to_string())
        );
    }

    #[test]
    fn screenshare_outranks_camera_but_muted_share_is_skipped() {
        let map = tracks(vec![
            (
                "cam",
                track("0xaaaa", VideoTrackSourceKind::Camera, false, 0),
            ),
            (
                "share",
                track("stream:p:1", VideoTrackSourceKind::Screenshare, false, 1),
            ),
        ]);
        assert_eq!(
            best_video_track(&map, None, &[], true),
            Some("share".to_string())
        );

        let map = tracks(vec![
            (
                "cam",
                track("0xaaaa", VideoTrackSourceKind::Camera, false, 0),
            ),
            (
                "share",
                track("stream:p:1", VideoTrackSourceKind::Screenshare, true, 1),
            ),
        ]);
        assert_eq!(
            best_video_track(&map, None, &[], true),
            Some("cam".to_string())
        );
    }

    #[test]
    fn follows_dominant_active_speaker_with_video_after_hold() {
        let map = tracks(vec![
            (
                "cam_a",
                track("0xaaaa", VideoTrackSourceKind::Camera, false, 0),
            ),
            (
                "cam_b",
                track("0xbbbb", VideoTrackSourceKind::Camera, false, 1),
            ),
        ]);
        let speakers = vec!["0xbbbb".to_string()];
        assert_eq!(
            best_video_track(&map, Some("cam_a"), &speakers, true),
            Some("cam_b".to_string())
        );
        // Hold not elapsed: stays on the current track.
        assert_eq!(
            best_video_track(&map, Some("cam_a"), &speakers, false),
            Some("cam_a".to_string())
        );
    }

    #[test]
    fn keeps_current_when_dominant_speaker_is_already_playing() {
        let map = tracks(vec![
            (
                "cam_a",
                track("0xaaaa", VideoTrackSourceKind::Camera, false, 0),
            ),
            (
                "cam_b",
                track("0xbbbb", VideoTrackSourceKind::Camera, false, 1),
            ),
        ]);
        // Current speaker is dominant; the runner-up must NOT steal the stream.
        let speakers = vec!["0xaaaa".to_string(), "0xbbbb".to_string()];
        assert_eq!(
            best_video_track(&map, Some("cam_a"), &speakers, true),
            Some("cam_a".to_string())
        );
    }

    #[test]
    fn speaker_without_video_falls_through_to_next_speaker() {
        let map = tracks(vec![(
            "cam_b",
            track("0xbbbb", VideoTrackSourceKind::Camera, false, 0),
        )]);
        let speakers = vec!["0xcccc".to_string(), "0xbbbb".to_string()];
        assert_eq!(
            best_video_track(&map, None, &speakers, true),
            Some("cam_b".to_string())
        );
    }

    #[test]
    fn falls_back_to_first_available_when_current_track_died() {
        let map = tracks(vec![
            (
                "cam_b",
                track("0xbbbb", VideoTrackSourceKind::Camera, false, 5),
            ),
            (
                "cam_c",
                track("0xcccc", VideoTrackSourceKind::Camera, false, 2),
            ),
        ]);
        assert_eq!(
            best_video_track(&map, Some("dead_sid"), &[], true),
            Some("cam_c".to_string())
        );
    }

    #[test]
    fn no_tracks_yields_none() {
        let map: HashMap<String, VideoTrackInfo> = HashMap::new();
        assert_eq!(best_video_track(&map, None, &[], true), None);
    }
}
