extends Node3D

@onready var avatar = $Avatar

var _elapsed: float = 0.0
const UPDATE_HZ: float = 10.0


func _ready():
	avatar.skip_process = false
	avatar.movement_type = 1  # LerpTwoPoints
	Engine.max_fps = 30
	Engine.physics_ticks_per_second = 30
	print("[repro_2734] remote avatar interpolation test started")


func _process(delta: float) -> void:
	_elapsed += delta
	# Simulate ~10 Hz network updates
	if fmod(_elapsed, 1.0 / UPDATE_HZ) < delta:
		var angle = _elapsed * 1.5
		var pos = Vector3(sin(angle) * 2.0, 0.0, cos(angle) * 2.0)
		var rot = Vector3(0.0, angle + PI, 0.0)
		var t = Transform3D(Basis.from_euler(rot), pos)
		avatar.set_target_position(t)
