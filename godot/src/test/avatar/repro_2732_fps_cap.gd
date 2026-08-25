extends Node


func _ready():
	Engine.max_fps = 30
	Engine.physics_ticks_per_second = 30
	print("[repro_2732] FPS cap forced to 30 — jump and check for landing bounce")
