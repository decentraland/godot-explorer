extends SceneTree

# Unit test for the avatar pool policy (feat/avatar-lod-pool).
#
# The pool decides how many of the closest in-frustum avatars keep their
# animated 3D mesh before the rest fall back to billboard impostors. This
# test pins:
#   1. flat 40-avatar pool (no graphics-profile dependency)
#   2. full_rate_for/overflow_start helpers
#   3. assign_caps partition boundaries: everyone FULL under the cap, MID
#      throttled within the pool, FAR beyond it, overflow silhouettes only
#      past pool + real impostor layers
#
# Run headless:
#   .bin/godot/godot4_bin --headless --path godot \
#     --script res://src/test/avatar/test_avatar_pool_policy.gd

const P := preload("res://src/decentraland_components/avatar/impostor/avatar_pool_policy.gd")

const FULL := 0
const MID := 1
const FAR := 3

var _failures: Array[String] = []


func _initialize() -> void:
	_test_local_constants_match_policy()
	_test_pool_constant_and_helpers()
	_test_assign_caps_under_pool()
	_test_assign_caps_over_pool()
	_test_assign_caps_throttle_split()
	_test_assign_caps_overflow_boundary()
	_test_assign_caps_edge_cases()
	_finish()


func _test_local_constants_match_policy() -> void:
	_expect_eq("LOD_FULL", P.LOD_FULL, FULL)
	_expect_eq("LOD_MID", P.LOD_MID, MID)
	_expect_eq("LOD_FAR", P.LOD_FAR, FAR)


func _test_pool_constant_and_helpers() -> void:
	# Flat 40-avatar pool, no graphics-profile dependency (deliberate: profile
	# affects render quality only; the mesh budget is the same on all tiers).
	_expect_eq("POOL_SIZE is 40", P.POOL_SIZE, 40)
	# full_rate_for clamps to FULL_RATE_CAP when the pool is smaller.
	_expect_eq("full rate at 40", P.full_rate_for(40), P.FULL_RATE_CAP)
	_expect_eq("full rate clamps", P.full_rate_for(4), 4)
	_expect_eq("overflow start", P.overflow_start(40, 8), 48)


func _test_assign_caps_under_pool() -> void:
	# Fewer avatars than the pool: everyone keeps the mesh, nobody overflows.
	var caps: Array = P.assign_caps(5, 8, 4)
	_expect_eq("size", caps.size(), 5)
	for i in range(caps.size()):
		_expect_cap("rank %d" % i, caps[i], FULL, false)


func _test_assign_caps_over_pool() -> void:
	# Exactly at the pool boundary the mesh ends; the very next rank is FAR.
	var caps: Array = P.assign_caps(10, 8, 4)
	_expect_eq("size", caps.size(), 10)
	for i in range(8):
		_expect_cap("in-pool rank %d" % i, caps[i], FULL, false)
	_expect_cap("first beyond pool", caps[8], FAR, false)
	_expect_cap("second beyond pool", caps[9], FAR, false)


func _test_assign_caps_throttle_split() -> void:
	# Old split restored: FULL_RATE_CAP closest run full-rate, the rest of the
	# pool keeps the mesh throttled (MID), billboards only past the pool.
	var caps: Array = P.assign_caps(41, 40, 4)
	for i in range(P.FULL_RATE_CAP):
		_expect_cap("full-rate rank %d" % i, caps[i], FULL, false)
	for i in range(P.FULL_RATE_CAP, 40):
		_expect_cap("throttled rank %d" % i, caps[i], MID, false)
	_expect_cap("first billboard", caps[40], FAR, false)
	# pool_size < FULL_RATE_CAP: full-rate tier clamps to the pool.
	caps = P.assign_caps(6, 4, 4)
	for i in range(4):
		_expect_cap("tiny pool rank %d" % i, caps[i], FULL, false)
	_expect_cap("tiny pool overflow rank", caps[4], FAR, false)


func _test_assign_caps_overflow_boundary() -> void:
	# Overflow (borrowed silhouette texture) starts at pool + real layers.
	var caps: Array = P.assign_caps(14, 8, 4)
	_expect_cap("last real impostor", caps[11], FAR, false)
	_expect_cap("first overflow", caps[12], FAR, true)
	_expect_cap("second overflow", caps[13], FAR, true)


func _test_assign_caps_edge_cases() -> void:
	_expect_eq("empty", P.assign_caps(0, 8, 4).size(), 0)
	# pool_size=0: everything is a billboard from rank 0 (defensive — real
	# profiles always grant >= 1).
	var caps: Array = P.assign_caps(2, 0, 1)
	_expect_cap("pool=0 rank 0", caps[0], FAR, false)
	_expect_cap("pool=0 rank 1 overflows", caps[1], FAR, true)


func _expect_cap(ctx: String, cap: Array, lod: int, overflow: bool) -> void:
	if cap.size() != 2 or cap[0] != lod or cap[1] != overflow:
		_fail("%s: got %s, expected [%d, %s]" % [ctx, str(cap), lod, overflow])


func _expect_eq(ctx: String, actual: int, expected: int) -> void:
	if actual != expected:
		_fail("%s: got %d, expected %d" % [ctx, actual, expected])


func _fail(msg: String) -> void:
	_failures.append(msg)


func _finish() -> void:
	if _failures.is_empty():
		print("[test_avatar_pool_policy] PASS")
		quit(0)
		return
	for f in _failures:
		printerr(f)
	printerr("[test_avatar_pool_policy] FAIL: %d case(s)" % _failures.size())
	quit(1)
