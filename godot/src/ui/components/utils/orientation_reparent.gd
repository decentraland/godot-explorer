## Reparents the owner of this node based on orientation.
## Add as a child of the node to be reparented, then set portrait_parent and landscape_parent.
class_name OrientationReparent
extends Node

@export var portrait_parent: Node
@export var landscape_parent: Node

var _watcher: OrientationWatcher


func _ready() -> void:
	_watcher = OrientationWatcher.new()
	_watcher.orientation_changed.connect(_on_orientation_changed)
	add_child(_watcher)


func _on_orientation_changed(is_portrait: bool) -> void:
	var target: Node = portrait_parent if is_portrait else landscape_parent
	if target == null or get_parent() == target:
		return
	get_parent().reparent(target, false)
