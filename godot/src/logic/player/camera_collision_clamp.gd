class_name CameraCollisionClamp
extends Node3D

## Secondary camera collision: sweeps a sphere from the SpringArm pivot to the
## ACTUAL camera position (lateral over-shoulder offset included) every physics
## frame and pulls the camera in along that segment when the sphere would hit
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


func _physics_process(delta: float) -> void:
	var pivot := _mount.global_transform.origin
	# Analytic target: spring-resolved length behind the pivot + lateral offset.
	# (Computed, not read from the camera, because we overwrite the camera
	# position below and it must not feed back into the next frame.)
	var target := _mount.global_transform * Vector3(lateral_offset, 0.0, _camera_arm.position.z)
	var segment := target - pivot
	var dist := segment.length()
	if dist < 0.001:
		return

	var allowed := dist
	_params.transform = Transform3D(Basis(), pivot)
	_params.motion = segment
	var result := _mount.get_world_3d().direct_space_state.cast_motion(_params)
	if not result.is_empty():
		var hit_dist: float = dist * result[0] - CameraRig.CLAMP_EXTRA_MARGIN
		# Floor at MIN_DISTANCE (capped by dist) so a fully blocked cast still
		# keeps the camera out of the avatar's head; the proximity fade cleans
		# up any residual poke. In first person (dist ~0.2 < MIN) this floor
		# effectively disables the clamp — intended.
		allowed = clampf(maxf(hit_dist, CameraRig.CLAMP_MIN_DISTANCE), 0.0, dist)

	if allowed < _smoothed_dist:
		_smoothed_dist = allowed  # shorten instantly — never clip
	else:
		_smoothed_dist = move_toward(_smoothed_dist, allowed, CameraRig.CLAMP_EXTEND_SPEED * delta)

	_camera.global_position = pivot + segment.normalized() * _smoothed_dist
