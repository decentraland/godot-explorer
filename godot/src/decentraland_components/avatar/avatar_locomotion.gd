class_name AvatarLocomotion
extends RefCounted

# Locomotion animation state for one avatar: the gait blend, the jump chain, the
# glide and the stun, published as the AnimationTree conditions avatar.tscn reads.
# The avatar reads its movement once a frame (~1 us per read) and hands it over.

# #2856 gait blend (Unity CharacterControllerSettings): blend normalization
# caps per gait kind (0 idle / 1 walk / 2 jog / 3 run) and approach speed.
const GAIT_BLEND_MAX_SPEEDS := [0.0, 1.5, 8.0, 10.0]
const GAIT_BLEND_SPEED := 5.0
const STILL_BLEND_THRESHOLD := 0.25
# #2855: Unity LongFallStunTime — the hard-landing stun animation covers the
# input lock (loco2 M6).
const LONG_FALL_STUN_TIME := 0.75
# Unity JumpHeightStun: the stun comes from a fall HEIGHT, not from the fall-
# speed flag the wire carries (that one trips at ~3.3m). Remotes measure their
# own height against this.
const HARD_LANDING_FALL_HEIGHT := 8.0
# Blend-space children in blend order; their clip lengths drive the cadence warp.
const GAIT_CLIPS := ["Idle", "Walk", "Jog", "Run"]
# Authored cycle length per gait. The clips ship retimed to one common length so
# the blend space cannot phase them apart; these restore each gait's cadence.
const GAIT_CYCLE_LENGTHS := [2.0, 1.0, 0.6666667, 0.5333333]

var _tree: AnimationTree
var _sm_double_jump_rise: AnimationNodeAnimation
var _gait_clip_length: float = 0.0
# Continuous gait blend (Unity MovementBlend), also read by the emote gate.
var _movement_blend: float = 0.0
# #2856: this air jump's variant, 0|1 (Unity JumpVariation).
var _jump_variation: float = 0.0
# Held until the SM enters Double_Jump_Rise (or the avatar lands): a 1-frame
# pulse races the tree's advance order and can be swallowed.
var _double_jump_latch := false
var _last_jump_count := 0
# the first frame must not read a wire-provided jump_count >= 2 as a rising
# edge, or a remote avatar first seen mid-double-jump plays SFX from nothing.
var _jump_count_sync_pending := true
# Apex of the remote avatar's airborne stretch, read from its own y.
var _remote_fall_apex_y := 0.0
var _remote_stun_timer := 0.0
var _stunned_now := false
var _glide_forward_blend := 0.0
# Reused so a frame allocates nothing; callers read it before the next call.
var _result: Dictionary = {}


func _init(tree: AnimationTree) -> void:
	_tree = tree
	var sm: AnimationNodeStateMachine = tree.tree_root.get_node("Locomotion")
	_sm_double_jump_rise = sm.get_node("Double_Jump_Rise")


