extends Node

var raycast_debugger = load("res://src/tool/raycast_debugger/raycast_debugger.gd").new()


func _ready():
	add_child(raycast_debugger)


func add_raycast(id: int, time: float, from: Vector3, to: Vector3) -> void:
	raycast_debugger.add_raycast(id, time, from, to)


var config_dictionary: Dictionary = {
	"gravity": 55.0,
	"jump_velocity": 12.0,
	"walk_velocity": 12.0,
	"run_velocity": 20.0,
	"process_tick_quota": 1,
	"scene_radius": 1,
}

signal config_changed


func _load():
	pass


func _save():
	emit_signal("config_changed")
	pass


func _default():
	pass


func get_resolution():
	return (
		config_dictionary
		. get(
			"resolution",
		)
	)


func get_gravity():
	return config_dictionary.get("gravity", 55.0)


func get_jump_velocity():
	return config_dictionary.get("jump_velocity", 12.0)


func get_walk_velocity():
	return config_dictionary.get("walk_velocity", 12.0)


func get_run_velocity():
	return config_dictionary.get("run_velocity", 20.0)


func get_process_tick_quota():
	return config_dictionary.get("process_tick_quota", 1)


func get_scene_radius():
	return config_dictionary.get("scene_radius", 1)


func get_tls_client():
	return TLSOptions.client_unsafe()
