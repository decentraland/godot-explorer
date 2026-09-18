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
var _ray_params := PhysicsRayQueryParameters3D.new()
# System 2: static contact sphere at the camera position (depenetration).
var _contact_sphere := SphereShape3D.new()
var _contact_params := PhysicsShapeQueryParameters3D.new()
var _contact_offset := Vector3.ZERO
# Current distance from the pivot along the camera segment; shortens instantly,
# extends smoothly so geometry doesn't pop through on the way out.
var _smoothed_dist := 0.0
# Diagnostics (enabled with DCL_CAM_DEBUG=1): classifies whatever the camera
# crosses unblocked — pointer-only collider, other layer, or no collider at
# all (single-sided shell / cmask=0).
var _debug_probe := false
var _debug_ray := PhysicsRayQueryParameters3D.new()
var _debug_seen := {}
# The spring resolves its length during its first physics steps (CameraArm
# starts at z=0). Skip those frames: clamping against the unresolved zero
# length would slam the camera to the pivot and slowly recover for no reason.
var _warmup_frames := 3

@onready var _mount: SpringArm3D = get_parent()
@onready var _camera_arm: Node3D = _mount.get_node("CameraArm")
@onready var _camera: Camera3D = _camera_arm.get_node("Camera3D")
# The player CharacterBody3D (Mount's parent). Its origin rests on the ground, so
# its Y is a scene-collider-free floor reference for the floor guard.
@onready var _player_body: Node3D = _mount.get_parent()


func _ready() -> void:
	_sphere.radius = CameraRig.CLAMP_SPHERE_RADIUS
	_params.shape = _sphere
	_params.collision_mask = CameraRig.CAMERA_COLLISION_MASK
	_params.collide_with_bodies = true
	_params.collide_with_areas = false
	_ray_params.collision_mask = CameraRig.CAMERA_COLLISION_MASK
	_ray_params.collide_with_bodies = true
	_ray_params.collide_with_areas = false
	# Inflates the swept shape so contact is reported with perpendicular
	# clearance — the near plane must never rest inside the wall.
	_params.margin = CameraRig.CLAMP_NEAR_CLEARANCE
	_contact_sphere.radius = CameraRig.CLAMP_CONTACT_RADIUS
	_contact_params.shape = _contact_sphere
	_contact_params.collision_mask = CameraRig.CAMERA_COLLISION_MASK
	_contact_params.collide_with_bodies = true
	_contact_params.collide_with_areas = false
	_contact_params.margin = 0.0
	# Start fully extended so there's no snap on the first frames.
	_smoothed_dist = CameraRig.THIRD_PERSON_CAMERA.z
	_debug_probe_setup()


