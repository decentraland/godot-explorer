extends SceneTree

# Regression test for #2603: using an action while gliding left the state
# machine in Idle while still airborne, and the unguarded Idle -> Walk
# transition (ordered before Idle -> Jump_Fall) played the walk animation
# mid-air.
#
# Pins two things:
#   1. AvatarAnimHelpers.locomotion_conditions gates walk/jog/run on is_grounded.
#   2. A state machine mirroring avatar.tscn's Idle transitions, fed through
#      that helper, picks Jump_Fall (never Walk) for an airborne moving avatar.
#
# Run headless:
#   Godot --headless --path godot \
#     --script res://src/test/avatar/test_avatar_locomotion_grounded.gd

const H := preload("res://src/decentraland_components/avatar/avatar_anim_helpers.gd")

var _failures: Array[String] = []


func _initialize() -> void:
	_test_mapping()
	_test_state_machine_picks_fall_when_airborne()
	_test_glide_reopens_from_idle()
	_test_glide_beats_walk_when_airborne()
	_test_rig_detects_unguarded_walk()
	_finish()


func _test_mapping() -> void:
	var airborne: Dictionary = H.locomotion_conditions(true, true, true, false)
	if airborne.walk or airborne.jog or airborne.run:
		_fail("airborne: locomotion conditions must all be false, got %s" % str(airborne))

	var grounded: Dictionary = H.locomotion_conditions(true, false, true, true)
	if not grounded.walk or grounded.jog or not grounded.run:
		_fail("grounded: flags must pass through, got %s" % str(grounded))


# Builds the production-relevant slice of avatar.tscn's state machine: Idle,
# Walk, Jump_Fall, with Idle -> Walk (condition walk) ordered BEFORE
# Idle -> Jump_Fall (condition fall), matching the .tscn transition order.
func _make_tree() -> AnimationTree:
	var tree := AnimationTree.new()
	var lib := AnimationLibrary.new()
	var anim := Animation.new()
	anim.length = 1.0
	anim.loop_mode = Animation.LOOP_LINEAR
	lib.add_animation("loop", anim)
	tree.add_animation_library("", lib)

	var sm := AnimationNodeStateMachine.new()
	for state in ["Idle", "Walk", "Jump_Fall", "Gliding_Start"]:
		var node := AnimationNodeAnimation.new()
		node.animation = "loop"
		sm.add_node(state, node)

	var to_walk := AnimationNodeStateMachineTransition.new()
	to_walk.advance_mode = AnimationNodeStateMachineTransition.ADVANCE_MODE_AUTO
	to_walk.advance_condition = &"walk"
	sm.add_transition("Idle", "Walk", to_walk)

	var to_fall := AnimationNodeStateMachineTransition.new()
	to_fall.advance_mode = AnimationNodeStateMachineTransition.ADVANCE_MODE_AUTO
	to_fall.advance_condition = &"fall"
	sm.add_transition("Idle", "Jump_Fall", to_fall)

	var to_glide := AnimationNodeStateMachineTransition.new()
	to_glide.advance_mode = AnimationNodeStateMachineTransition.ADVANCE_MODE_AUTO
	to_glide.advance_condition = &"gliding"
	sm.add_transition("Idle", "Gliding_Start", to_glide)

	tree.tree_root = sm
	tree.active = true
	root.add_child(tree)
	return tree


# Simulates avatar.gd's per-tick condition writes. The bug scenario: gliding
# descends slowly, so vertical speed sits inside the rise/fall deadband
# (fall=false) while horizontal input keeps walk=true and is_grounded=false.
func _drive(tree: AnimationTree, use_helper: bool, is_grounded: bool, fall: bool) -> String:
	var playback: AnimationNodeStateMachinePlayback = tree.get("parameters/playback")
	playback.start("Idle")

	var walk_cond: bool
	if use_helper:
		walk_cond = H.locomotion_conditions(true, false, false, is_grounded).walk
	else:
		walk_cond = true  # pre-fix mapping: raw flag, no grounded gate

	tree.set("parameters/conditions/walk", walk_cond)
	tree.set("parameters/conditions/fall", fall)
	tree.advance(0.05)
	var state: String = playback.get_current_node()
	tree.queue_free()
	return state


func _test_state_machine_picks_fall_when_airborne() -> void:
	# Glide-descent deadband: airborne, moving, fall=false. Must NOT walk.
	var state := _drive(_make_tree(), true, false, false)
	if state == "Walk":
		_fail("airborne + moving (glide deadband): reached Walk (bug #2603)")
	elif state != "Idle":
		_fail("airborne + moving (glide deadband): expected Idle, got '%s'" % state)

	# Real falling: airborne, moving, fall=true. Must go to Jump_Fall.
	state = _drive(_make_tree(), true, false, true)
	if state != "Jump_Fall":
		_fail("airborne + falling: expected Jump_Fall, got '%s'" % state)

	# Normal ground locomotion must be unaffected.
	state = _drive(_make_tree(), true, true, false)
	if state != "Walk":
		_fail("grounded + moving: expected Walk, got '%s'" % state)


# Reopening the glider after a ground-graze close leaves the state machine in
# Idle (Gliding_End -> Idle is at-end). The sustained `gliding` condition must
# bring it back to Gliding_Start even after the 0.5s start_glide window expired,
# or the avatar glides with an Idle body.
func _test_glide_reopens_from_idle() -> void:
	var tree := _make_tree()
	var playback: AnimationNodeStateMachinePlayback = tree.get("parameters/playback")
	playback.start("Idle")
	tree.set("parameters/conditions/gliding", true)
	tree.advance(0.05)
	var state: String = playback.get_current_node()
	tree.queue_free()
	if state != "Gliding_Start":
		_fail("glide reopen from Idle: expected Gliding_Start, got '%s'" % state)


# Remote glider on the compressed update path: horizontal movement flags are
# on but is_grounded stays false (dcl_avatar.rs derivation excludes glide
# states), so the walk condition is gated off and gliding must win even though
# Idle -> Walk is ordered before Idle -> Gliding_Start.
func _test_glide_beats_walk_when_airborne() -> void:
	var tree := _make_tree()
	var playback: AnimationNodeStateMachinePlayback = tree.get("parameters/playback")
	playback.start("Idle")
	var loco: Dictionary = H.locomotion_conditions(true, false, false, false)
	tree.set("parameters/conditions/walk", loco.walk)
	tree.set("parameters/conditions/gliding", true)
	tree.advance(0.05)
	var state: String = playback.get_current_node()
	tree.queue_free()
	if state != "Gliding_Start":
		_fail("airborne gliding peer: expected Gliding_Start, got '%s'" % state)


# Guard that the rig actually mirrors production ordering: with the old
# unguarded mapping, Walk must win in the deadband scenario — if it doesn't,
# the test above is vacuous.
func _test_rig_detects_unguarded_walk() -> void:
	var state := _drive(_make_tree(), false, false, false)
	if state != "Walk":
		_fail("rig check: unguarded walk should reach Walk, got '%s'" % state)


func _fail(msg: String) -> void:
	_failures.append(msg)


func _finish() -> void:
	if _failures.is_empty():
		print("[test_avatar_locomotion_grounded] PASS")
		quit(0)
		return
	for f in _failures:
		printerr(f)
	printerr("[test_avatar_locomotion_grounded] FAIL: %d case(s)" % _failures.size())
	quit(1)
