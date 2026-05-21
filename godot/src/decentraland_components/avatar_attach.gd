extends PlayerColliderFilter

@export var user_id: String = "":
	set(value):
		if user_id != value:
			_player_avatar_node = null
			user_id = value

# See AvatarAnchorPointType in avatar_attach.proto.
var attach_point: int = -1

var _player_avatar_node: Avatar

# Scene-authored local scale snapshot. transform_and_parent.rs (Rust) sets
# p.scale whenever the SDK Transform is updated; we adopt that value whenever
# it diverges from what we last wrote, otherwise we keep our snapshot so the
# global_transform override below doesn't clobber the scene's scale.
var _scene_scale: Vector3 = Vector3.ONE
var _last_written_scale: Vector3 = Vector3.INF


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

	# If p.scale differs from what we wrote last frame, the scene's Transform
	# handler must have updated it — capture the new scene scale.
	if not p.scale.is_equal_approx(_last_written_scale):
		_scene_scale = p.scale

	p.global_transform = _player_avatar_node.get_anchor_point_global_transform(attach_point)
	p.scale = _scene_scale
	_last_written_scale = _scene_scale


func look_up_player():
	var primary_player_user_id := Global.player_identity.get_address_str()

	# default to current player
	var look_up_player_user_id := user_id if not user_id.is_empty() else primary_player_user_id
	if primary_player_user_id == look_up_player_user_id:
		_player_avatar_node = get_node("/root/explorer/world/Player/Avatar")
	else:
		_player_avatar_node = Global.avatars.get_avatar_by_address(user_id)
		if _player_avatar_node != null:
			_player_avatar_node.activate_attach_points()
