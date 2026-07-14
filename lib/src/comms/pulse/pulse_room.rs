//! The Pulse protocol layer — godot-explorer's re-expression of bevy-explorer's
//! `crates/comms/src/pulse/plugin.rs` @ 3f65c164 (Bevy ECS systems → one struct polled per frame
//! from `CommunicationManager::process()`).
//!
//! Owns the [`PulseDecoder`] and the driver lifecycle, and pumps the byte boundary: inbound
//! `ServerMessage` bytes are decoded and bridged into the shared `MessageProcessor` as ordinary
//! `IncomingMessage`s with `room_id = "pulse"` — downstream (avatars, profiles, emotes) never
//! learns Pulse exists. Outbound gameplay (`PlayerStateInput`, emotes, teleports, profile
//! versions) is encoded onto the driver.
//!
//! Connect sequence (mirroring the Unity reference client): once the driver reports
//! [`PulseStatus::Connected`] we sign and send a `HandshakeRequest` (auth chain in the
//! `x-identity-*` header-dictionary shape, delivered as protobuf bytes); on the server's
//! `HandshakeResponse` the room reports [`PulseRoomEvent::Established`] and the
//! `CommunicationManager` sends the first gameplay message — a `TeleportRequest` announcing
//! realm + position, which is what makes the server start streaming same-realm peers.
//!
//! Reconnection model: a driver runs exactly one connection attempt and, when that attempt ends,
//! drops its channel ends. The protocol layer treats that *pipe close* — not the advisory
//! `Disconnected(reason)` message — as the authoritative "transport is gone" signal, and rebuilds
//! the whole driver/link from `Down` (unless the reason was terminal, in which case it parks in
//! `Dead` until the room itself is rebuilt on the next realm change).

use std::time::{Duration, Instant};

use ethers_core::types::H160;
use godot::prelude::Vector3;
use prost::Message as _;
use tokio::sync::mpsc;

use super::decoder::{from_movement, PulseDecoder, PulseEvent, PulseParcelGrid};
use super::transport::{
    self, PulseDriverHandle, PulseFrame, PulseLink, PulseReliability, PulseStatus,
    PulseTransportConfig,
};
use super::PULSE_ROOM_ID;
use crate::auth::ephemeral_auth_chain::EphemeralAuthChain;
use crate::auth::wallet::sign_pulse_connect;
use crate::comms::adapter::message_processor::{IncomingMessage, MessageType, Rfc4Message};
use crate::comms::consts::DEFAULT_PROTOCOL_VERSION;
use crate::dcl::components::proto_components::{kernel::comms::rfc4, pulse};
use crate::scene_runner::tokio_runtime::TokioRuntime;

/// Cooldown before re-attempting after any retryable failure (no identity yet, sign error,
/// response timeout, server rejection, or a retryable disconnect).
pub const RETRY_COOLDOWN: Duration = Duration::from_secs(2);
/// How long to wait for the server's `HandshakeResponse` before assuming it was lost and retrying.
pub const HANDSHAKE_RESPONSE_TIMEOUT: Duration = Duration::from_secs(5);
/// Byte-boundary channel capacity (outbound frames / inbound messages).
const LINK_CAPACITY: usize = 1024;

/// The connection's lifecycle — a single linear progression with two off-ramps (`Down` to
/// rebuild, `Dead` to give up). The driver/link are live from `Connecting` through `Established`;
/// `Down` and `Dead` have no driver.
enum Connection {
    /// No live driver. The next poll after `respawn_at` (re)builds one. This is both the initial
    /// state and where a retryable transport drop lands.
    Down { respawn_at: Instant },
    /// Driver up, waiting for it to report `Connected` (the ENet connect completing).
    Connecting,
    /// Transport connected, ready to sign once an identity is present and the cooldown elapses.
    Idle { retry_after: Instant },
    /// Signing the auth chain off-thread; the encoded handshake arrives on `handshake_rx`.
    /// Re-signed on each attempt so the connect-signature timestamp is fresh when sent.
    Signing,
    /// Handshake sent; awaiting the server's `HandshakeResponse` until `timeout_at`.
    AwaitingResponse { timeout_at: Instant },
    /// Handshake accepted; steady state.
    Established,
    /// Terminally disconnected (auth rejected, banned, evicted, flagged) — no reconnect attempted.
    /// Only rebuilding the room (realm change / `change_adapter`) clears this.
    Dead,
}

