extends Node

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

var _player_node: Node3D


func _ready():
	process_priority = 0


func _set_children_physics(node: Node, enable: bool):
	for child in node.get_children():
		_set_children_physics(child, enable)

		if child.has_meta("dcl_col"):
			if enable:
				child.collision_layer = child.get_meta("dcl_col")
			else:
				child.collision_layer = 0


func _exit_tree():
	_set_children_physics(self.get_parent(), true)


func _enter_tree():
	_set_children_physics(self.get_parent(), false)


func _process(_delta):
	var p: Node3D = get_parent()
	if p == null:
		return

	if _player_node == null:
		look_up_player()
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
	# default to current player
	if user_id.is_empty():
		# TODO: change this
		_player_node = get_node("/root/explorer/Player/Avatar")
	else:
		_player_node = Global.avatars.get_avatar_by_address(user_id)
