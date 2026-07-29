class_name CameraRigHelpers
extends RefCounted

# Pure, Global-free camera-rig config extracted from player.gd so the
# see-through-walls fix (issue #1814) is unit-testable headless without the full
# player scene or the Rust extension (see test/player/test_player_camera_collision.gd).

# Solid world geometry lives on CL_PHYSICS (layer 2) — the same mask the player
# CharacterBody3D collides with (player.tscn). The camera SpringArm3D ("Mount")
# uses this so it shortens against exactly the walls the player can't walk
# through, and ignores pointer-only / non-solid colliders. Mirrors the
# collision_mask set on Mount in player.tscn; the unit test pins the value.
const CAMERA_COLLISION_MASK := 2

# Over-shoulder framing: x = lateral camera offset, z = third-person back distance.
const THIRD_PERSON_CAMERA := Vector3(0.75, 0, 3)
# First person sits just in front of the pivot (inside the head).
const FIRST_PERSON_SPRING_LENGTH := -0.2

# CameraCollisionClamp (secondary volumetric sweep to the ACTUAL offset camera
# position — the SpringArm alone only ray-casts its own axis).
# Radius doubles as the standoff distance from walls. Not larger than ~0.4:
# bigger spheres touch both frames of ~1m doorways and glue the camera to the
# avatar's head in every doorway.
const CLAMP_SPHERE_RADIUS := 0.4
# Extra pull-in beyond the sphere contact point.
const CLAMP_EXTRA_MARGIN := 0.02
# Perpendicular clearance the camera must keep from wall faces. The near plane
# is 0.05 — anything less and it pokes into the wall even when the clamp
# "worked" (the reported "camera rests right at the edge" clip, worst on
# angled/curved trimesh where a margin along the segment shrinks to ~0
# perpendicularly). Applied as the sphere query margin and as the ray's
# normal-based backoff.
const CLAMP_NEAR_CLEARANCE := 0.08
# Extension recovery speed (m/s). Shortening is instant (never clip), extending
# is smoothed so geometry doesn't pop through on the way out.
const CLAMP_EXTEND_SPEED := 8.0


# Spring-arm length + camera-local X offset per camera mode.
#
# The X offset is a CAMERA offset, applied on the Camera3D BELOW the spring arm,
# never on the arm's pivot. That keeps the pivot centered on the player capsule
# so the collision cast starts from a protected point — not the 0.75-lateral
# point outside the capsule that used to let the camera clip through walls.
#
# spring_length uses THIRD_PERSON_CAMERA.z (the pure back distance), not the
# diagonal .length(): the lateral component is handled by the camera offset, so
# baking it into the arm length would push the camera too far back.
static func rig_targets(third_person: bool) -> Dictionary:
	if third_person:
		return {
			"spring_length": THIRD_PERSON_CAMERA.z,
			"camera_offset_x": THIRD_PERSON_CAMERA.x,
		}
	return {
		"spring_length": FIRST_PERSON_SPRING_LENGTH,
		"camera_offset_x": 0.0,
	}
