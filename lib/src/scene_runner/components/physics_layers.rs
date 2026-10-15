//! Godot layer assignment for the SDK's `ColliderLayer` masks.
//!
//! SDK collision bits map 1:1 onto Godot layers (`project.godot`
//! `[layer_names]`), which already reproduces Unity's character collision
//! matrix: the capsule blocks on every mask carrying `CL_PHYSICS` — the
//! invisible-blocker and trigger-area layers included — and on nothing else.
//! The avatar-only mask is the one case Unity gives a layer of its own:
//! `PhysicsLayers.TryGetUnityLayerFromSDKLayer` routes it to `SDKAvatarHit`, so
//! the capsule walks through it while avatar-targeting queries still find it.

/// `CL_PLAYER` | `CL_MAIN_PLAYER`: the bits that mark a collider as an avatar.
pub const AVATAR_BITS: u32 = 4 | 8;

/// Layer 17, the `SDKAvatarHit` equivalent.
pub const SDK_AVATAR_HIT_LAYER: u32 = 1 << 16;

/// Godot collision layer for a collider a scene authored.
pub fn collider_layer(mask: u32) -> u32 {
    if mask & AVATAR_BITS != 0 && mask & !AVATAR_BITS == 0 {
        SDK_AVATAR_HIT_LAYER
    } else {
        mask
    }
}

/// Collision mask for a query a scene authored (raycast, trigger area): the
/// avatar bits also match colliders routed to the `SDKAvatarHit` layer.
pub fn query_mask(mask: u32) -> u32 {
    if mask & AVATAR_BITS != 0 {
        mask | SDK_AVATAR_HIT_LAYER
    } else {
        mask
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CL_PHYSICS: u32 = 2;
    const CL_POINTER: u32 = 1;
    const CL_CUSTOM1: u32 = 256;

    #[test]
    fn avatar_only_masks_move_to_the_avatar_hit_layer() {
        assert_eq!(collider_layer(4), SDK_AVATAR_HIT_LAYER);
        assert_eq!(collider_layer(8), SDK_AVATAR_HIT_LAYER);
        assert_eq!(collider_layer(4 | 8), SDK_AVATAR_HIT_LAYER);
    }

    #[test]
    fn every_other_mask_keeps_its_sdk_bits() {
        assert_eq!(collider_layer(0), 0);
        assert_eq!(collider_layer(CL_POINTER), CL_POINTER);
        assert_eq!(collider_layer(CL_PHYSICS), CL_PHYSICS);
        assert_eq!(
            collider_layer(CL_POINTER | CL_PHYSICS),
            CL_POINTER | CL_PHYSICS
        );
        assert_eq!(collider_layer(CL_PHYSICS | 4), CL_PHYSICS | 4);
        assert_eq!(collider_layer(4 | CL_CUSTOM1), 4 | CL_CUSTOM1);
    }

    #[test]
    fn avatar_targeting_queries_also_match_the_avatar_hit_layer() {
        assert_eq!(query_mask(4), 4 | SDK_AVATAR_HIT_LAYER);
        assert_eq!(query_mask(4 | 8), 4 | 8 | SDK_AVATAR_HIT_LAYER);
        assert_eq!(
            query_mask(4 | CL_PHYSICS),
            4 | CL_PHYSICS | SDK_AVATAR_HIT_LAYER
        );
    }

    #[test]
    fn queries_without_avatar_bits_are_untouched() {
        assert_eq!(query_mask(0), 0);
        assert_eq!(query_mask(CL_POINTER), CL_POINTER);
        assert_eq!(query_mask(CL_POINTER | CL_PHYSICS), CL_POINTER | CL_PHYSICS);
        assert_eq!(query_mask(CL_CUSTOM1), CL_CUSTOM1);
    }
}
