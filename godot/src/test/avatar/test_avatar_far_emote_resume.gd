extends SceneTree

# Regression test: a scene NPC (AvatarShape) keeps its looping emote across FAR LOD.
#
# At FAR the avatar is an impostor and any running emote is stopped. A remote player
# re-announces a loop, but a scene sends AvatarShape.expressionTriggerId only once, so a
# dancing NPC that went FAR (as it does at spawn) never danced again and, once the scene
# moved it, walked instead. A looping emote must come back when the NPC leaves FAR; a
# one-shot emote, a player's emote, or one the scene stopped meanwhile must not. The
# replay waits a frame: started mid-_process, a moving NPC would cancel it at once.
#
# Run headless:
#   Godot --headless --path godot \
#     --script res://src/test/avatar/test_avatar_far_emote_resume.gd

# Loaded at run time, untyped: avatar.gd needs the Global autoload, which does not
# exist yet while this script compiles.
const AVATAR_SCENE := "res://src/decentraland_components/avatar/avatar.tscn"
const CONTROLLER := "res://src/decentraland_components/avatar/avatar_emote_controller.gd"
const SPAWN_REVEAL := "res://src/decentraland_components/avatar/avatar_spawn_reveal.gd"
const DANCE := "urn:decentraland:off-chain:base-emotes:dance"
const SCENE_ID := 7
# Avatar.LODState
const FULL := 0
const MID := 1
const FAR := 3


func _initialize() -> void:
	# After the first frame: the autoloads are in the tree by then.
	process_frame.connect(_async_run, CONNECT_ONE_SHOT)


func _async_run() -> void:
	var failures: Array[String] = []

	var avatar = _new_avatar(true)
	_start(avatar, true)
	avatar._on_lod_state_changed(FAR, FULL)
	if avatar.emote_controller.is_playing():
		failures.append("[npc loop] still playing at FAR")
	avatar._on_lod_state_changed(MID, FAR)
	# Not in the same frame: a moving NPC's gait for this frame is already set.
	_expect_plays(failures, avatar, [], "npc loop, not resumed mid-frame")
	await process_frame
	_expect_plays(failures, avatar, [[DANCE, -1, SCENE_ID]], "npc loop, back from FAR")
	avatar._on_lod_state_changed(FULL, MID)
	await process_frame
	_expect_plays(failures, avatar, [[DANCE, -1, SCENE_ID]], "npc loop, resumed only once")
	avatar.free()

	# The anti-spam cooldown must not drop the resume.
	avatar = _new_avatar(true)
	_start(avatar, true)
	avatar._on_lod_state_changed(FAR, FULL)
	avatar.emote_controller._last_emote_time = Time.get_ticks_msec() / 1000.0
	await _async_back_from_far(avatar)
	_expect_plays(failures, avatar, [[DANCE, -1, SCENE_ID]], "npc loop, within the cooldown")
	avatar.free()

	avatar = _new_avatar(true)
	_start(avatar, false)
	avatar._on_lod_state_changed(FAR, FULL)
	await _async_back_from_far(avatar)
	_expect_plays(failures, avatar, [], "npc one-shot")
	avatar.free()

	avatar = _new_avatar(false)
	_start(avatar, true)
	avatar._on_lod_state_changed(FAR, FULL)
	await _async_back_from_far(avatar)
	_expect_plays(failures, avatar, [], "player loop")
	avatar.free()

	avatar = _new_avatar(true)
	_start(avatar, true)
	avatar._on_lod_state_changed(FAR, FULL)
	avatar.emote_controller.stop_emote()
	await _async_back_from_far(avatar)
	_expect_plays(failures, avatar, [], "npc loop stopped by the scene while FAR")
	avatar.free()

	if failures.is_empty():
		print("[test_avatar_far_emote_resume] PASS")
		quit(0)
		return
	for f in failures:
		printerr(f)
	printerr("[test_avatar_far_emote_resume] FAIL: %d case(s) failing" % failures.size())
	quit(1)


# Left out of the tree, so avatar.gd's _ready() (and its loading) never runs: wire by
# hand what _on_lod_state_changed touches, with a controller that records its plays.
func _new_avatar(is_shape: bool):
	var avatar = load(AVATAR_SCENE).instantiate()
	avatar.is_avatar_shape = is_shape
	avatar.animation_tree = avatar.get_node("AnimationTree")
	avatar.animation_player = avatar.get_node("AnimationPlayer")
	avatar._spawn_reveal = load(SPAWN_REVEAL).new(avatar)
	avatar.emote_controller = _recording_controller().new(
		avatar, avatar.animation_player, avatar.animation_tree
	)
	return avatar


# Leaving FAR: the resume is deferred, so it has landed by the next frame.
func _async_back_from_far(avatar) -> void:
	avatar._on_lod_state_changed(MID, FAR)
	await process_frame


# What play_emote leaves behind for an emote that is running.
func _start(avatar, looping: bool) -> void:
	var ec = avatar.emote_controller
	ec.loaded_emotes_by_urn[DANCE] = null
	ec.current_emote_urn = DANCE
	ec.current_emote_mask = -1
	ec.current_emote_scene_id = SCENE_ID
	ec.playing_single = true
	ec.playing_loop = looping


func _expect_plays(failures: Array[String], avatar, expected: Array, case_name: String) -> void:
	var plays: Array = avatar.emote_controller.plays
	if plays != expected:
		failures.append("[%s] plays %s, expected %s" % [case_name, plays, expected])


# The controller, with play_emote recording its calls. Built at run time for the same
# reason the avatar is loaded then: the controller needs the Global autoload.
func _recording_controller() -> GDScript:
	var script := GDScript.new()
	script.source_code = (
		(
			'extends "%s"\n'
			+ "var plays: Array = []\n"
			+ "func play_emote(id: String, mask: int = -1, owner_scene_id: int = -1):\n"
			+ "\tplays.append([id, mask, owner_scene_id])\n"
		)
		% CONTROLLER
	)
	script.reload()
	return script
