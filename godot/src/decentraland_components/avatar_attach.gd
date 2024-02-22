extends PlayerColliderFilter

@export var user_id: String = "":
	set(value):
		if user_id != value:
			_player_avatar_node = null
			user_id = value

#  AAPT_POSITION = 0;
#  AAPT_NAME_TAG = 1;
#  AAPT_LEFT_HAND = 2;
#  AAPT_RIGHT_HAND = 3;
var attach_point: int = -1

var _player_avatar_node: Avatar


func init():
	self.init_player_collider_filter()

	# Compute initial transform position
	_process(0)
	process_priority = 0


func _process(_delta):
	var p: Node3D = get_parent()
	if p == null:
		return

	if _player_avatar_node == null:
		look_up_player()
		if _player_avatar_node == null:
			return

	match attach_point:
		0:
			p.global_transform = _player_avatar_node.global_transform
		1:
			p.global_transform = _player_avatar_node.label_3d_name.global_transform
		2:
			p.global_transform = (
				_player_avatar_node.body_shape_skeleton_3d.global_transform
				* _player_avatar_node.left_hand_position
			)
		3:
			p.global_transform = (
				_player_avatar_node.body_shape_skeleton_3d.global_transform
				* _player_avatar_node.right_hand_position
			)
		_:
			p.transform = Transform3D.IDENTITY


func look_up_player():
	var primary_player_user_id := Global.player_identity.get_address_str()

	# default to current player
	var look_up_player_user_id := user_id if not user_id.is_empty() else primary_player_user_id
	if primary_player_user_id == look_up_player_user_id:
		_player_avatar_node = get_node("/root/explorer/world/Player/Avatar")
	else:
		_player_avatar_node = Global.avatars.get_avatar_by_address(user_id)
