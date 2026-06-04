extends SceneTree

# Regression test for the avatar-impostor "freezes instead of throttling" bug.
#
# When an avatar transitions from FULL (full-rate animation) to MID/CROSSFADE
# (throttled), the AnimationTree is driven in MANUAL callback mode and advanced
# every N frames. The off-screen freeze used to switch _anim_throttle_active off
# WITHOUT resetting the callback mode, leaving the tree in MANUAL with nothing
# advancing it; the per-frame re-activation then forced active=true, so an avatar
# the on-screen check flagged as off-screen (screen edge / camera-pan lag) but
# that Godot still drew appeared frozen on a stale pose instead of throttling.
#
# The fix routes every throttle change through AvatarLODHelpers.set_animation_throttle
# (callback mode and flag move together) and drives the freeze off the real
# VisibleOnScreenNotifier3D state. This test pins the invariant that makes the
# bug impossible: the tree is never left "active + MANUAL + throttle-off".
#
# Run headless:
#   .bin/godot/godot4_bin --headless --path godot \
#     --script res://src/test/avatar/test_avatar_anim_throttle.gd

# Preload only the (Global-free, RefCounted) helper so this test compiles
# standalone under --script, without dragging in avatar.gd / the Global autoload.
const H := preload("res://src/decentraland_components/avatar/impostor/avatar_lod_helpers.gd")

# LODState values (mirror of Avatar.LODState / AvatarLODHelpers.LOD_*).
const FULL := 0
const MID := 1
const CROSSFADE := 2
const FAR := 3

var _failures: Array[String] = []


# Duck-typed stand-in for Avatar: exposes exactly the fields AvatarLODHelpers
# reads/writes, plus a real (minimal) AnimationTree so advance()/active/callback
# behave like production without needing the full Avatar scene.
class FakeAvatar:
	extends Node

	var animation_tree: AnimationTree
	var animation_player: AnimationPlayer = null
	var is_local_player: bool = false
	var _on_screen: bool = true
	var _lod_state: int = 0
	var _anim_throttle_active: bool = false
	var _anim_throttle_acc: float = 0.0
	var _anim_throttle_counter: int = 0
	var _anim_frozen_off_screen: bool = false
	var _anim_freeze_start_ms: int = 0


func _initialize() -> void:
	_test_local_constants_match_helper()
	_test_resolve_policy()
	_test_state_invariant_across_transitions()
	_test_detects_frozen_while_drawn()
	_finish()


func _make_fake_avatar() -> FakeAvatar:
	var fa := FakeAvatar.new()
	var tree := AnimationTree.new()
	var lib := AnimationLibrary.new()
	var anim := Animation.new()
	anim.length = 1.0
	anim.loop_mode = Animation.LOOP_LINEAR
	lib.add_animation("loop", anim)
	tree.add_animation_library("", lib)
	var anim_root := AnimationNodeAnimation.new()
	anim_root.animation = "loop"
	tree.tree_root = anim_root
	fa.add_child(tree)
	fa.animation_tree = tree
	root.add_child(fa)
	return fa


func _is_manual(fa: FakeAvatar) -> bool:
	return (
		fa.animation_tree.callback_mode_process
		== AnimationMixer.ANIMATION_CALLBACK_MODE_PROCESS_MANUAL
	)


# The core invariant: callback mode and the throttle flag must always agree, so
# the tree can never sit "active + MANUAL + throttle-off" (frozen while drawn).
func _check_invariant(fa: FakeAvatar, ctx: String) -> void:
	var manual := _is_manual(fa)
	if manual != fa._anim_throttle_active:
		_fail(
			(
				"%s: callback MANUAL=%s but _anim_throttle_active=%s (inconsistent)"
				% [ctx, manual, fa._anim_throttle_active]
			)
		)
	if fa.animation_tree.active and manual and not fa._anim_throttle_active:
		_fail("%s: FROZEN-WHILE-DRAWN (active + MANUAL + throttle off)" % ctx)
	# On-screen, animating LOD ⇒ the tree must actually be running.
	if fa._on_screen and fa._lod_state != FAR and not fa.is_local_player:
		if not fa.animation_tree.active:
			_fail("%s: on-screen LOD=%d but tree inactive (frozen)" % [ctx, fa._lod_state])
	# Off-screen (non-FAR) ⇒ frozen, so the CPU saving actually holds.
	if not fa._on_screen and not fa.is_local_player:
		if fa.animation_tree.active:
			_fail("%s: off-screen but tree still active (freeze didn't take)" % ctx)


# Replicates Avatar._on_lod_state_changed's animation portion.
func _set_lod(fa: FakeAvatar, lod: int) -> void:
	fa._lod_state = lod
	var drive: Dictionary = H.resolve_anim_drive(fa._on_screen, lod)
	H.set_animation_active(fa, drive.active)
	H.set_animation_throttle(fa, drive.throttle)


