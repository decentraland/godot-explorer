class_name CameraCollisionClamp
extends Node3D

## Secondary camera collision: sweeps a sphere from the SpringArm pivot to the
## ACTUAL camera position (lateral over-shoulder offset included) every frame
## and pulls the camera in along that segment when the sphere would hit
## solid geometry (issue #1814 review).
##
## Why this exists: the SpringArm3D only collision-checks a thin ray along its
## own axis. The camera sits 0.75 to the side (over-shoulder), so walls on that
## side were invisible to the arm. A sphere ON the arm big enough to cover the
## offset (r >= 0.75) would over-shorten near any geometry; sweeping a modest
## sphere to the real camera position is precise — it only shortens when the
## actual camera volume is blocked.
##
## Must run as a child of Mount (SpringArm3D), AFTER CameraArm: children
## process after the parent's internal physics step, so CameraArm.position.z
## already holds this frame's spring-resolved length.

const CameraRig := preload("res://src/logic/player/camera_rig_helpers.gd")

## Tweened by player.gd on camera-mode changes (0.75 third person, 0.0 first).
var lateral_offset := 0.0

var _sphere := SphereShape3D.new()
var _params := PhysicsShapeQueryParameters3D.new()
# Current distance from the pivot along the camera segment; shortens instantly,
# extends smoothly so geometry doesn't pop through on the way out.
var _smoothed_dist := 0.0
# The spring resolves its length during its first physics steps (CameraArm
# starts at z=0). Skip those frames: clamping against the unresolved zero
# length would slam the camera to the pivot and slowly recover for no reason.
var _warmup_frames := 3

@onready var _mount: SpringArm3D = get_parent()
@onready var _camera_arm: Node3D = _mount.get_node("CameraArm")
@onready var _camera: Camera3D = _camera_arm.get_node("Camera3D")


func _ready() -> void:
	_sphere.radius = CameraRig.CLAMP_SPHERE_RADIUS
	_params.shape = _sphere
	_params.collision_mask = CameraRig.CAMERA_COLLISION_MASK
	_params.collide_with_bodies = true
	_params.collide_with_areas = false
	# Start fully extended so there's no snap on the first frames.
	_smoothed_dist = CameraRig.THIRD_PERSON_CAMERA.z


func _process(delta: float) -> void:
	# Runs in _process (not _physics_process): physics steps complete first, so
	# CameraArm.position.z is fresh, and input-driven camera rotations applied
	# between physics ticks are caught on the same rendered frame — a fast mouse
	# flick can otherwise rotate the camera into a wall for a visible frame.
	if _warmup_frames > 0:
		_warmup_frames -= 1
		return
	var pivot := _mount.global_transform.origin
	# Analytic target: spring-resolved length behind the pivot + lateral offset.
	# (Computed, not read from the camera, because we overwrite the camera
	# position below and it must not feed back into the next frame.)
	var target := _mount.global_transform * Vector3(lateral_offset, 0.0, _camera_arm.position.z)
	var segment := target - pivot
	var dist := segment.length()
	if dist < 0.001:
		return
	var dir := segment / dist

	# CRITICAL: cast_motion IGNORES geometry the shape already overlaps at the
	# start (verified empirically: it reports safe=1.0 = all clear). With the
	# player's back against a wall the pivot sits 0.25m from it and an r=0.4
	# sphere at the pivot already penetrates — the cast went blind and the
	# camera sailed through the wall. So the sphere is cast from a point
	# shifted along the mount's FORWARD (toward the player's front, always
	# opposite to where the camera looks back from): it starts clear of the
	# wall behind and the wall is detected. When facing a wall the shifted
	# origin may start inside it, but the motion then moves away from it, so
	# the ignored overlap is harmless (that wall is not between origin and
	# camera). No detection of the wall direction needed — the forward shift
	# is self-aligning.
	var shift: float = minf(CameraRig.CLAMP_SPHERE_RADIUS, dist)
	var origin: Vector3 = pivot - _mount.global_transform.basis.z * shift
	var motion: Vector3 = target - origin

	var allowed := dist
	_params.transform = Transform3D(Basis(), origin)
	_params.motion = motion
	var result := _mount.get_world_3d().direct_space_state.cast_motion(_params)
	# cast_motion always returns [safe, unsafe]; safe < 1.0 means blocked.
	if result.size() >= 2 and result[0] < 1.0:
		# Contact position of the sphere center, projected back onto the camera
		# segment so framing stays on the arm's line. May end up in front of
		# the pivot — correct when hugging a wall (the own-avatar proximity
		# fade hides the head the camera is forced into; no distance floor).
		var contact_pos: Vector3 = (
			origin + motion * result[0] - motion.normalized() * CameraRig.CLAMP_EXTRA_MARGIN
		)
		allowed = clampf(dir.dot(contact_pos - pivot), -shift, dist)

	if allowed < _smoothed_dist:
		_smoothed_dist = allowed  # shorten instantly — never clip
	else:
		_smoothed_dist = move_toward(_smoothed_dist, allowed, CameraRig.CLAMP_EXTEND_SPEED * delta)

	_camera.global_position = pivot + dir * _smoothed_dist