func _process(delta: float) -> void:
	# Runs in _process (not _physics_process): physics steps complete first, so
	# CameraArm.position.z is fresh, and input-driven camera rotations applied
	# between physics ticks are caught on the same rendered frame — a fast mouse
	# flick can otherwise rotate the camera into a wall for a visible frame.
	if _warmup_frames > 0:
		_warmup_frames -= 1
		return
	var pivot := _mount.global_transform.origin
	var space := _mount.get_world_3d().direct_space_state

	# Analytic target: spring-resolved length behind the pivot + lateral offset.
	# (Computed, not read from the camera, because we overwrite the camera
	# position below and it must not feed back into the next frame.)
	var target := _mount.global_transform * Vector3(lateral_offset, 0.0, _camera_arm.position.z)
	var segment := target - pivot
	var dist := segment.length()
	if dist < 0.001:
		return
	var dir := segment / dist

	# cast_motion IGNORES geometry the shape already overlaps at the start
	# (verified empirically: it reports safe=1.0 = all clear). Two layers:
	#
	# 1) RAY WHISKERS from the pivot: three parallel rays along the segment
	#    (center ± sphere radius, horizontally). A point can never start
	#    overlapped (the pivot lives inside the player capsule), so the camera
	#    CENTER can never cross a collider in the ribbon — no matter the orbit,
	#    wall thickness, or shift alignment. The OUTER whiskers also catch
	#    walls BESIDE the center line — curved surfaces bulging into the camera
	#    (the live Genesis Plaza case: sphere blind from a hugged wall, single
	#    ray missing a bulge).
	var allowed := dist
	var perp := dir.cross(Vector3.UP)
	if perp.length_squared() < 0.01:
		perp = dir.cross(Vector3.RIGHT)
	perp = perp.normalized()
	for k in [-CameraRig.CLAMP_SPHERE_RADIUS, 0.0, CameraRig.CLAMP_SPHERE_RADIUS]:
		var from: Vector3 = pivot + perp * float(k)
		_ray_params.from = from
		_ray_params.to = from + segment
		var hit := space.intersect_ray(_ray_params)
		if not hit.is_empty():
			# Back off along the segment enough that the PERPENDICULAR clearance
			# to the wall equals CLAMP_NEAR_CLEARANCE (capped for grazing rays).
			var n: Vector3 = hit.get("normal", -dir)
			var cosang: float = clampf(absf(dir.dot(n)), 0.3, 1.0)
			var backoff: float = CameraRig.CLAMP_NEAR_CLEARANCE / cosang
			var hit_allowed: float = maxf(dir.dot(hit.position - pivot) - backoff, 0.0)
			allowed = minf(allowed, hit_allowed)

	# 2) SPHERE VOLUME from a point shifted along the mount's FORWARD (the
	#    player's front, always opposite to where the camera looks back from).
	#    Back against a wall, a sphere at the pivot would already penetrate it
	#    and the cast would go blind; shifted, it starts clear and detects it.
	#    But facing a THIN wall (e.g. a door box), the shifted origin can land
	#    INSIDE the wall — then the cast is blind again (the reported case:
	#    hugging a door and orbiting 360° put the camera on the far side). So
	#    the sphere is only trusted when its origin is NOT overlapped; the
	#    whiskers always apply regardless.
	var shift: float = minf(CameraRig.CLAMP_SPHERE_RADIUS, dist)
	var origin: Vector3 = pivot - _mount.global_transform.basis.z * shift
	var motion: Vector3 = target - origin

	_params.transform = Transform3D(Basis(), origin)
	_params.motion = motion
	if space.get_rest_info(_params).is_empty():
		var result := space.cast_motion(_params)
		# cast_motion always returns [safe, unsafe]; safe < 1.0 means blocked.
		if result.size() >= 2 and result[0] < 1.0:
			# Contact position of the sphere center, projected back onto the
			# camera segment so framing stays on the arm's line. May end up in
			# front of the pivot — correct when hugging a wall (the own-avatar
			# proximity fade hides the head; no distance floor).
			var contact_pos: Vector3 = (
				origin + motion * result[0] - motion.normalized() * CameraRig.CLAMP_EXTRA_MARGIN
			)
			allowed = minf(allowed, clampf(dir.dot(contact_pos - pivot), -shift, dist))

	if allowed < _smoothed_dist:
		_smoothed_dist = allowed  # shorten instantly — never clip
	else:
		_smoothed_dist = move_toward(_smoothed_dist, allowed, CameraRig.CLAMP_EXTEND_SPEED * delta)

	var base := pivot + dir * _smoothed_dist

	# SYSTEM 2 — CONTACT SPHERE: a static sphere at the camera position,
	# depenetrated from anything the line clamp could not see. Covers the edge
	# cases no longitudinal cast handles: walls PARALLEL to the segment
	# (pull-in can't help — clearance along them is constant), corners poking
	# into the frustum sides, and any residual poke while the sweep sphere is
	# blind.
	#
	# The resolution is computed STATELESSLY from `base` every frame: while the
	# contact persists, the depenetrated position is a deterministic fixed
	# point — no oscillation. (The previous stateful version started from
	# `base + offset` and decayed the offset the moment contact cleared, which
	# pulled the camera straight back INTO contact — the reported
	# frame-on/frame-off flicker.) The stored offset only smooths the RETURN
	# to the arm's line once the contact is truly gone, and snaps instantly
	# whenever a push is needed (never clip).
	var pos := base
	var pushed := false
	for i in range(3):
		_contact_params.transform = Transform3D(Basis(), pos)
		var rest := space.get_rest_info(_contact_params)
		if rest.is_empty():
			break
		var n: Vector3 = rest.get("normal", Vector3.UP)
		var depth: float = CameraRig.CLAMP_CONTACT_RADIUS - (pos - rest.get("point", pos)).dot(n)
		if depth <= 0.0:
			break
		pushed = true
		pos += n * (depth + CameraRig.CLAMP_EXTRA_MARGIN)
	var target_offset := pos - base if pushed else Vector3.ZERO
	if pushed and target_offset.length() >= _contact_offset.length():
		_contact_offset = target_offset  # growing push: instant — never clip
	else:
		_contact_offset = _contact_offset.move_toward(
			target_offset, CameraRig.CLAMP_EXTEND_SPEED * delta
		)

	_camera.global_position = base + _contact_offset

	# Floor guard: keep the camera above the player's own ground contact so it can't
	# dip below scene floors that have no usable collider (the sweep casts slip
	# through single-sided / cmask=0 meshes). Additive — only ever raises the camera.
	if is_instance_valid(_player_body):
		var floor_y: float = _player_body.global_position.y + CameraRig.FLOOR_CLEARANCE
		if _camera.global_position.y < floor_y:
			_camera.global_position.y = floor_y

	if _debug_probe:
		_debug_probe_pass(space, pivot, target, allowed, dist)


# --- Diagnostics (DCL_CAM_DEBUG=1) ---
# When nothing on the physics layer blocked the camera (allowed == dist),
# probe what ELSE is on the path so field reports of "camera goes through X"
# can be classified: pointer-only collider, no collider at all, or a
# single-sided (backface-less) trimesh the casts slip through from behind.


func _debug_probe_setup() -> void:
	_debug_probe = OS.get_environment("DCL_CAM_DEBUG") == "1"
	_debug_ray.collide_with_bodies = true
	_debug_ray.collide_with_areas = false


func _debug_probe_pass(
	space: PhysicsDirectSpaceState3D, pivot: Vector3, target: Vector3, allowed: float, dist: float
) -> void:
	if allowed < dist - 0.001:
		return  # physics collision handled this frame — nothing to diagnose
	_debug_ray.from = pivot
	_debug_ray.to = target
	_debug_ray.collision_mask = 1  # CL_POINTER
	var hit := space.intersect_ray(_debug_ray)
	if not hit.is_empty():
		_debug_report("POINTER-ONLY collider", hit)
		return
	_debug_ray.collision_mask = 3  # pointer | physics: anything at all?
	hit = space.intersect_ray(_debug_ray)
	if not hit.is_empty():
		_debug_report("collider on other layer(s)", hit)
		return
	_debug_report_key("NO-COLLIDER on camera path (single-sided shell or cmask=0)", "no-collider")


func _debug_report(kind: String, hit: Dictionary) -> void:
	var collider = hit.get("collider")
	var key := str(collider.get_instance_id()) if collider != null else "?"
	var layers: int = collider.collision_layer if collider != null else -1
	_debug_report_key("%s: %s layers=%d at %s" % [kind, collider, layers, hit.get("position")], key)


func _debug_report_key(msg: String, key: String) -> void:
	if _debug_seen.has(key):
		return
	_debug_seen[key] = true
	print("[CamDebug] ", msg)
