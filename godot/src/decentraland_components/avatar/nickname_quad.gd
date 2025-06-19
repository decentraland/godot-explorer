@tool
extends Sprite3D

@export_range(0,20) var fade_end = 7.0
@export_range(0,20) var fade_start = 6.0

@onready var camera_3d = get_viewport().get_camera_3d()

func _ready():
	visibility_range_end = fade_end

func _process(_delta:float)->void:
	# Poll for camera_3d existance
	if !camera_3d: camera_3d = get_viewport().get_camera_3d() 
	if !camera_3d: return 
	var dist : float = camera_3d.global_transform.origin.distance_squared_to(global_transform.origin)
	modulate = Color(1.0,1.0,1.0,smoothstep(fade_end*fade_end, fade_start*fade_start, dist))
