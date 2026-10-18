//! LOD tuning for distant avatars: distance bands and the pool that decides who keeps a mesh.

/// Past this distance an avatar is a billboard even with a free pool slot.
pub const DISTANCE_FAR: f32 = 80.0;
pub const MID_RANGE_NEAR: f32 = 15.0;
pub const TINT_FULL_DISTANCE: f32 = 110.0;
pub const DISTANCE_CHECK_PERIOD_FRAMES: u64 = 6;
/// The throttled (MID) tier advances its AnimationTree every Nth frame.
pub const MID_ANIM_ADVANCE_EVERY_N_FRAMES: u32 = 3;

/// The pool is a flat 40 meshes on every graphics profile: the profile changes quality only.
pub const POOL_SIZE: usize = 40;
/// Closest avatars that animate every frame (at mobile frame rates each full-rate tree is
/// costly); the rest of the pool is throttled.
pub const FULL_RATE_CAP: usize = 4;

/// Level of detail of an avatar; the numbers are the ones scenes and the shell saw before.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum LodState {
    #[default]
    Full = 0,
    Mid = 1,
    Far = 3,
}

pub fn full_rate_for(pool_size: usize) -> usize {
    FULL_RATE_CAP.min(pool_size)
}

/// First rank that borrows another slot's texture: real impostor layers are finite VRAM.
pub fn overflow_start(pool_size: usize, max_real_impostors: usize) -> usize {
    pool_size + max_real_impostors
}

/// The cap and the overflow flag of one rank in the distance-sorted in-frustum list.
pub fn cap_for_rank(rank: usize, pool_size: usize, max_real_impostors: usize) -> (LodState, bool) {
    if rank < full_rate_for(pool_size) {
        (LodState::Full, false)
    } else if rank < pool_size {
        (LodState::Mid, false)
    } else {
        (
            LodState::Far,
            rank >= overflow_start(pool_size, max_real_impostors),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ranks(count: usize, pool: usize, real: usize) -> Vec<(LodState, bool)> {
        (0..count).map(|r| cap_for_rank(r, pool, real)).collect()
    }

    #[test]
    fn pool_constants_and_helpers() {
        assert_eq!(POOL_SIZE, 40);
        assert_eq!(full_rate_for(40), FULL_RATE_CAP);
        assert_eq!(full_rate_for(4), 4);
        assert_eq!(overflow_start(40, 8), 48);
    }

    #[test]
    fn everyone_keeps_the_mesh_under_the_pool() {
        for (rank, cap) in ranks(5, 8, 4).into_iter().enumerate() {
            let lod = if rank < FULL_RATE_CAP {
                LodState::Full
            } else {
                LodState::Mid
            };
            assert_eq!(cap, (lod, false));
        }
    }

    #[test]
    fn mesh_ends_at_the_pool_boundary() {
        let caps = ranks(10, 8, 4);
        assert!(caps[..FULL_RATE_CAP]
            .iter()
            .all(|c| *c == (LodState::Full, false)));
        assert!(caps[FULL_RATE_CAP..8]
            .iter()
            .all(|c| *c == (LodState::Mid, false)));
        assert_eq!(caps[8], (LodState::Far, false));
        assert_eq!(caps[9], (LodState::Far, false));
    }

    #[test]
    fn throttled_tier_sits_between_full_rate_and_billboards() {
        let caps = ranks(41, 40, 4);
        assert!(caps[..FULL_RATE_CAP]
            .iter()
            .all(|c| *c == (LodState::Full, false)));
        assert!(caps[FULL_RATE_CAP..40]
            .iter()
            .all(|c| *c == (LodState::Mid, false)));
        assert_eq!(caps[40], (LodState::Far, false));

        let tiny = ranks(6, 4, 4);
        assert!(tiny[..4].iter().all(|c| *c == (LodState::Full, false)));
        assert_eq!(tiny[4], (LodState::Far, false));
    }

    #[test]
    fn overflow_starts_after_the_real_layers() {
        let caps = ranks(14, 8, 4);
        assert_eq!(caps[11], (LodState::Far, false));
        assert_eq!(caps[12], (LodState::Far, true));
        assert_eq!(caps[13], (LodState::Far, true));
    }

    #[test]
    fn empty_pool_is_all_billboards() {
        assert_eq!(cap_for_rank(0, 0, 1), (LodState::Far, false));
        assert_eq!(cap_for_rank(1, 0, 1), (LodState::Far, true));
        assert!(ranks(0, 8, 4).is_empty());
    }

    #[test]
    fn lod_numbers_are_stable() {
        assert_eq!(LodState::Full as i32, 0);
        assert_eq!(LodState::Mid as i32, 1);
        assert_eq!(LodState::Far as i32, 3);
    }
}