/// What `poll()` surfaces up to `CommunicationManager`.
#[derive(Debug, PartialEq, Eq)]
pub enum PulseRoomEvent {
    /// Handshake accepted. Caller resets the session failure counter and sends the initial
    /// `TeleportRequest` (realm + position) — the mandatory first gameplay message.
    Established,
    /// A connection attempt ended before ever establishing (connect failure, pipe close,
    /// handshake never acked). Caller counts it toward the session-fallback strike limit.
    AttemptFailed,
    /// Terminal disconnect — the room is parked in `Dead` and will not reconnect.
    Dead,
}

pub struct PulseRoom {
    /// Byte boundary to the current driver. `None` between attempts (`Down`/`Dead`).
    link: Option<PulseLink>,
    /// Current driver; dropping it stops + joins the `pulse-enet` thread. Replaced wholesale on
    /// every (re)connect.
    driver: Option<PulseDriverHandle>,
    decoder: PulseDecoder,
    grid: PulseParcelGrid,
    config: PulseTransportConfig,
    state: Connection,
    /// Whether the current driver attempt ever reached `Established` — a drop before that counts
    /// as a session-fallback strike.
    established_this_attempt: bool,

    /// Signed handshake bytes arriving from the off-thread signing task.
    handshake_rx: mpsc::Receiver<Result<Vec<u8>, String>>,
    handshake_tx: mpsc::Sender<Result<Vec<u8>, String>>,

    /// Sink into the shared `MessageProcessor` — the same channel every room feeds.
    processor_sender: mpsc::Sender<IncomingMessage>,

    /// Last `PlayerState` sent, cached by [`Self::send_movement`]. An outbound `EmoteStart`
    /// attaches this — the server rejects an emote with a null `player_state`.
    last_state: Option<pulse::PlayerState>,
    /// Last profile version announced over Pulse; announcements are reliable + stored
    /// server-side, so only send on change (the handshake itself carries the connect-time one).
    last_announced_profile_version: Option<u32>,
    /// Log the out-of-grid suppression only once per excursion.
    warned_out_of_grid: bool,
}

impl PulseRoom {
    pub fn new(
        config: PulseTransportConfig,
        processor_sender: mpsc::Sender<IncomingMessage>,
    ) -> Self {
        let (handshake_tx, handshake_rx) = mpsc::channel(4);
        let grid = PulseParcelGrid::default();
        Self {
            link: None,
            driver: None,
            decoder: PulseDecoder::new(grid),
            grid,
            config,
            state: Connection::Down {
                respawn_at: Instant::now(),
            },
            established_this_attempt: false,
            handshake_rx,
            handshake_tx,
            processor_sender,
            last_state: None,
            last_announced_profile_version: None,
            warned_out_of_grid: false,
        }
    }

    pub fn is_established(&self) -> bool {
        matches!(self.state, Connection::Established)
    }

    /// Drive the room one frame: drain driver status, advance the connection state machine
    /// (signing when `identity` yields one), then decode + bridge inbound. `identity` is called
    /// at most once, only when a handshake is actually about to be signed; it returns the
    /// ephemeral auth chain and the current profile version.
    pub fn poll(
        &mut self,
        now: Instant,
        identity: impl FnOnce() -> Option<(EphemeralAuthChain, i32)>,
    ) -> Vec<PulseRoomEvent> {
        let mut events = Vec::new();
        self.drain_status(now, &mut events);
        self.drive_connection(now, identity);
        self.drain_inbound(now, &mut events);
        events
    }

    /// Graceful teardown: flood synthetic `PeerLeft`s for every Pulse-known peer (so
    /// LiveKit-driven rendering takes over within a frame) and drop the driver (its `Drop`
    /// disconnects the ENet peer cleanly and joins the thread).
    pub fn clean(&mut self) {
        self.flood_peer_left();
        self.link = None;
        self.driver = None;
        self.state = Connection::Dead;
    }

