//! Pulse transport — the ENet/UDP avatar-state relay that coexists with LiveKit.
//!
//! Module map (port of bevy-explorer `crates/comms/src/pulse/` @ 3f65c164, ENet only):
//! - [`transport`] — the byte-boundary seam (frames, status, disconnect codes, channel pair)
//! - `native` — the `pulse-enet` driver thread (rusty_enet, ENet-CSharp modified protocol)
//! - [`decoder`] — quantized state → `rfc4::Movement` reconstruction + parcel grid
//! - [`pulse_room`] — connection state machine, handshake, `MessageProcessor` bridging
//! - [`lsd_realm`] — Local Scene Development preview → Pulse realm key (cross-repo contract)

pub mod decoder;
pub mod lsd_realm;
mod native;
pub mod pulse_room;
pub mod transport;

/// See `comms::consts::PULSE_ROOM_ID` (lives there, unconditional, so MessageProcessor's
/// transport-preference gate compiles without the `use_pulse` feature).
pub use crate::comms::consts::PULSE_ROOM_ID;