# One frame of locomotion conditions. `state` carries what the avatar read from
# itself — run, jog, walk, rise, fall, land, grounded, fall_fast, long_fall,
# jump_count, stunned, glide_state, movement_speed, position_y — plus `local`
# and the emote controller's playing_single/playing_mixed. The dictionary is
# only read, never kept, so the caller may refill and reuse it. Returns the
# values the avatar still needs — grounded, stunned, movement_speed,
# jump_rising_edge — in a dictionary it must read before the next call.
func apply(delta: float, state: Dictionary) -> Dictionary:
	var run_now: bool = state["run"]
	var jog_now: bool = state["jog"]
	var walk_now: bool = state["walk"]
	var rise_now: bool = state["rise"]
	var fall_now: bool = state["fall"]
	var land_now: bool = state["land"]
	var grounded_now: bool = state["grounded"]
	var fall_fast_now: bool = state["fall_fast"]
	var long_fall_now: bool = state["long_fall"]
	var jump_count_now: int = state["jump_count"]
	var stunned_now: bool = state["stunned"]
	var glide_state_now: int = state["glide_state"]
	var movement_speed_now: float = state["movement_speed"]
	var is_local_player: bool = state["local"]

	# Full-body emotes only from a standstill; masked (upper-body) mixes stay
	# available while moving.
	_tree.set(
		"parameters/Locomotion/conditions/emote",
		state["emote_single"] and _movement_blend < STILL_BLEND_THRESHOLD
	)
	_tree.set("parameters/Locomotion/conditions/nemote", not state["emote_single"])
	_tree.set(
		"parameters/Locomotion/conditions/emix",
		state["emote_mixed"] and _movement_blend < STILL_BLEND_THRESHOLD
	)
	_tree.set("parameters/Locomotion/conditions/nemix", not state["emote_mixed"])

	# #2856: continuous gait blend (Unity MovementBlend). kind comes from the
	# walk/jog/run classification (local input mode / remote wire), speed is the
	# measured horizontal speed; blend = clamp01(speed/blendMax)*kind, approached
	# at MoveAnimBlendSpeed (5/s, scaled by distance). Sides are the same on both
	# ends (remotes get them off the wire).
	var kind := 0
	if run_now:
		kind = 3
	elif jog_now:
		kind = 2
	elif walk_now:
		kind = 1
	var target_blend := 0.0
	if kind > 0:
		target_blend = clampf(movement_speed_now / GAIT_BLEND_MAX_SPEEDS[kind], 0.0, 1.0) * kind
	var blend_rate: float = maxf(
		GAIT_BLEND_SPEED, GAIT_BLEND_SPEED * absf(target_blend - _movement_blend)
	)
	_movement_blend = move_toward(_movement_blend, target_blend, blend_rate * delta)
	_tree.set("parameters/Locomotion/Grounded/gait/blend_position", _movement_blend)
	_warp_cadence(grounded_now)
	# #2856: jump-chain selection by gait, as single-condition transitions.
	_tree.set("parameters/Locomotion/conditions/rise_walk", rise_now and kind < 3)
	_tree.set("parameters/Locomotion/conditions/rise_run", rise_now and kind == 3)
	# #2855: bounce pads launch without a jump (jump_count stays 0) — they skip
	# the Jump_Start anticipation and go straight to the rise loop.
	_tree.set("parameters/Locomotion/conditions/rise_bounce", rise_now and jump_count_now == 0)

	_tree.set("parameters/Locomotion/conditions/rise", rise_now)
	_tree.set("parameters/Locomotion/conditions/fall", fall_now)
	_tree.set("parameters/Locomotion/conditions/land", land_now)
	# #1553: Unity anim thresholds — fall pose past vy<-3 (AnimationFallSpeed);
	# apex = airborne between rise end and fall start (the Mid state floats it).
	_tree.set("parameters/Locomotion/conditions/fall_fast", fall_fast_now)
	_tree.set(
		"parameters/Locomotion/conditions/apex",
		not grounded_now and not rise_now and not fall_fast_now
	)
	# nfall reads is_grounded, not `land`: `land` is a short local pulse
	# (in_grace_time), so remotes would diverge. is_grounded has the same shape
	# on both sides and still avoids fall's 1-2 frame deadband at apex.
	_tree.set("parameters/Locomotion/conditions/nfall", grounded_now)

	# #2855: long fall + hard-landing stun. Local avatar: stunned comes from the
	# M6 fall-height timer (player.gd). Remote: derive — landed while the wire
	# said long_fall this airtime.
	if is_local_player:
		_stunned_now = stunned_now
	else:
		# The sender stuns on fall height (Unity JumpHeightStun, 8m); the wire
		# only carries the fall-speed flag, so an 8m drop and a 3.3m one look
		# identical there. This avatar's own y is the same measure.
		var y_now: float = state["position_y"]
		if grounded_now:
			if _remote_fall_apex_y - y_now > HARD_LANDING_FALL_HEIGHT:
				_remote_stun_timer = LONG_FALL_STUN_TIME
			_remote_fall_apex_y = y_now
		else:
			_remote_fall_apex_y = maxf(_remote_fall_apex_y, y_now)
		_remote_stun_timer = maxf(_remote_stun_timer - delta, 0.0)
		_stunned_now = _remote_stun_timer > 0.0
	_tree.set("parameters/Locomotion/conditions/long_fall", long_fall_now)
	_tree.set("parameters/Locomotion/conditions/fall_short", fall_fast_now and not long_fall_now)
	_tree.set("parameters/Locomotion/conditions/stunned", _stunned_now)
	_tree.set("parameters/Locomotion/conditions/nstunned", not _stunned_now)
	# Long_Fall exits: hard landing (stunned) must beat the normal landing —
	# land_soft only fires when grounded without an active stun.
	_tree.set("parameters/Locomotion/conditions/land_soft", grounded_now and not _stunned_now)

	# Rising-edge detection for one-frame AnimationTree condition pulses.
	var jump_rising_edge: bool = jump_count_now > _last_jump_count and jump_count_now >= 2
	if _jump_count_sync_pending:
		# First frame: a wire-provided jump_count is not an edge.
		jump_rising_edge = false
		_jump_count_sync_pending = false
	_double_jump_latch = _double_jump_latch or jump_rising_edge
	# #2856: jump variation — every air jump re-rolls 0|1 and the double-jump rise
	# swaps between the two clip variants for the current gait (Unity's Jump state
	# blends Double_Jump_<gait> x JumpVariation).
	if jump_count_now != _last_jump_count:
		_jump_variation = 0.0 if randf() < 0.5 else 1.0
		var gait := "Base"
		if run_now:
			gait = "Run"
		elif jog_now:
			gait = "Jog"
		var dj_clip := "Double_Jump_" + gait + ("_Right" if _jump_variation > 0.5 else "")
		_sm_double_jump_rise.animation = StringName("double_jump_and_glide/" + dj_clip)
	_last_jump_count = jump_count_now
	var gliding_now: bool = glide_state_now == 1 or glide_state_now == 2
	_tree.set("parameters/Locomotion/conditions/double_jump", _double_jump_latch)
	_tree.set("parameters/Locomotion/conditions/gliding", gliding_now)
	_tree.set("parameters/Locomotion/conditions/ngliding", not gliding_now)

	var glide_moving: bool = walk_now or jog_now or run_now
	var glide_forward_target: float = 1.0 if glide_moving else 0.0
	_glide_forward_blend = move_toward(_glide_forward_blend, glide_forward_target, delta * 4.0)
	_tree.set("parameters/Locomotion/Gliding_Idle/Blend2/blend_amount", _glide_forward_blend)

	# Clear the latch once consumed (entered the state) or moot (landed).
	if _double_jump_latch:
		var pb: AnimationNodeStateMachinePlayback = _tree.get("parameters/Locomotion/playback")
		if pb.get_current_node() == "Double_Jump_Rise" or grounded_now:
			_double_jump_latch = false

	_result["grounded"] = grounded_now
	_result["stunned"] = _stunned_now
	_result["movement_speed"] = movement_speed_now
	_result["jump_rising_edge"] = jump_rising_edge
	return _result


# The gait clips ship retimed to one length so the blend space holds them in
# phase; this puts each gait's authored cadence back, blended across the pair.
func _warp_cadence(grounded_now: bool) -> void:
	if not grounded_now:
		_tree.set("parameters/Locomotion/Grounded/cadence/scale", 1.0)
		return
	if _gait_clip_length <= 0.0:
		var anim: Animation = _tree.get_animation(StringName("loco/" + GAIT_CLIPS[0]))
		_gait_clip_length = anim.length
	var blend := clampf(_movement_blend, 0.0, float(GAIT_CLIPS.size() - 1))
	var low := int(floorf(blend))
	var high := mini(low + 1, GAIT_CLIPS.size() - 1)
	var wanted := lerpf(GAIT_CYCLE_LENGTHS[low], GAIT_CYCLE_LENGTHS[high], blend - float(low))
	_tree.set("parameters/Locomotion/Grounded/cadence/scale", _gait_clip_length / wanted)
