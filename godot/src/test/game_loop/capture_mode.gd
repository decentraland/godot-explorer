class_name CaptureMode
extends RefCounted

## Momentary global freeze for deterministic screenshots.
##
## Games are never still: avatars loop Idle + play emotes (wave/fistpump/shrug),
## foot-dust particles emit, the login spinner spins, tweens run. So a screenshot's
## pixels depend on which tick you grab it on — useless for a smoke test's
## distinct-color count and impossible to compare across runs.
##
## `async_enter()` freezes everything animatable to a canonical, still state; the
## caller renders a frame and reads it back; `exit()` restores.
##
## What it freezes, and how:
##   * SDK scenes (Rust runner)         -> Global.scene_runner.set_pause(true)
##   * avatars -> canonical idle pose   -> emote_controller.freeze_on_idle()
##   * avatar foot-dust particles       -> AvatarLODHelpers.set_particles_visible(false)
##   * UI spinner / tweens / flipbooks  -> Engine.time_scale = 0  (delta -> 0)
##
## ORDER MATTERS: freeze_on_idle() only disables the AnimationTree; the AnimationPlayer needs
## a few REAL frames (normal time_scale) to settle off the emote onto Idle (the proven path:
## avatar_preview.async_get_viewport_image = freeze_on_idle + 5 process_frames). So settle at
## normal time_scale FIRST, THEN time_scale=0 — pinning too early freezes a mid-emote pose.
##
## time_scale=0 (not get_tree().paused) so rendering keeps running (frame_post_draw fires for
## readback) and PROCESS_MODE_ALWAYS nodes are covered. GOTCHA: while time_scale==0,
## create_timer().timeout NEVER fires — after async_enter() await ONLY frame_post_draw /
## process_frame until exit().

## Real frames to let the Idle pose apply after freeze_on_idle (matches the proven
## avatar_preview snapshot path).
const SETTLE_FRAMES := 5
## Fixed phase (seconds) every LOOPING UI animation (login spinner, flipbooks) is
## pinned to. time_scale=0 alone freezes them at a wall-clock-dependent phase — the
## same spinner lands on a different angle each run — so we seek them to this canonical
## time. Any constant works as long as it's identical run-to-run; 0 = the loop start.
const CANONICAL_ANIM_TIME := 0.0

var _prev_time_scale := 1.0
var _frozen_avatars: Array = []
var _snapped_previews: Array = []
## Nodes hidden for the capture, as [node, previous_visible] pairs (restored on exit).
var _hidden_for_capture: Array = []


## Freeze everything to a still, deterministic state, then return. Pair with exit().
## Async because the avatars need a few real frames to settle onto the idle pose.
## `settle_frames` overrides how many REAL frames to advance before pinning time — bump
## it for a screen whose intro animation needs longer to reach its settled end state.
## `hide_world` blanks the live 3D world + SDK UI (default) — pass false for an IN-WORLD
## capture that WANTS the loaded scene visible (still frozen via scene pause + time_scale=0).
func async_enter(settle_frames: int = SETTLE_FRAMES, hide_world: bool = true) -> void:
	# 0. Once past the lobby, the live 3D world + SDK scene UI are non-deterministic
	#    (scene load / spawn / skybox). Hide them so the capture never draws them — the
	#    persistent chrome/UI stays. No-op during the lobby (no explorer yet). Skipped for
	#    an in-world capture that deliberately wants the (loaded, frozen) scene on screen.
	if hide_world:
		_hide_live_world()

	# 1. SDK-driven scene animation (Rust side) — entity transforms, scene tweens/emotes.
	if Global.scene_runner != null:
		Global.scene_runner.set_pause(true)

	# 1b. Snap every avatar-preview camera to its lerp target so the framing/rotation
	#     is identical across runs (the exponential camera lerp is otherwise the main
	#     source of the ~1px cross-run silhouette jitter).
	_snapped_previews.clear()
	for preview in _all_avatar_previews():
		preview.snap_camera_for_capture()
		_snapped_previews.append(preview)

	# 2. Every avatar -> canonical Idle pose (cancels wave/fistpump/shrug emotes),
	#    and hide foot-dust particles (their sim doesn't honor time_scale).
	_frozen_avatars.clear()
	for avatar in _all_avatars():
		if avatar.emote_controller != null:
			avatar.emote_controller.freeze_on_idle()
		AvatarLODHelpers.set_particles_visible(avatar, false)
		_frozen_avatars.append(avatar)

	# 3. Let real frames elapse (normal time_scale) so freeze_on_idle's Idle pose
	#    actually applies before we freeze time. Skipping this leaves the avatar on
	#    the emote pose it was mid-way through.
	var tree := Engine.get_main_loop() as SceneTree
	if tree != null:
		for _i in range(maxi(settle_frames, 0)):
			await tree.process_frame

	# 4. Global delta -> 0: pins the UI spinner, tweens, AnimatedTextureRect flipbooks,
	#    and any remaining delta-driven animation. Rendering keeps running.
	_prev_time_scale = Engine.time_scale
	Engine.time_scale = 0.0

	# 5. Canonicalize UI AnimationPlayer phase. time_scale=0 froze them, but at whatever
	#    (wall-clock-dependent) phase they'd reached — a spinner sits at a random angle,
	#    a mid-play intro at a random point. Seek loops to a fixed phase and one-shots to
	#    their settled end so the frozen frame is identical every run.
	_canonicalize_animation_players()


