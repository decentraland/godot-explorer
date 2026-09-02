class_name AvatarPoolPolicy
extends RefCounted

# Avatar pool policy: how many in-frustum avatars stay fully-rendered 3D meshes
# before the rest fall back to billboard impostors. Pure statics only (no
# Global / scene-tree deps) so the policy is unit-testable headless
# (see test/avatar/test_avatar_pool_policy.gd).

# Mirror of Avatar.LODState — local so this file doesn't pull in avatar.gd.
const LOD_FULL := 0
const LOD_MID := 1
const LOD_FAR := 3

# Rank-based animation throttle, same split as the old MAX_FULL_AVATARS=8:
# only the FULL_RATE_CAP closest avatars run their AnimationTree every frame;
# the rest of the pool keeps the mesh but advances the skeleton throttled
# (~20fps, see AvatarImpostorConfig.MID_ANIM_ADVANCE_EVERY_N_FRAMES).
const FULL_RATE_CAP := 8

# Pool size: how many in-frustum avatars keep their 3D mesh before the rest
# fall back to billboard impostors. Constant 40 = the old fixed budget
# (MAX_FULL_AVATARS 8 + MAX_THROTTLED_AVATARS 32): FULL_RATE_CAP full-rate +
# 32 throttled. Deliberately NOT tied to graphics profile — profile only
# affects render quality, avatar mesh budget stays the same on all tiers.
const POOL_SIZE := 40


# How many of the pool's avatars run their AnimationTree every frame.
static func full_rate_for(pool_size: int) -> int:
	return mini(FULL_RATE_CAP, pool_size)


# First rank that must borrow another slot's texture (real impostor layers are
# a finite VRAM resource). Avatars at/after this rank render fully tinted —
# distant silhouette, no capture cost, no LRU thrash.
static func overflow_start(pool_size: int, max_real_impostors: int) -> int:
	return pool_size + max_real_impostors


# Partition a distance-sorted in-frustum avatar list into pool decisions.
# Returns one [lod_cap, overflow] entry per rank:
#   rank < FULL_RATE_CAP                  -> FULL (mesh, full-rate anim)
#   rank < pool_size                      -> MID (mesh, throttled anim)
#   otherwise                             -> FAR (billboard impostor)
#   rank >= pool_size + max_real_impostors -> overflow = true (borrowed
#     silhouette texture, no capture — the real layers are a finite resource)
static func assign_caps(entry_count: int, pool_size: int, max_real_impostors: int) -> Array:
	var caps: Array = []
	caps.resize(entry_count)
	var full_rate: int = full_rate_for(pool_size)
	var overflow: int = overflow_start(pool_size, max_real_impostors)
	for i in range(entry_count):
		if i < full_rate:
			caps[i] = [LOD_FULL, false]
		elif i < pool_size:
			caps[i] = [LOD_MID, false]
		else:
			caps[i] = [LOD_FAR, i >= overflow]
	return caps
