class_name PlayerColliderFilter
extends Node

# This node can be added when a every child which has a collider needs to be disabled
#	> Functionality needed when entities are parented to the camera or player
#	> It only disable the PHYSICS layer, so it doesn't miss the raycast functionality

# in seconds
const TIME_TO_RETURN_BACK_LAYER = 0.5

var _computed_colliders: Array[Node]
var _main_scene_tree: SceneTree
var _player_node: Node3D


# This function MUST be called manually
func init_player_collider_filter():
	var scene = SceneHelper.search_scene_node(self.get_parent())
	scene.tree_changed.connect(self._on_tree_changed)

	_main_scene_tree = get_tree()

	var colliders: Array[Node] = []
	_get_collider_tree(self.get_parent(), colliders)
	_update_colliders(colliders)


func _get_collider_tree(node: Node, colliders: Array[Node]) -> void:
	for child in node.get_children():
		_get_collider_tree(child, colliders)
		if child.has_meta("dcl_col"):
			colliders.push_back(child)


func _update_colliders(current_colliders: Array[Node]) -> void:
	for collider in current_colliders:
		var col_index := _computed_colliders.find(collider)
		if col_index != -1:
			_computed_colliders.remove_at(col_index)
		else:
			collider.collision_layer = collider.collision_layer & 0xFFFFFFFD

	_async_apply_after_physics(_computed_colliders.duplicate())

	_computed_colliders = current_colliders


func _async_apply_after_physics(check_computed_colliders: Array[Node]):
	await _main_scene_tree.create_timer(TIME_TO_RETURN_BACK_LAYER).timeout

	for old_collider in check_computed_colliders:
		if not _is_filter_still_needed(old_collider):
			old_collider.collision_layer = old_collider.get_meta("dcl_col")


# Corner case when is moved from one collider filtered to another
#	> From avatar attach to another avatar attached
#	> From PlayerEntity/Camera entity, to another
#	> between avatar attach and player_entity/camera
func _is_filter_still_needed(_collider: Node) -> bool:
	# TODO
	return false


func _on_tree_changed():
	var colliders: Array[Node] = []
	_get_collider_tree(self.get_parent(), colliders)
	_update_colliders(colliders)


func _exit_tree():
	_update_colliders([])
