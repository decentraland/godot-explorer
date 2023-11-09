# It emits a signal when the scene gets active

extends Node3D

signal on_scene_active(active: bool)

var _my_scene_id: int = 0


func _search_scene_node(target: Node3D) -> DclSceneNode:
	if target is DclSceneNode:
		return target

	var parent_node_3d = target.get_parent_node_3d()
	if parent_node_3d == null:
		return null

	return _search_scene_node(parent_node_3d)


func _ready():
	var scene_node: DclSceneNode = _search_scene_node(self)
	if scene_node.is_global():
		self.queue_free()
		return

	_my_scene_id = scene_node.get_scene_id()
	Global.scene_runner.on_change_scene_id.connect(self._on_change_scene_id)


func _on_change_scene_id(scene_id: int):
	on_scene_active.emit(scene_id == _my_scene_id)
