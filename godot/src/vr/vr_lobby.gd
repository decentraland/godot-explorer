extends Node3D

@onready var ui_root = %Viewport2Din3D

# Called when the node enters the scene tree for the first time.
func _ready():
	prints("Vr Lobby")
	ui_root.scene_node.change_scene.connect(self.change_scene)


func change_scene(new_scene: String):
	ui_root.set_scene(load(new_scene))
