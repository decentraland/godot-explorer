extends Node3D


# Called when the node enters the scene tree for the first time.
func _ready():
	var animation = Global.animation_importer.get_animation_from_gltf("angry")
	$AnimationPlayer.get_animation_library("").add_animation("angry", animation)