## Restore live animation. Safe to call even if async_enter() froze nothing.
func exit() -> void:
	Engine.time_scale = _prev_time_scale

	if Global.scene_runner != null:
		Global.scene_runner.set_pause(false)

	for avatar in _frozen_avatars:
		if not is_instance_valid(avatar):
			continue
		# freeze_on_idle() disabled the AnimationTree node; re-enable it. avatar._process
		# re-drives the state-machine conditions on the next frame.
		if avatar.animation_tree != null:
			avatar.animation_tree.process_mode = Node.PROCESS_MODE_INHERIT
			avatar.animation_tree.active = true
		AvatarLODHelpers.set_particles_visible(avatar, true)
	_frozen_avatars.clear()

	for preview in _snapped_previews:
		if is_instance_valid(preview):
			preview.resume_after_capture()
	_snapped_previews.clear()

	for pair in _hidden_for_capture:
		if is_instance_valid(pair[0]):
			pair[0].visible = pair[1]
	_hidden_for_capture.clear()


## Hides the live 3D world (explorer's `world` Node3D) and the SDK scene UI
## (`scene_runner.base_ui`) so a post-lobby capture doesn't draw non-deterministic scene
## content. Records each node's previous visibility so exit() restores it. No-op in the
## lobby, where there is no explorer yet.
func _hide_live_world() -> void:
	_hidden_for_capture.clear()
	var explorer := Global.get_explorer()
	if explorer != null and is_instance_valid(explorer.world):
		_hidden_for_capture.append([explorer.world, explorer.world.visible])
		explorer.world.visible = false
	if Global.scene_runner != null and is_instance_valid(Global.scene_runner.base_ui):
		var base_ui = Global.scene_runner.base_ui
		_hidden_for_capture.append([base_ui, base_ui.visible])
		base_ui.visible = false


## Every live Avatar node anywhere in the tree. Crucially this includes the lobby's
## avatar-preview avatar (used on the login/onboarding screens), which is NOT under
## Global.avatars nor the scene player — so a Global.avatars-only walk would miss the
## one avatar that's actually on screen during login and leave it frozen mid-emote.
## owned=false so runtime-instantiated avatars (the preview) are found too.
func _all_avatars() -> Array:
	var tree := Engine.get_main_loop() as SceneTree
	if tree == null or tree.root == null:
		return []
	return tree.root.find_children("*", "Avatar", true, false)


## Every AvatarPreview in the tree (the login/onboarding avatar viewers).
func _all_avatar_previews() -> Array:
	var tree := Engine.get_main_loop() as SceneTree
	if tree == null or tree.root == null:
		return []
	return tree.root.find_children("*", "AvatarPreview", true, false)


## Pins every currently-playing UI AnimationPlayer to a deterministic pose. Avatar
## players are skipped — their pose is already settled to canonical idle via
## freeze_on_idle() and re-seeking would disturb it. LOOPING animations (spinner,
## flipbooks) are seeked to CANONICAL_ANIM_TIME (fixed phase); one-shot animations
## (screen intros) are seeked to their END so a capture taken mid-intro still shows
## the settled final frame. Call AFTER time_scale=0 so nothing re-advances them.
func _canonicalize_animation_players() -> void:
	var tree := Engine.get_main_loop() as SceneTree
	if tree == null or tree.root == null:
		return
	for player in tree.root.find_children("*", "AnimationPlayer", true, false):
		if _is_under_avatar(player):
			continue
		var anim_name: String = player.current_animation
		if anim_name.is_empty():
			continue
		var anim: Animation = player.get_animation(anim_name)
		if anim == null:
			continue
		if anim.loop_mode == Animation.LOOP_NONE:
			player.seek(anim.length, true)
		else:
			player.seek(CANONICAL_ANIM_TIME, true)


## Whether `node` sits inside an Avatar subtree (whose animation is handled separately).
func _is_under_avatar(node: Node) -> bool:
	var p := node.get_parent()
	while p != null:
		if p is Avatar:
			return true
		p = p.get_parent()
	return false