    fn flood_peer_left(&mut self) {
        for wallet in self.decoder.known_wallets() {
            self.send_to_processor(wallet, MessageType::PeerLeft);
        }
        // Rebuild the decoder to drop all subject state; a future connection starts clean.
        self.decoder = PulseDecoder::new(self.grid);
    }

    // ---- outbound API ------------------------------------------------------------------------

    /// Quantize + send the local movement as a `PlayerStateInput` (unreliable-sequenced, ch1).
    /// `movement` fields are in DCL world convention — the same values the rfc4 packets carry.
    /// Out-of-grid positions are suppressed entirely (invalid parcel index = terminal disconnect).
    pub fn send_movement(&mut self, movement: &rfc4::Movement) {
        if !self.is_established() {
            return;
        }
        let world = Vector3::new(
            movement.position_x,
            movement.position_y,
            movement.position_z,
        );
        if !self.check_in_grid(world) {
            return;
        }
        let state = from_movement(movement, &self.grid);
        self.last_state = Some(state.clone());
        self.send_client_message(
            pulse::client_message::Message::Input(pulse::PlayerStateInput { state: Some(state) }),
            PulseReliability::UnreliableSequenced,
        );
    }

    /// Reliable `EmoteStart`, attaching the last sent `PlayerState` (the server rejects a null
    /// one — skip with a warning if no movement was ever sent on this connection).
    pub fn send_emote_start(&mut self, urn: &str) {
        if !self.is_established() {
            return;
        }
        let Some(state) = self.last_state.clone() else {
            tracing::warn!("pulse: emote before any movement sent; skipping EmoteStart");
            return;
        };
        self.send_client_message(
            pulse::client_message::Message::EmoteStart(pulse::EmoteStart {
                emote_id: urn.to_owned(),
                duration_ms: None,
                player_state: Some(state),
                mask: None,
            }),
            PulseReliability::Reliable,
        );
    }

    /// Reliable `EmoteStop` (looping emotes only; one-shots expire server-side).
    pub fn send_emote_stop(&mut self) {
        if !self.is_established() {
            return;
        }
        self.send_client_message(
            pulse::client_message::Message::EmoteStop(pulse::EmoteStop {}),
            PulseReliability::Reliable,
        );
    }

    /// Reliable `ProfileVersionAnnouncement`, deduped on change (the 10s LiveKit rebroadcast
    /// idiom is noise on a reliable, server-stored channel).
    pub fn send_profile_version(&mut self, version: u32) {
        if !self.is_established() || self.last_announced_profile_version == Some(version) {
            return;
        }
        self.last_announced_profile_version = Some(version);
        self.send_client_message(
            pulse::client_message::Message::ProfileAnnouncement(
                pulse::ProfileVersionAnnouncement {
                    version: version as i32,
                },
            ),
            PulseReliability::Reliable,
        );
    }

    /// Reliable `TeleportRequest` announcing realm + position (DCL world convention). The
    /// mandatory first gameplay message after the handshake, re-sent on instant repositions
    /// (server supports same-realm re-teleports). The caller guards the realm-name validity;
    /// this guards the grid bounds.
    pub fn send_teleport(&mut self, world: Vector3, realm: &str) {
        if !self.check_in_grid(world) {
            return;
        }
        let (parcel_index, local) = self.grid.encode_to_parcel(world);
        use pulse::TeleportRequest as T;
        self.send_client_message(
            pulse::client_message::Message::Teleport(T {
                parcel_index,
                position_x: T::position_x_quantized(local.x),
                position_y: T::position_y_quantized(local.y),
                position_z: T::position_z_quantized(local.z),
                realm: realm.to_owned(),
            }),
            PulseReliability::Reliable,
        );
        tracing::info!("pulse: teleport sent (parcel {parcel_index}, realm {realm})");
    }

    // ---- internals ---------------------------------------------------------------------------

