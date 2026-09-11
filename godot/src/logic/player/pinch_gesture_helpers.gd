class_name PinchGestureHelpers
extends RefCounted

# Pure pinch-recognizer math extracted from MobileCameraInput so it is
# unit-testable headless without the player scene or the Rust extension
# (see test/player/test_pinch_gesture.gd).

# Finger-spread change (px) from where the candidate pair formed that commits
# the gesture. Deliberately small: a thumb-anchored or angled pinch still trips it.
const COMMIT_SPREAD := 16.0


# Distance between the two fingers.
static func spread(a: Vector2, b: Vector2) -> float:
	return a.distance_to(b)


# The pinch commits once the spread has changed past COMMIT_SPREAD from where
# the pair formed — in either direction (pinch-in or pinch-out). There is no
# "both fingers must move" or axial-angle gate: the spread number alone decides.
static func should_commit(start_spread: float, current_spread: float) -> bool:
	return absf(current_spread - start_spread) >= COMMIT_SPREAD
