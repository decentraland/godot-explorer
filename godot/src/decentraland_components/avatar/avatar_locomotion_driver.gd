class_name AvatarLocomotionDriver
extends RefCounted
## Writes the avatar AnimationTree locomotion conditions from the packed
## DclAvatar.get_anim_bits() value, only on the frames where an input changed.

const WALK := 1
const JOG := 2
const RUN := 4
const RISE := 8
const FALL := 16
const LAND := 32
const GROUNDED := 64
const LOCOMOTION_MASK := 127
const MOVING := WALK | JOG | RUN

const P_IDLE := &"parameters/Locomotion/conditions/idle"
const P_EMOTE := &"parameters/Locomotion/conditions/emote"
const P_NEMOTE := &"parameters/Locomotion/conditions/nemote"
const P_EMIX := &"parameters/Locomotion/conditions/emix"
const P_NEMIX := &"parameters/Locomotion/conditions/nemix"
const P_RUN := &"parameters/Locomotion/conditions/run"
const P_JOG := &"parameters/Locomotion/conditions/jog"
const P_WALK := &"parameters/Locomotion/conditions/walk"
const P_RISE := &"parameters/Locomotion/conditions/rise"
const P_FALL := &"parameters/Locomotion/conditions/fall"
const P_LAND := &"parameters/Locomotion/conditions/land"
const P_NFALL := &"parameters/Locomotion/conditions/nfall"
const P_DOUBLE_JUMP := &"parameters/Locomotion/conditions/double_jump"
const P_GLIDING := &"parameters/Locomotion/conditions/gliding"
const P_NGLIDING := &"parameters/Locomotion/conditions/ngliding"
const P_GLIDE_BLEND := &"parameters/Locomotion/Gliding_Idle/Blend2/blend_amount"

var _last_key: int = -1
var _last_jump_count: int = 0
# #b2: a remote first seen mid-double-jump must not play the SFX from nothing.
var _jump_count_sync_pending: bool = true
var _glide_forward_blend: float = 0.0
var _written_glide_blend: float = -1.0


static func glide_state_of(bits: int) -> int:
	return (bits >> 16) & 3


func tick(avatar: Node, bits: int, delta: float) -> void:
	var grounded: bool = (bits & GROUNDED) != 0
	# #b18: `is_grounded` guard suppresses the all-false condition window at the
	# jump apex (rise/fall ±0.3 deadband) so Idle doesn't leak in mid-air.
	var idle: bool = grounded and (bits & (MOVING | RISE | FALL)) == 0
	var emote_controller = avatar.emote_controller
	emote_controller.process(idle)

	# Masked (upper-body) emotes keep playing while moving, so idle can't gate
	# them — otherwise Pulse would broadcast EmoteStop for a walking emote.
	if avatar.is_local_player:
		Global.comms.set_emoting(
			emote_controller.is_playing() and (idle or emote_controller.playing_masked)
		)

	var jump_count: int = (bits >> 8) & 255
	var jump_rising_edge: bool = jump_count > _last_jump_count and jump_count >= 2
	if _jump_count_sync_pending:
		jump_rising_edge = false
		_jump_count_sync_pending = false
	_last_jump_count = jump_count
	var glide_state: int = glide_state_of(bits)
	var gliding_now: bool = glide_state == 1 or glide_state == 2
	var single: bool = emote_controller.playing_single
	var mixed: bool = emote_controller.playing_mixed

	var tree: AnimationTree = avatar.animation_tree
	var key: int = (
		(bits & LOCOMOTION_MASK)
		| (int(single) << 8)
		| (int(mixed) << 9)
		| (int(jump_rising_edge) << 10)
		| (int(gliding_now) << 11)
	)
	if key != _last_key:
		_last_key = key
		tree.set(P_IDLE, idle)
		tree.set(P_EMOTE, single)
		tree.set(P_NEMOTE, not single)
		tree.set(P_EMIX, mixed)
		tree.set(P_NEMIX, not mixed)
		tree.set(P_RUN, (bits & RUN) != 0 and grounded)
		tree.set(P_JOG, (bits & JOG) != 0 and grounded)
		tree.set(P_WALK, (bits & WALK) != 0 and grounded)
		tree.set(P_RISE, (bits & RISE) != 0)
		tree.set(P_FALL, (bits & FALL) != 0)
		tree.set(P_LAND, (bits & LAND) != 0)
		# #b3: nfall reads is_grounded, not the `land` pulse.
		tree.set(P_NFALL, grounded)
		tree.set(P_DOUBLE_JUMP, jump_rising_edge)
		tree.set(P_GLIDING, gliding_now)
		tree.set(P_NGLIDING, not gliding_now)

	var glide_target: float = 1.0 if (bits & MOVING) != 0 else 0.0
	_glide_forward_blend = move_toward(_glide_forward_blend, glide_target, delta * 4.0)
	if _glide_forward_blend != _written_glide_blend:
		_written_glide_blend = _glide_forward_blend
		tree.set(P_GLIDE_BLEND, _glide_forward_blend)

	if jump_rising_edge:
		avatar.audio_player_double_jump.play()