    /// Grid-bounds guard shared by every position-bearing send. Out-of-grid (some Worlds /
    /// preview realms) → suppress and log once per excursion.
    fn check_in_grid(&mut self, world: Vector3) -> bool {
        if self.grid.contains_world(world) {
            self.warned_out_of_grid = false;
            true
        } else {
            if !self.warned_out_of_grid {
                self.warned_out_of_grid = true;
                tracing::warn!(
                    "pulse: position {world:?} outside the pulse parcel grid; suppressing sends until back in bounds"
                );
            }
            false
        }
    }

    fn send_client_message(
        &mut self,
        message: pulse::client_message::Message,
        reliability: PulseReliability,
    ) {
        let Some(link) = self.link.as_ref() else {
            return;
        };
        let bytes = pulse::ClientMessage {
            message: Some(message),
        }
        .encode_to_vec();
        let _ = link.outbound.try_send(PulseFrame { bytes, reliability });
    }

    fn send_to_processor(&self, address: H160, message: MessageType) {
        let _ = self.processor_sender.try_send(IncomingMessage {
            message,
            address,
            room_id: PULSE_ROOM_ID.to_owned(),
        });
    }

    fn send_rfc4_to_processor(&self, address: H160, message: rfc4::packet::Message) {
        self.send_to_processor(
            address,
            MessageType::Rfc4(Rfc4Message {
                message,
                protocol_version: DEFAULT_PROTOCOL_VERSION,
            }),
        );
    }

