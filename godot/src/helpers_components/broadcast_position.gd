extends Timer

@export var follow_node: Node3D

var last_position_sent: Vector3 = Vector3.ZERO
var counter: int = 0

func _on_timeout():
	var transform: Transform3D = follow_node.global_transform
	var position = transform.origin
	var rotation = transform.basis.get_rotation_quaternion()

	if last_position_sent.is_equal_approx(position):
		counter += 1
		if counter < 10:
			return

	Global.comms.broadcast_position_and_rotation(position, rotation)
	last_position_sent = position
	counter = 0
