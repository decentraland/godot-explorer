extends Node


func _ready():
	start.call_deferred()


func start():
	self._start.call_deferred()


func _start():
	get_tree().change_scene_to_file("res://src/ui/explorer.tscn")
