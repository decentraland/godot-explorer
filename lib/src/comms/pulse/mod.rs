//! Pulse transport — the ENet/UDP avatar-state relay that coexists with LiveKit.
//! See `plan_pulse/` at the repo root for the architecture and rollout docs.
//!
//! Module map (port of bevy-explorer `crates/comms/src/pulse/` @ 3f65c164, ENet only):
//! - [`transport`] — the byte-boundary seam (frames, status, disconnect codes, channel pair)
//! - `native` — the `pulse-enet` driver thread (rusty_enet, ENet-CSharp modified protocol)
//! - [`decoder`] — quantized state → `rfc4::Movement` reconstruction + parcel grid
//! - [`pulse_room`] — connection state machine, handshake, `MessageProcessor` bridging

pub mod decoder;
mod native;
pub mod pulse_room;
pub mod transport;

/// The `IncomingMessage::room_id` for everything bridged from Pulse. A load-bearing room id
/// (like `"scene-"`/`"livekit-"` prefixes): `MessageProcessor` keys the per-peer transport
/// preference and the inactivity-sweep exemption on it.
pub const PULSE_ROOM_ID: &str = "pulse";