# Replicates Avatar's screen notifier handler.
func _set_on_screen(fa: FakeAvatar, on_screen: bool) -> void:
	fa._on_screen = on_screen
	H.apply_screen_freeze(fa)


# Replicates one Avatar._process tick: reconcile freeze + guarded re-activation.
func _tick(fa: FakeAvatar) -> void:
	H.apply_screen_freeze(fa)
	H.ensure_anim_active(fa)


func _test_local_constants_match_helper() -> void:
	_expect_eq("LOD_FULL", FULL, H.LOD_FULL)
	_expect_eq("LOD_MID", MID, H.LOD_MID)
	_expect_eq("LOD_CROSSFADE", CROSSFADE, H.LOD_CROSSFADE)
	_expect_eq("LOD_FAR", FAR, H.LOD_FAR)


func _test_resolve_policy() -> void:
	# off-screen ⇒ frozen regardless of LOD
	for lod in [FULL, MID, CROSSFADE, FAR]:
		var d: Dictionary = H.resolve_anim_drive(false, lod)
		_expect_drive("off-screen LOD=%d" % lod, d, false, false)
	_expect_drive("on FULL", H.resolve_anim_drive(true, FULL), true, false)
	_expect_drive("on MID", H.resolve_anim_drive(true, MID), true, true)
	_expect_drive("on CROSSFADE", H.resolve_anim_drive(true, CROSSFADE), true, true)
	_expect_drive("on FAR", H.resolve_anim_drive(true, FAR), false, false)


# Walk the bug-prone transition sequences and assert the invariant after every
# step. The headline case: FULL→MID (throttle on) → off-screen (freeze) → a
# couple of _process ticks (re-activation) → back on-screen (restore).
func _test_state_invariant_across_transitions() -> void:
	var fa := _make_fake_avatar()

	_set_on_screen(fa, true)
	_set_lod(fa, FULL)
	_check_invariant(fa, "on-screen FULL")

	_set_lod(fa, MID)
	_check_invariant(fa, "FULL→MID throttled")

	_set_on_screen(fa, false)
	_check_invariant(fa, "MID then off-screen (freeze)")

	# The exact bug trigger: re-activation ticks while frozen must not unfreeze.
	_tick(fa)
	_check_invariant(fa, "frozen + _process tick #1")
	_tick(fa)
	_check_invariant(fa, "frozen + _process tick #2")

	_set_on_screen(fa, true)
	_check_invariant(fa, "back on-screen MID (restore)")
	if not fa._anim_throttle_active:
		_fail("back on-screen MID: throttle not restored (should be throttling)")

	# Full matrix of on/off × every LOD, each followed by a tick.
	for on_screen in [true, false]:
		for lod in [FULL, MID, CROSSFADE, FAR]:
			_set_lod(fa, lod)
			_set_on_screen(fa, on_screen)
			_tick(fa)
			_check_invariant(fa, "matrix on=%s LOD=%d" % [on_screen, lod])

	fa.queue_free()


# Guard that the invariant check actually detects the historical bug state,
# constructed by hand the way the old freeze + unguarded re-activation produced it.
func _test_detects_frozen_while_drawn() -> void:
	var fa := _make_fake_avatar()
	fa._on_screen = true
	fa._lod_state = MID
	H.set_animation_throttle(fa, true)  # MANUAL + throttle flag on
	# Old bug: freeze cleared the flag directly, leaving callback in MANUAL …
	fa._anim_throttle_active = false
	# … then the unguarded re-activation forced the tree back on.
	fa.animation_tree.active = true

	var before := _failures.size()
	_check_invariant(fa, "synthetic old-bug state")
	if _failures.size() == before:
		_fail("invariant check failed to detect the frozen-while-drawn bug state")
	else:
		# Expected detections — drop them so the suite still passes.
		_failures.resize(before)

	fa.queue_free()


func _expect_drive(ctx: String, d: Dictionary, active: bool, throttle: bool) -> void:
	if d.get("active") != active or d.get("throttle") != throttle:
		_fail(
			(
				"%s: resolve_anim_drive=%s, expected {active:%s, throttle:%s}"
				% [ctx, str(d), active, throttle]
			)
		)


func _expect_eq(ctx: String, actual: int, expected: int) -> void:
	if actual != expected:
		_fail("%s: got %d, expected %d" % [ctx, actual, expected])


func _fail(msg: String) -> void:
	_failures.append(msg)


func _finish() -> void:
	if _failures.is_empty():
		print("[test_avatar_anim_throttle] PASS")
		quit(0)
		return
	for f in _failures:
		printerr(f)
	printerr("[test_avatar_anim_throttle] FAIL: %d case(s)" % _failures.size())
	quit(1)