    /// Drain the driver's status channel into the state machine. A `Disconnected`/`Failed`
    /// status is the driver signing off; a bare channel close means it vanished with no word.
    /// Either way `lost_connection` nulls the link, ending the loop and making a
    /// status-then-close sequence idempotent.
    fn drain_status(&mut self, now: Instant, events: &mut Vec<PulseRoomEvent>) {
        while let Some(status) = self.link.as_mut().map(|link| link.status.try_recv()) {
            match status {
                Ok(PulseStatus::Connecting) => tracing::debug!("pulse: connecting"),
                Ok(PulseStatus::Connected) => {
                    tracing::info!("pulse: connected");
                    if matches!(self.state, Connection::Connecting) {
                        self.state = Connection::Idle { retry_after: now };
                    }
                }
                Ok(PulseStatus::Disconnected(reason)) => {
                    tracing::warn!("pulse: disconnected ({reason:?})");
                    self.lost_connection(reason.should_retry(), now, events);
                }
                // Never established (DNS/socket/connect timeout) — always transient.
                Ok(PulseStatus::Failed(error)) => {
                    tracing::warn!("pulse: failed ({error})");
                    self.lost_connection(true, now, events);
                }
                Err(mpsc::error::TryRecvError::Empty) => break,
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.lost_connection(true, now, events);
                    break;
                }
            }
        }
    }

    /// The transport is gone — tear the driver/link down, surface the peer loss, and decide
    /// what's next: rebuild from `Down` after a cooldown, or park in `Dead` for a terminal
    /// reason. Idempotent: with no live link it's a no-op.
    fn lost_connection(&mut self, retry: bool, now: Instant, events: &mut Vec<PulseRoomEvent>) {
        if self.link.is_none() {
            return;
        }
        self.link = None;
        self.driver = None; // dropping joins the already-exited driver thread
        self.last_state = None;
        self.last_announced_profile_version = None;
        self.flood_peer_left();
        if !self.established_this_attempt {
            events.push(PulseRoomEvent::AttemptFailed);
        }
        self.state = if retry {
            tracing::info!("pulse: transport dropped — reinitialising after cooldown");
            Connection::Down {
                respawn_at: now + RETRY_COOLDOWN,
            }
        } else {
            tracing::warn!("pulse: terminal disconnect — not reconnecting");
            events.push(PulseRoomEvent::Dead);
            Connection::Dead
        };
    }

    /// Advance the connection one step. `Down` (re)builds the driver; `Connecting` waits
    /// passively for the transport-up status; `Idle` waits for an identity + cooldown, then
    /// signs off-thread; each retryable handshake failure folds back to `Idle` (driver still up).
    fn drive_connection(
        &mut self,
        now: Instant,
        identity: impl FnOnce() -> Option<(EphemeralAuthChain, i32)>,
    ) {
        match &self.state {
            Connection::Dead | Connection::Established | Connection::Connecting => {}
            Connection::Down { respawn_at } => {
                if now < *respawn_at {
                    return;
                }
                let (link, channels) = transport::pulse_channels(LINK_CAPACITY);
                let driver = transport::spawn_pulse_driver(self.config.clone(), channels);
                self.link = Some(link);
                self.driver = Some(driver);
                self.established_this_attempt = false;
                self.state = Connection::Connecting;
            }
            Connection::Idle { retry_after } => {
                if now < *retry_after {
                    return;
                }
                let Some((ephemeral, profile_version)) = identity() else {
                    return;
                };
                // Discard any stale signing result from an earlier, torn-down attempt so the
                // handshake we send always carries this attempt's fresh timestamp.
                while self.handshake_rx.try_recv().is_ok() {}
                let tx = self.handshake_tx.clone();
                let sign_task = async move {
                    let result = sign_pulse_connect(&ephemeral).await.map(|auth_chain| {
                        pulse::ClientMessage {
                            message: Some(pulse::client_message::Message::Handshake(
                                pulse::HandshakeRequest {
                                    auth_chain,
                                    profile_version,
                                    initial_state: None,
                                },
                            )),
                        }
                        .encode_to_vec()
                    });
                    let _ = tx.send(result).await;
                };
                // Prefer an ambient tokio runtime (unit tests); the Godot main thread has none,
                // so production goes through the shared TokioRuntime (whose Godot-singleton
                // lookup would panic in a non-engine process).
                if let Ok(handle) = tokio::runtime::Handle::try_current() {
                    handle.spawn(sign_task);
                } else {
                    TokioRuntime::spawn(sign_task);
                }
                self.state = Connection::Signing;
            }
            Connection::Signing => match self.handshake_rx.try_recv() {
                Ok(Ok(bytes)) => {
                    if let Some(link) = self.link.as_ref() {
                        let _ = link.outbound.try_send(PulseFrame {
                            bytes,
                            reliability: PulseReliability::Reliable,
                        });
                    }
                    tracing::info!("pulse: handshake sent");
                    self.state = Connection::AwaitingResponse {
                        timeout_at: now + HANDSHAKE_RESPONSE_TIMEOUT,
                    };
                }
                Ok(Err(err)) => {
                    tracing::warn!("pulse: failed to build handshake, retrying: {err}");
                    self.state = Connection::Idle {
                        retry_after: now + RETRY_COOLDOWN,
                    };
                }
                Err(mpsc::error::TryRecvError::Empty) => {}
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.state = Connection::Idle {
                        retry_after: now + RETRY_COOLDOWN,
                    };
                }
            },
            Connection::AwaitingResponse { timeout_at } => {
                if now > *timeout_at {
                    tracing::warn!("pulse: handshake response timed out, retrying");
                    self.state = Connection::Idle {
                        retry_after: now + RETRY_COOLDOWN,
                    };
                }
            }
        }
    }

    /// Decode + bridge inbound `ServerMessage` bytes into the shared `MessageProcessor`.
    fn drain_inbound(&mut self, now: Instant, events: &mut Vec<PulseRoomEvent>) {
        while let Some(Ok(bytes)) = self.link.as_mut().map(|link| link.inbound.try_recv()) {
            let decoded = match pulse::ServerMessage::decode(bytes.as_slice()) {
                Ok(message) => self.decoder.handle(message),
                Err(err) => {
                    tracing::warn!("pulse: failed to decode ServerMessage: {err}");
                    continue;
                }
            };
            for event in decoded {
                match event {
                    PulseEvent::Connected { success, error } => {
                        self.on_handshake_response(now, success, error, events)
                    }
                    PulseEvent::Movement {
                        address,
                        mut movement,
                        teleport,
                    } => {
                        // A teleport is a discontinuous reposition — snap, don't interpolate.
                        movement.is_instant = teleport;
                        self.send_rfc4_to_processor(
                            address,
                            rfc4::packet::Message::Movement(*movement),
                        );
                    }
                    // The subject↔wallet alias was established: surface the peer and its
                    // connect-time profile version (idempotent downstream).
                    PulseEvent::Joined {
                        address,
                        profile_version,
                        ..
                    } => {
                        self.send_to_processor(address, MessageType::PeerJoined);
                        self.bridge_profile_version(address, profile_version);
                    }
                    // Removes only the "pulse" room membership; the avatar survives if the peer
                    // is still live on LiveKit (multi-room peer lifecycle in MessageProcessor).
                    PulseEvent::Left { address } => {
                        self.send_to_processor(address, MessageType::PeerLeft);
                    }
                    PulseEvent::ProfileVersion { address, version } => {
                        self.bridge_profile_version(address, version);
                    }
                    PulseEvent::EmoteStart { address, urn, tick } => {
                        self.send_rfc4_to_processor(
                            address,
                            rfc4::packet::Message::PlayerEmote(rfc4::PlayerEmote {
                                incremental_id: tick,
                                urn,
                                timestamp: 0.0,
                                is_stopping: Some(false),
                            }),
                        );
                    }
                    PulseEvent::EmoteStop { address } => {
                        self.send_rfc4_to_processor(
                            address,
                            rfc4::packet::Message::PlayerEmote(rfc4::PlayerEmote {
                                incremental_id: 0,
                                urn: String::new(),
                                timestamp: 0.0,
                                is_stopping: Some(true),
                            }),
                        );
                    }
                    // A sequence gap — ask the server to replay full state, reliably.
                    PulseEvent::Resync(request) => {
                        self.send_client_message(
                            pulse::client_message::Message::Resync(request),
                            PulseReliability::Reliable,
                        );
                    }
                }
            }
        }
    }

    fn bridge_profile_version(&self, address: H160, version: i32) {
        self.send_rfc4_to_processor(
            address,
            rfc4::packet::Message::ProfileVersion(rfc4::AnnounceProfileVersion {
                // Pulse carries i32; rfc4 is u32 — clamp negatives (bevy parity).
                profile_version: version.max(0) as u32,
            }),
        );
    }

    fn on_handshake_response(
        &mut self,
        now: Instant,
        success: bool,
        error: Option<String>,
        events: &mut Vec<PulseRoomEvent>,
    ) {
        // Ignore a stray response we're not waiting on (e.g. a duplicate after establishing).
        if !matches!(self.state, Connection::AwaitingResponse { .. }) {
            return;
        }
        if !success {
            tracing::warn!(
                "pulse: handshake rejected, retrying: {}",
                error.unwrap_or_default()
            );
            self.state = Connection::Idle {
                retry_after: now + RETRY_COOLDOWN,
            };
            return;
        }
        tracing::info!("pulse: handshake accepted");
        self.state = Connection::Established;
        self.established_this_attempt = true;
        events.push(PulseRoomEvent::Established);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::wallet::{SimpleAuthChain, Wallet};
    use crate::comms::pulse::transport::{pulse_channels, PulseDriverChannels};

    fn processor_channel() -> (
        mpsc::Sender<IncomingMessage>,
        mpsc::Receiver<IncomingMessage>,
    ) {
        mpsc::channel(64)
    }

    /// A room wired to in-memory channels instead of a live driver, parked in `Connecting`.
    /// Returns the driver-side channel ends so tests can play the server/transport role.
    fn test_room(processor: mpsc::Sender<IncomingMessage>) -> (PulseRoom, PulseDriverChannels) {
        let mut room = PulseRoom::new(
            PulseTransportConfig {
                host: "test.invalid".into(),
                port: 7777,
            },
            processor,
        );
        let (link, channels) = pulse_channels(64);
        room.link = Some(link);
        room.state = Connection::Connecting;
        (room, channels)
    }

    async fn test_identity() -> EphemeralAuthChain {
        let signer = Wallet::new_local_wallet();
        let ephemeral = ethers_signers::LocalWallet::new(&mut rand::thread_rng());
        let ephemeral_keys = ephemeral.signer().to_bytes().to_vec();
        let expiration = std::time::SystemTime::now() + Duration::from_secs(600);
        // The chain-link message content doesn't matter for these tests (nothing verifies it);
        // only the ephemeral wallet signing the connect payload does.
        let message = "test ephemeral".to_owned();
        let signature = signer.sign_message(&message).await.unwrap();
        let chain = SimpleAuthChain::new_ephemeral_identity_auth_chain(
            signer.address(),
            message,
            signature,
        );
        EphemeralAuthChain::new(signer.address(), ephemeral_keys, chain, expiration)
    }

    fn server_message_bytes(message: pulse::server_message::Message) -> Vec<u8> {
        pulse::ServerMessage {
            message: Some(message),
        }
        .encode_to_vec()
    }

    fn handshake_response(success: bool) -> Vec<u8> {
        server_message_bytes(pulse::server_message::Message::Handshake(
            pulse::HandshakeResponse {
                success,
                error: (!success).then(|| "rejected".to_owned()),
            },
        ))
    }

    fn player_joined(subject_id: u32, wallet: &str) -> Vec<u8> {
        server_message_bytes(pulse::server_message::Message::PlayerJoined(
            pulse::PlayerJoined {
                user_id: wallet.to_owned(),
                profile_version: 3,
                state: Some(pulse::PlayerStateFull {
                    subject_id,
                    sequence: 1,
                    server_tick: 100,
                    state: Some(pulse::PlayerState {
                        parcel_index: 54858,
                        ..Default::default()
                    }),
                }),
                realm: "main".to_owned(),
            },
        ))
    }

    /// Full happy path: Connected → Idle → Signing → handshake frame on the wire →
    /// HandshakeResponse{success} → Established event.
    #[tokio::test]
    async fn connect_sign_handshake_established() {
        let (sender, _keep) = processor_channel();
        let (mut room, mut driver) = test_room(sender);
        let identity = test_identity().await;

        driver.status.try_send(PulseStatus::Connected).unwrap();
        let now = Instant::now();
        assert!(room.poll(now, || Some((identity.clone(), 7))).is_empty());
        assert!(matches!(room.state, Connection::Signing));

        // The signing task runs on a fallback thread in tests; keep polling until the room
        // observes the signed bytes and puts the handshake frame on the wire.
        let deadline = Instant::now() + Duration::from_secs(5);
        while !matches!(room.state, Connection::AwaitingResponse { .. }) {
            assert!(Instant::now() < deadline, "sign timed out");
            tokio::time::sleep(Duration::from_millis(10)).await;
            room.poll(now, || None);
        }
        let frame = driver.outbound.try_recv().expect("no handshake frame");
        assert_eq!(frame.reliability, PulseReliability::Reliable);

        // Decode what we would send and sanity-check the auth chain shape.
        let sent = pulse::ClientMessage::decode(frame.bytes.as_slice()).unwrap();
        let Some(pulse::client_message::Message::Handshake(handshake)) = sent.message else {
            panic!("expected handshake message");
        };
        assert_eq!(handshake.profile_version, 7);
        let dict: serde_json::Map<String, serde_json::Value> =
            serde_json::from_slice(&handshake.auth_chain).unwrap();
        assert!(dict.contains_key("x-identity-timestamp"));
        assert_eq!(dict["x-identity-metadata"], "{}");
        assert!(dict.contains_key("x-identity-auth-chain-0"));
        // The signed-entity link carries the verbatim connect payload.
        let last_link = dict
            .iter()
            .filter(|(k, _)| k.starts_with("x-identity-auth-chain-"))
            .map(|(_, v)| v.as_str().unwrap())
            .find(|v| v.contains("connect:/:"))
            .expect("no connect signed entity in chain");
        assert!(last_link.contains("ECDSA_SIGNED_ENTITY"));

        driver.inbound.try_send(handshake_response(true)).unwrap();
        let events = room.poll(now, || None);
        assert_eq!(events, vec![PulseRoomEvent::Established]);
        assert!(room.is_established());
    }

    #[tokio::test]
    async fn handshake_rejection_folds_back_to_idle_with_cooldown() {
        let (sender, _keep) = processor_channel();
        let (mut room, driver) = test_room(sender);
        let now = Instant::now();
        room.state = Connection::AwaitingResponse {
            timeout_at: now + HANDSHAKE_RESPONSE_TIMEOUT,
        };

        driver.inbound.try_send(handshake_response(false)).unwrap();
        assert!(room.poll(now, || None).is_empty());
        match room.state {
            Connection::Idle { retry_after } => assert!(retry_after > now),
            _ => panic!("expected Idle"),
        }
    }

    #[tokio::test]
    async fn handshake_timeout_retries() {
        let (sender, _keep) = processor_channel();
        let (mut room, _driver) = test_room(sender);
        let now = Instant::now();
        room.state = Connection::AwaitingResponse {
            timeout_at: now - Duration::from_millis(1),
        };
        room.poll(now, || None);
        assert!(matches!(room.state, Connection::Idle { .. }));
    }

    #[tokio::test]
    async fn retryable_disconnect_before_established_counts_a_strike() {
        let (sender, _keep) = processor_channel();
        let (mut room, driver) = test_room(sender);
        let now = Instant::now();

        driver
            .status
            .try_send(PulseStatus::Disconnected(
                super::super::transport::PulseDisconnect::ServerFull,
            ))
            .unwrap();
        let events = room.poll(now, || None);
        assert_eq!(events, vec![PulseRoomEvent::AttemptFailed]);
        assert!(matches!(room.state, Connection::Down { .. }));
    }

    #[tokio::test]
    async fn terminal_disconnect_parks_dead() {
        let (sender, _keep) = processor_channel();
        let (mut room, driver) = test_room(sender);
        let now = Instant::now();

        driver
            .status
            .try_send(PulseStatus::Disconnected(
                super::super::transport::PulseDisconnect::AuthFailed,
            ))
            .unwrap();
        let events = room.poll(now, || None);
        assert_eq!(
            events,
            vec![PulseRoomEvent::AttemptFailed, PulseRoomEvent::Dead]
        );
        assert!(matches!(room.state, Connection::Dead));
        // Dead is sticky: further polls do nothing.
        assert!(room.poll(now, || None).is_empty());
    }

    #[tokio::test]
    async fn pipe_close_is_authoritative_teardown() {
        let (sender, _keep) = processor_channel();
        let (mut room, driver) = test_room(sender);
        room.state = Connection::Established;
        room.established_this_attempt = true;
        let now = Instant::now();

        drop(driver); // driver vanished with no word
        let events = room.poll(now, || None);
        // Established before the drop → no strike; retryable → Down.
        assert!(events.is_empty());
        assert!(matches!(room.state, Connection::Down { .. }));
    }

    /// Inbound peers bridged with room_id "pulse"; teardown floods synthetic PeerLefts.
    #[tokio::test]
    async fn joined_bridges_peer_and_teardown_floods_left() {
        const WALLET: &str = "0x00000000000000000000000000000000000000aa";
        let (sender, mut processor_rx) = processor_channel();
        let (mut room, driver) = test_room(sender);
        room.state = Connection::Established;
        room.established_this_attempt = true;
        let now = Instant::now();

        driver.inbound.try_send(player_joined(1, WALLET)).unwrap();
        room.poll(now, || None);

        // PeerJoined + ProfileVersion + Movement, all room_id = "pulse".
        let mut kinds = Vec::new();
        while let Ok(msg) = processor_rx.try_recv() {
            assert_eq!(msg.room_id, PULSE_ROOM_ID);
            kinds.push(match msg.message {
                MessageType::PeerJoined => "joined",
                MessageType::PeerLeft => "left",
                MessageType::Rfc4(r) => match r.message {
                    rfc4::packet::Message::Movement(_) => "movement",
                    rfc4::packet::Message::ProfileVersion(_) => "profile",
                    _ => "other",
                },
                _ => "other",
            });
        }
        assert_eq!(kinds, vec!["joined", "profile", "movement"]);

        // Teardown → synthetic PeerLeft for the known wallet.
        drop(driver);
        room.poll(now, || None);
        let msg = processor_rx.try_recv().expect("no teardown PeerLeft");
        assert!(matches!(msg.message, MessageType::PeerLeft));
        assert_eq!(msg.room_id, PULSE_ROOM_ID);
    }
}
