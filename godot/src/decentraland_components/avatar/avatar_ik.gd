class_name AvatarIK
extends RefCounted

# Feet, hands and head IK wiring: Unity's per-rig gates and the frame calls.
# Visual only — bone modifiers, never a physics query. A remote avatar gets the
# head alone, since Unity's hands and feet need the sender's rigid transform,
# which a remote entity does not carry. Head sync leaves from here too: Unity's
# HeadSync sends yaw and pitch, not a look-at vector.

# Unity HeadIKRemotePlayersDistance: a remote head turn is dropped past this,
# so a crowd costs nothing at distance.
const REMOTE_DISTANCE := 10.0

var _avatar: Avatar
var _feet: AvatarFeetIK
var _hands: AvatarHandsIK
var _head: AvatarHeadIK


func _init(avatar: Avatar) -> void:
	_avatar = avatar
	_feet = AvatarFeetIK.new(avatar, avatar.body_shape_skeleton_3d)
	_hands = AvatarHandsIK.new(avatar, avatar.body_shape_skeleton_3d)
	_head = AvatarHeadIK.new(avatar, avatar.body_shape_skeleton_3d)


# `rig_free` is the avatar's own gate (full LOD, no spawn reveal, loaded world,
# no emote): everything the IK needs to know about the rest of the avatar.
func update(
	delta: float, rig_free: bool, grounded: bool, stunned: bool, movement_speed: float
) -> void:
	if _avatar.is_local_player:
		# Unity gates feet and hands on the same rig state.
		_update_local(delta, rig_free and grounded and not stunned, movement_speed)
		return
	# A remote avatar's head comes over the wire, never from our camera, and it
	# is told even when the turn is off or the bones freeze at the last
	# influence.
	var head_on: bool = (
		(_avatar.head_ik_yaw_enabled or _avatar.head_ik_pitch_enabled)
		and rig_free
		and not stunned
		and _avatar._lod_distance <= REMOTE_DISTANCE
	)
	var body := AvatarHeadIK.wire_to_body(
		_avatar.head_ik_yaw, _avatar.head_ik_pitch, _avatar.global_transform.basis
	)
	_head.update_remote(delta, head_on, body.x, body.y)


func _update_local(delta: float, enabled: bool, movement_speed: float) -> void:
	_feet.update_ik(delta, enabled)
	_hands.update_ik(delta, enabled)

	# Head IK looks where the view camera looks, and only while nearly standing
	# still (Unity disables it above ~0.7 m/s of movement).
	var camera := _avatar.get_viewport().get_camera_3d()
	var look_dir := Vector3.ZERO if camera == null else -camera.global_transform.basis.z
	var head_on: bool = enabled and movement_speed < AvatarHeadIK.MAX_SPEED and camera != null
	_head.update_ik(delta, head_on, look_dir)
	# Mirrored onto the avatar so the sender's dirty check can see the turn while
	# it stands still.
	_avatar.head_ik_yaw = _head.head_yaw_deg
	_avatar.head_ik_pitch = _head.head_pitch_deg
	_avatar.head_ik_yaw_enabled = head_on
	_avatar.head_ik_pitch_enabled = head_on
	var wire := AvatarHeadIK.body_to_wire(
		_head.head_yaw_deg, _head.head_pitch_deg, _avatar.global_transform.basis
	)
	Global.comms.set_head_ik(head_on, wire.x, wire.y)
