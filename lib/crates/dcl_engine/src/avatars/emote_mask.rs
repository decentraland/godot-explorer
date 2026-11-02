//! Conversions between the three avatar-emote-mask representations.
//!
//! - **Internal** (GDScript/signal boundary): `i64` where `-1` = full body and
//!   `0` = `AvatarMask::AM_UPPER_BODY`. Godot signals can't carry `Option`, and
//!   deno fast ops can't either, so the sentinel travels everywhere in-process.
//! - **Component** (`PBAvatarEmoteCommand.mask`, SDK enum `AvatarMask`): optional
//!   field where *absent* means full body and `0` means `AM_UPPER_BODY`.
//! - **Wire** (rfc4 `PlayerEmote.mask` / Pulse `EmoteStart(ed).mask`, comms enum
//!   `AvatarEmoteMask`): `0`/absent = `AEM_FULL_BODY`, `1` = `AEM_UPPER_BODY`.
//!   Note the off-by-one vs the component enum — Unity encodes exactly this way
//!   (`LiveKitEmotesMessageBus` casts `AvatarEmoteMask` to uint).

/// SDK `AvatarMask::AM_UPPER_BODY` in the internal representation.
pub const INTERNAL_UPPER_BODY: i64 = 0;
/// Comms `AvatarEmoteMask::AEM_UPPER_BODY` raw wire value.
const WIRE_UPPER_BODY: u32 = 1;

/// Internal -> `PBAvatarEmoteCommand.mask` (`AvatarMask`, absent = full body).
pub fn component_mask_from_internal(mask: i64) -> Option<i32> {
    (mask >= 0).then_some(mask as i32)
}

/// `PBAvatarEmoteCommand.mask` -> internal.
pub fn internal_from_component_mask(mask: Option<i32>) -> i64 {
    mask.map_or(-1, |m| m as i64)
}

/// Internal -> rfc4/Pulse wire value (`AvatarEmoteMask`, absent = full body).
pub fn wire_mask_from_internal(mask: i64) -> Option<u32> {
    (mask == INTERNAL_UPPER_BODY).then_some(WIRE_UPPER_BODY)
}

/// rfc4/Pulse wire value -> internal. Unknown values fall back to full body.
pub fn internal_from_wire_mask(mask: Option<u32>) -> i64 {
    match mask {
        Some(WIRE_UPPER_BODY) => INTERNAL_UPPER_BODY,
        _ => -1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn component_round_trip() {
        assert_eq!(component_mask_from_internal(-1), None);
        assert_eq!(component_mask_from_internal(0), Some(0));
        assert_eq!(internal_from_component_mask(None), -1);
        assert_eq!(internal_from_component_mask(Some(0)), 0);
    }

    #[test]
    fn wire_round_trip_has_unity_offset() {
        assert_eq!(wire_mask_from_internal(-1), None);
        assert_eq!(wire_mask_from_internal(0), Some(1));
        assert_eq!(internal_from_wire_mask(None), -1);
        assert_eq!(internal_from_wire_mask(Some(0)), -1);
        assert_eq!(internal_from_wire_mask(Some(1)), 0);
        assert_eq!(internal_from_wire_mask(Some(7)), -1);
    }
}
