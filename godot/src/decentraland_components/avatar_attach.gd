extends Node

# in seconds
const TIME_TO_RETURN_BACK_LAYER = 0.5

@export var user_id: String = "":
	set(value):
		if user_id != value:
			_player_node = null
			user_id = value

#  AAPT_POSITION = 0;
#  AAPT_NAME_TAG = 1;
#  AAPT_LEFT_HAND = 2;
#  AAPT_RIGHT_HAND = 3;
var attach_point: int = -1

var _computed_colliders: Array[Node]
var _main_scene_tree: SceneTree
var _player_node: Node3D


func init():
	var scene = SceneHelper.search_scene_node(self.get_parent())
	scene.tree_changed.connect(self._on_tree_changed)

	# Compute initial transform position
	_process(0)
	process_priority = 0

	_main_scene_tree = get_tree()

	var colliders: Array[Node] = []
	get_collider_tree(self.get_parent(), colliders)
	update_colliders(colliders)


func get_collider_tree(node: Node, colliders: Array[Node]) -> void:
	for child in node.get_children():
		get_collider_tree(child, colliders)
		if child.has_meta("dcl_col"):
			colliders.push_back(child)


func update_colliders(current_colliders: Array[Node]) -> void:
	for collider in current_colliders:
		var col_index := _computed_colliders.find(collider)
		if col_index != -1:
			_computed_colliders.remove_at(col_index)
		else:
			collider.collision_layer = 0

	async_apply_after_physics(_computed_colliders.duplicate())

	_computed_colliders = current_colliders


func async_apply_after_physics(check_computed_colliders: Array[Node]):
	await _main_scene_tree.create_timer(TIME_TO_RETURN_BACK_LAYER).timeout

	for old_collider in check_computed_colliders:
		# Corner case when is moved from one avatar attach to other
		if not is_parented_with_avatar_attach(old_collider):
			old_collider.collision_layer = old_collider.get_meta("dcl_col")


func is_parented_with_avatar_attach(_collider: Node) -> bool:
	# TODO
	return false


func _on_tree_changed():
	var colliders: Array[Node] = []
	get_collider_tree(self.get_parent(), colliders)
	update_colliders(colliders)


func _exit_tree():
	update_colliders([])


func _process(_delta):
	var p: Node3D = get_parent()
	if p == null:
		return

	if _player_node == null:
		look_up_player()
		if _player_node == null:
			return

	match attach_point:
		0:
			p.global_transform = _player_node.global_transform
		1:
			p.global_transform = _player_node.label_3d_name.global_transform
		2:
			p.global_transform = (
				_player_node.body_shape_skeleton_3d.global_transform
				* _player_node.left_hand_position
			)
		3:
			p.global_transform = (
				_player_node.body_shape_skeleton_3d.global_transform
				* _player_node.right_hand_position
			)
		_:
			p.transform = Transform3D.IDENTITY


func look_up_player():
	var primary_player_user_id := Global.player_identity.get_address_str()

	# default to current player
	var look_up_player_user_id := user_id if not user_id.is_empty() else primary_player_user_id
	if primary_player_user_id == look_up_player_user_id:
		_player_node = get_node("/root/explorer/world/Player/Avatar")
	else:
		_player_node = Global.avatars.get_avatar_by_address(user_id)
