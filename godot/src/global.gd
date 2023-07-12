extends Node

var raycast_debugger = load("res://src/tool/raycast_debugger/raycast_debugger.gd").new()


func _ready():
	add_child(raycast_debugger)


func add_raycast(id: int, time: float, from: Vector3, to: Vector3) -> void:
	raycast_debugger.add_raycast(id, time, from, to)

func get_tls_client():
	return TLSOptions.client_unsafe()
	
