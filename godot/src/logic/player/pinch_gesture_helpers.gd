class_name PinchGestureHelpers
extends RefCounted

# Pure pinch-recognizer math extracted from MobileCameraInput so it is
# unit-testable headless without the player scene or the Rust extension
# (see test/player/test_pinch_gesture.gd).

# Finger-spread change (px) from where the candidate pair formed that commits
# the gesture. Deliberately small: a thumb-anchored or angled pinch still trips it.
const COMMIT_SPREAD := 16.0

# Accumulated spread change (px) in one direction that swaps 1p<->3p (issue #2709
# team decision: two fixed camera positions, no continuous zoom curve). Large
# enough to be a deliberate gesture, small enough that a slow pinch doesn't feel
# dead — device-QA tunable.
const MODE_TOGGLE_SPREAD_FRACTION := 0.075

# Spread at which the pinching fingers count as brought together, as a fraction of
# the screen's shorter side (~162 px on a 1080x2340 phone). Absolute pixels do not
# work: two fingertips stop ~100 px apart, so a fixed 90 px threshold made the
# closing pinch (zoom out) impossible to commit.
const END_SPREAD_FRACTION := 0.15

# Half side of the pinch zone, as a fraction of the viewport per axis: the zone is
# 30% of the screen wide and tall, centred on the avatar — tighter than the 40%
# first proposed for #3317, which still swallowed fingers landing near the middle.
const ZONE_HALF_FRACTION := 0.15

# Outer band excluded from pinch membership on each side (fraction of screen width).
# The joystick's active area is the left 40% of the screen and its second-finger
# look lives there too, so a touch settled near either edge must not count as the
# finger that anchors the pinch even when the avatar's projection drifts sideways.
const SIDE_DEADZONE_FRACTION := 0.12


# Distance between the two fingers.
static func spread(a: Vector2, b: Vector2) -> float:
	return a.distance_to(b)


# The pinch commits once the spread changes COMMIT_SPREAD from where the pair
# formed, in either direction: opening zooms in, closing zooms out. Requiring the
# closing pinch to END with the fingertips together made it unusable on a phone
# (QA: "practically impossible"); the accidental case #3317 is filtered by where
# the fingers are, not by how far they travelled.
static func should_commit(start_spread: float, current_spread: float) -> bool:
	return absf(current_spread - start_spread) >= COMMIT_SPREAD


# The pinch ends once the fingers are brought together: the spread dropped under
# `near_spread` after starting wider. A pair that merely starts close (the usual
# zoom-in opening) is not ended on its first drag, so spreading still works.
static func should_end(start_spread: float, current_spread: float, near_spread: float) -> bool:
	return current_spread < near_spread and current_spread < start_spread


# "Fingers together" distance for a given viewport.
static func end_spread(viewport_size: Vector2) -> float:
	return minf(viewport_size.x, viewport_size.y) * END_SPREAD_FRACTION


# Half side of the pinch zone in pixels for a given viewport.
static func zone_half_size(viewport_size: Vector2) -> Vector2:
	return viewport_size * ZONE_HALF_FRACTION


# Inside the pinch zone (border included), i.e. on screen near the avatar. A free
# finger anywhere else is not pinch-eligible on its own (#3317).
static func is_inside_zone(point: Vector2, center: Vector2, half_size: Vector2) -> bool:
	return absf(point.x - center.x) <= half_size.x and absf(point.y - center.y) <= half_size.y


# Inside one of the screen's side bands, where the joystick and its second-finger
# look live: never the finger that anchors a pinch.
static func is_in_side_deadzone(point: Vector2, viewport_size: Vector2) -> bool:
	var margin: float = viewport_size.x * SIDE_DEADZONE_FRACTION
	return point.x <= margin or point.x >= viewport_size.x - margin


# The finger that anchors a pinch: inside the zone around the avatar and clear of
# the side bands.
static func can_anchor_pinch(
	point: Vector2, center: Vector2, half_size: Vector2, viewport_size: Vector2
) -> bool:
	return (
		is_inside_zone(point, center, half_size) and not is_in_side_deadzone(point, viewport_size)
	)


# A pinch pair may form only when it is located on the avatar: either one of the two
# fingers anchors it (thumb resting outside the zone — joystick, idle — can still be
# the other finger), or the pair's midpoint lands inside the zone. The midpoint case
# is what makes a horizontal pinch work: its fingers sit left and right of the
# avatar, so neither is inside the narrow zone while the pinch itself is dead on it.
static func pair_has_zone_member(
	a: Vector2, b: Vector2, center: Vector2, half_size: Vector2, viewport_size: Vector2
) -> bool:
	if (
		can_anchor_pinch(a, center, half_size, viewport_size)
		or can_anchor_pinch(b, center, half_size, viewport_size)
	):
		return true
	var midpoint: Vector2 = (a + b) * 0.5
	# ...but only while the pair is compact: a two-finger spread across most of the
	# screen is walk+look, not a pinch on the avatar.
	if a.distance_to(b) > maxf(half_size.x, half_size.y) * 2.0:
		return false
	return (
		is_inside_zone(midpoint, center, half_size)
		and not is_in_side_deadzone(midpoint, viewport_size)
	)


# Selection score for a candidate pair: the further both fingers are from the zone
# centre, the worse. Used to pick the intended pair when three fingers are eligible
# (idle-joystick thumb plus two free ones): the two on the avatar win over the thumb.
static func pair_score(a: Vector2, b: Vector2, center: Vector2) -> float:
	return a.distance_to(center) + b.distance_to(center)


# Accumulated spread that swaps camera mode, as a fraction of the screen's shorter
# side (~81 px on a 1080-wide phone) so it scales with the end spacing below.
static func toggle_spread(viewport_size: Vector2) -> float:
	return minf(viewport_size.x, viewport_size.y) * MODE_TOGGLE_SPREAD_FRACTION


# Net spread direction once the accumulated delta passes `threshold` (see
# toggle_spread): 1 = spreading (fingers opening), -1 = closing (fingers
# together), 0 = below it. The caller maps direction to camera mode
# (Roblox-style: opening zooms in toward first person, closing zooms out).
static func toggle_direction(accumulated_delta: float, threshold: float) -> int:
	if accumulated_delta >= threshold:
		return 1
	if accumulated_delta <= -threshold:
		return -1
	return 0
