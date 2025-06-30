extends Timer

# Tuning constants
const POSITION_THRESHOLD := 0.05  # 5 cm
const ROTATION_THRESHOLD := 0.01  # Approx ~0.57 degrees (in radians)

@export var player_node: Node3D

var last_position_sent: Vector3 = Vector3.ZERO
var last_rotation_sent: Quaternion = Quaternion()
var counter: int = 0


func _on_timeout():
	var position: Vector3 = player_node.get_broadcast_position()
	var rotation_y = player_node.get_broadcast_rotation_y()

	# 3. Build the quaternion
	var rotation: Quaternion = Quaternion.from_euler(Vector3(0, rotation_y, 0))

	var position_changed: bool = position.distance_to(last_position_sent) >= POSITION_THRESHOLD
	var rotation_changed: bool = rotation.angle_to(last_rotation_sent) >= ROTATION_THRESHOLD

	# If neither changed significantly, delay broadcasting
	if not position_changed and not rotation_changed:
		counter += 1
		if counter < 10:
			return

	counter = 0 # Reset counter when movement/rotation occurs

	# Use the new broadcast_movement function with compression enabled
	var avatar = player_node.avatar
	Global.comms.broadcast_movement(false, position, rotation_y, player_node.velocity, avatar.walk, avatar.run, avatar.jog, avatar.rise, avatar.fall, avatar.land)
	last_position_sent = position
	last_rotation_sent = rotation
