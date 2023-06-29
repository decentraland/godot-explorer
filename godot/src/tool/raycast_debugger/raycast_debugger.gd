extends Node

var raycast_line_instancer := preload("res://src/tool/raycast_debugger/raycast_line.tscn")
var raycasts = {}


# TODO: add a tween to fade out the raycast line and remove it
func add_raycast(id: int, time: float, from: Vector3, to: Vector3) -> void:
	if raycasts.get(id) == null:
		var r = raycast_line_instancer.instantiate()
		r.set_name("raycast_debug_" + str(id))
		add_child(r)

		raycasts[id] = r

	var raycast: Node3D = raycasts[id]
	var diff = to - from
	raycast.set_global_position(from)
	raycast.set_scale(Vector3(1.0, 1.0, -diff.length()))
	raycast.look_at(to)
