extends Timer

# Tuning constants
const POSITION_THRESHOLD := 0.05  # 5 cm
const ROTATION_THRESHOLD := 0.01  # Approx ~0.57 degrees (in radians)

@export var player_node: Node3D

var last_position_sent: Vector3 = Vector3.ZERO
var last_rotation_sent: Quaternion = Quaternion()
var last_velocity_sent: Vector3 = Vector3.ZERO
var counter: int = 0


func _on_timeout():
	var position: Vector3 = player_node.get_broadcast_position()
	var rotation_y = player_node.get_broadcast_rotation_y()

	# 3. Build the quaternion
	var rotation: Quaternion = Quaternion.from_euler(Vector3(0, rotation_y, 0))
	
	# Calculate velocity based on position change
	var velocity: Vector3 = Vector3.ZERO
	if last_position_sent != Vector3.ZERO:
		velocity = (position - last_position_sent) / wait_time

	var position_changed: bool = position.distance_to(last_position_sent) >= POSITION_THRESHOLD
	var rotation_changed: bool = rotation.angle_to(last_rotation_sent) >= ROTATION_THRESHOLD

	# If neither changed significantly, delay broadcasting
	if not position_changed and not rotation_changed:
		counter += 1
		if counter < 10:
			return
	else:
		counter = 0  # Reset counter when movement/rotation occurs

	# Use the new broadcast_movement function with compression enabled
	Global.comms.broadcast_movement(position, rotation_y, velocity, false)
	last_position_sent = position
	last_rotation_sent = rotation
	last_velocity_sent = velocity
