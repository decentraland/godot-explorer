extends SceneTree

# Midrange gliding statue repro, take 2: avatar.tscn wires the AnimationTree
# to an AnimationPlayer that has autoplay="Idle". With the tree in IDLE
# callback mode (FULL) it evaluates after the player every frame and wins.
# In MANUAL mode (MID throttle) it only writes bones on advance() every 3rd
# frame, so the player's Idle stomps the tree's pose in between.
#
# The rig wires a tree + autoplaying player the same way and samples the
# driven value every frame. Fix under test: drop the player's autoplay.
#
# Run headless:
#   Godot --headless --path godot \
#     --script res://src/test/avatar/test_avatar_autoplay_stomp.gd

var _failures: Array[String] = []


class Probe:
	extends Node
	var dummy: float = -1.0


# gdlint:ignore = async-function-name
func _initialize() -> void:
	await _async_test(true)  # autoplay on: expect the stomp (documents the bug)
	await _async_test(false)  # autoplay off (the fix): tree must own the pose
	_finish()


func _make_anim(value: float) -> Animation:
	var anim := Animation.new()
	anim.length = 1.0
	anim.loop_mode = Animation.LOOP_LINEAR
	var track := anim.add_track(Animation.TYPE_VALUE)
	anim.track_set_path(track, "../Probe:dummy")
	anim.track_insert_key(track, 0.0, value)
	return anim


func _async_test(with_autoplay: bool) -> void:
	var ctx: String = "autoplay" if with_autoplay else "no-autoplay"

	var rig := Node.new()
	root.add_child(rig)
	var probe := Probe.new()
	probe.name = "Probe"
	root.add_child(probe)

	var player := AnimationPlayer.new()
	rig.add_child(player)
	var lib := AnimationLibrary.new()
	lib.add_animation("idle_anim", _make_anim(0.0))
	lib.add_animation("glide_anim", _make_anim(1.0))
	player.add_animation_library("", lib)
	# AnimationPlayer root_node defaults to ".." (the rig), so tracks resolve
	# against root/Probe via the relative path above.

	var tree := AnimationTree.new()
	rig.add_child(tree)
	tree.anim_player = tree.get_path_to(player)
	var sm := AnimationNodeStateMachine.new()
	var idle_node := AnimationNodeAnimation.new()
	idle_node.animation = "idle_anim"
	sm.add_node("Idle", idle_node)
	var glide_node := AnimationNodeAnimation.new()
	glide_node.animation = "glide_anim"
	sm.add_node("Gliding_Idle", glide_node)
	var to_glide := AnimationNodeStateMachineTransition.new()
	to_glide.advance_mode = AnimationNodeStateMachineTransition.ADVANCE_MODE_AUTO
	to_glide.advance_condition = &"gliding"
	sm.add_transition("Idle", "Gliding_Idle", to_glide)
	tree.tree_root = sm
	tree.active = true

	var playback: AnimationNodeStateMachinePlayback = tree.get("parameters/playback")
	playback.start("Idle")
	if with_autoplay:
		player.play("idle_anim")  # what autoplay="Idle" does on ready
	tree.set("parameters/conditions/gliding", true)

	# Production MID drive: tree MANUAL, advance every 3rd frame; player keeps
	# its own IDLE callback and updates every frame.
	tree.callback_mode_process = AnimationMixer.ANIMATION_CALLBACK_MODE_PROCESS_MANUAL

	var acc := 0.0
	var dt := 1.0 / 60.0
	var stomped_frames := 0
	var total_frames := 0
	for i in range(60):
		acc += dt
		if i % 3 == 2:
			tree.advance(acc)
			acc = 0.0
		# Let the scene tree run one real frame so the player's own IDLE
		# processing applies its pose, like in the running client.
		await process_frame
		total_frames += 1
		if is_zero_approx(probe.dummy):
			stomped_frames += 1

	var state: String = playback.get_current_node()
	print(
		(
			"[%s] state=%s stomped=%d/%d (dummy=0 means player Idle won that frame)"
			% [ctx, state, stomped_frames, total_frames]
		)
	)

	if state != "Gliding_Idle":
		_fail("%s: rig did not reach Gliding_Idle (got %s)" % [ctx, state])
	elif with_autoplay and stomped_frames == 0:
		_fail("autoplay: no stomp observed — rig does not reproduce the bug")
	elif not with_autoplay and stomped_frames > 0:
		_fail("no-autoplay: player still stomped %d frames" % stomped_frames)

	rig.queue_free()
	probe.queue_free()


func _fail(msg: String) -> void:
	_failures.append(msg)


func _finish() -> void:
	if _failures.is_empty():
		print("[test_avatar_autoplay_stomp] PASS")
		quit(0)
		return
	for f in _failures:
		printerr(f)
	printerr("[test_avatar_autoplay_stomp] FAIL: %d case(s)" % _failures.size())
	quit(1)
