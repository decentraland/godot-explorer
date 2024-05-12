class_name RaycastDebugger
extends Node

var raycast_line_instancer := preload("res://src/tool/raycast_debugger/raycast_line.tscn")
var raycasts = {}


func _process(delta):
	for id in raycasts.keys():
		var mesh: MeshInstance3D = raycasts[id].get_node("MeshInstance3D")
		var material: BaseMaterial3D = mesh.get_active_material(0)
		material.albedo_color.a -= delta * 0.01
		if material.albedo_color.a <= 0:
			material.albedo_color.a = 0
			remove_raycast.call_deferred(id)


func remove_raycast(id):
	var raycast = raycasts[id]
	remove_child(raycast)
	raycast.queue_free()
	raycasts.erase(id)


func add_raycast(id: int, time: float, from: Vector3, to: Vector3) -> void:
	if raycasts.get(id) == null:
		var r = raycast_line_instancer.instantiate()
		r.set_name("raycast_debug_" + str(id))
		add_child(r)
		raycasts[id] = r

	var mesh: MeshInstance3D = raycasts[id].get_node("MeshInstance3D")
	var material: BaseMaterial3D = mesh.get_active_material(0)
	material.albedo_color.a = time
	var raycast: Node3D = raycasts[id]
	var diff = to - from
	raycast.set_global_position(from)
	raycast.set_scale(Vector3(1.0, 1.0, -diff.length()))
	if diff.normalized().abs() == Vector3.UP:
		raycast.set_rotation(Vector3(PI / 2 * sign(diff.y), 0.0, 0.0))
	else:
		raycast.look_at(to)
