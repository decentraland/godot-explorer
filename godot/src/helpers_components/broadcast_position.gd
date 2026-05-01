extends Timer

# Tuning constants
const POSITION_THRESHOLD := 0.05  # 5 cm
const ROTATION_THRESHOLD := 0.01  # Approx ~0.57 degrees (in radians)

@export var player_node: Node3D

var last_position_sent: Vector3 = Vector3.ZERO
var last_rotation_sent: float = INF
var counter: int = 0

# Zero-tolerance force-send tracking: any jump_count / is_grounded / glide_state
# change emits on the next timer tick regardless of position delta.
var last_jump_count_sent: int = 0
var last_is_grounded_sent: bool = true
var last_glide_state_sent: int = 0


func _on_timeout():
	if not player_node or not player_node.avatar:
		printerr("No player node or player_node.avatar")
		return
	var position: Vector3 = player_node.get_broadcast_position()
	var rotation_y = player_node.get_broadcast_rotation_y()
	var avatar = player_node.avatar

	var position_changed: bool = position.distance_to(last_position_sent) >= POSITION_THRESHOLD
	var rotation_changed: bool = (
		absf(wrapf(rotation_y - last_rotation_sent, -PI, PI)) >= ROTATION_THRESHOLD
	)
	var anim_state_changed: bool = (
		avatar.jump_count != last_jump_count_sent
		or avatar.is_grounded != last_is_grounded_sent
		or avatar.glide_state != last_glide_state_sent
	)

	if not position_changed and not rotation_changed and not anim_state_changed:
		counter += 1
		if counter < 10:
			return

	counter = 0

	(
		Global
		. comms
		. broadcast_movement(
			false,
			position,
			rotation_y,
			player_node.velocity,
			avatar.walk,
			avatar.run,
			avatar.jog,
			avatar.rise,
			avatar.fall,
			avatar.land,
			avatar.jump_count,
			avatar.glide_state,
			avatar.is_grounded,
		)
	)

	last_position_sent = position
	last_rotation_sent = rotation_y
	last_jump_count_sent = avatar.jump_count
	last_is_grounded_sent = avatar.is_grounded
	last_glide_state_sent = avatar.glide_state
