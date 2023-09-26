extends Node

var _player_node: Node3D


#  AAPT_POSITION = 0;
#  AAPT_NAME_TAG = 1;
#  AAPT_LEFT_HAND = 2;
#  AAPT_RIGHT_HAND = 3;
var attach_point: int = -1

@export var user_id : String = "" :
	set(value):
		if user_id != value:
			_player_node = null
			user_id = value


func _ready():
	process_priority = 1

func _process(delta):
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
			p.global_transform = _player_node.global_transform * _player_node.left_hand_position
		3:
			var pg = _player_node.global_transform 
			var rg = _player_node.right_hand_position
			p.global_transform = pg * rg
		_:
			p.transform = Transform3D.IDENTITY

func look_up_player():
	# default to current player
	if user_id.is_empty():
		# TODO: change this
		_player_node = get_node("/root/explorer/Player/Avatar")
	else:
		_player_node = Global.avatars.get_avatar_by_address(user_id)
		
		
