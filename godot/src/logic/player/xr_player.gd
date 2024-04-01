class_name XRPlayer
extends XROrigin3D

@onready var camera: Camera3D = $XRCamera3D
@onready var avatar := $Avatar

func _ready():
	prints("Starts XRPlayer")


func _process(_dt):
	position.y = max(position.y, 0)
	avatar.hide() # Temp for tests
