extends Control
const SCENE_3 = preload("res://src/ui/components/side-bar/scene3.tscn")

var parent_reference: Node


func _ready():
	pass


func set_parent(node: Node):
	parent_reference = node


func _on_button_pressed():
	parent_reference.push(SCENE_3)
