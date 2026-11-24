extends SceneTree

# Unit test for the pinch recognizer math (issue #2709), extracted from
# MobileCameraInput into PinchGestureHelpers so it runs headless:
#   .bin/godot/godot4_bin --headless --path godot \
#     --script res://src/test/player/test_pinch_gesture.gd

const Pinch := preload("res://src/logic/player/pinch_gesture_helpers.gd")

# Portrait phone the zone/spacing numbers are checked against (Samsung A54).
const VIEWPORT := Vector2(1080, 2400)

var _failures: Array[String] = []


func _initialize() -> void:
	_test_commit_threshold()
	_test_thumb_anchored_pinch()
	_test_angled_pinch()
	_test_spread_sign()
	_test_end_on_close()
	_test_closing_gesture_can_toggle()
	_test_zone_size()
	_test_zone_bounds()
	_test_pair_needs_zone_member()
	_test_horizontal_pinch_pair()
	_test_side_deadzone()
	_test_issue_3317_pair()
	_finish()


# Opening commits past COMMIT_SPREAD; closing only commits once the fingers END
# UP together (under the screen's end spacing), so a concentric drag that stops
# half way does nothing.
func _test_commit_threshold() -> void:
	if not is_equal_approx(Pinch.end_spread(VIEWPORT), 1080.0 * Pinch.END_SPREAD_FRACTION):
		_fail("end spacing is not a fraction of the short side")
	if Pinch.should_commit(100.0, 100.0 + Pinch.COMMIT_SPREAD - 0.5):
		_fail("opening committed below the threshold")
	if not Pinch.should_commit(100.0, 100.0 + Pinch.COMMIT_SPREAD):
		_fail("opening did not commit at the threshold")
	if Pinch.should_commit(400.0, 400.0 - Pinch.COMMIT_SPREAD + 0.5):
		_fail("closing committed below the threshold")
	if not Pinch.should_commit(400.0, 400.0 - Pinch.COMMIT_SPREAD):
		_fail("closing did not commit at the threshold (zoom out unusable)")


# One finger anchored (thumb), the other travels: the spread still changes past
# the threshold, so it must commit (no "both fingers must move" gate).
func _test_thumb_anchored_pinch() -> void:
	var anchor := Vector2(100, 400)
	var moving := Vector2(200, 400)
	var start := Pinch.spread(anchor, moving)
	moving.x += Pinch.COMMIT_SPREAD + 10
	if not Pinch.should_commit(start, Pinch.spread(anchor, moving)):
		_fail("thumb-anchored opening pinch not caught")


# Diagonal spread (angled pinch) counts by distance, not by axis.
func _test_angled_pinch() -> void:
	var a := Vector2.ZERO
	var b := Vector2(50, 50)
	var start := Pinch.spread(a, b)
	b += Vector2(20, 20)  # ~28px more spread, diagonally
	if not Pinch.should_commit(start, Pinch.spread(a, b)):
		_fail("angled pinch not caught")


# Sign convention fed to Player.apply_pinch_zoom: spreading grows the spread
# (positive delta = zoom in, toward first person — Roblox-style), closing
# shrinks it (negative = zoom out).
func _test_spread_sign() -> void:
	var a := Vector2.ZERO
	var b := Vector2(100, 0)
	var start := Pinch.spread(a, b)
	b.x += 30
	if Pinch.spread(a, b) - start <= 0:
		_fail("spreading should grow the spread")
	b.x -= 60
	if Pinch.spread(a, b) - start >= 0:
		_fail("closing should shrink the spread")


# The zone is 30% of the screen per axis: half side = 15% of each viewport
# dimension, so it scales with the device instead of a pixel constant.
func _test_zone_size() -> void:
	var viewport := Vector2(1080, 2400)
	var half := Pinch.zone_half_size(viewport)
	if not half.is_equal_approx(Vector2(162, 360)):
		_fail("zone half size is not 15%% per axis: %s" % half)
	var side := half * 2.0
	if not side.is_equal_approx(viewport * 0.3):
		_fail("zone side is not 30%% of the viewport: %s" % side)


# Inside/outside around a centre, border included (a finger exactly on the edge
# counts; excluding it would make the edge flicker).
func _test_zone_bounds() -> void:
	var center := Vector2(540, 1200)
	var half := Vector2(162, 360)
	if not Pinch.is_inside_zone(center, center, half):
		_fail("zone centre rejected")
	if not Pinch.is_inside_zone(center + Vector2(100, -200), center, half):
		_fail("point inside the zone rejected")
	if Pinch.is_inside_zone(center + Vector2(half.x + 0.5, 0), center, half):
		_fail("point past the right edge accepted")
	if Pinch.is_inside_zone(center + Vector2(0, -half.y - 0.5), center, half):
		_fail("point above the top edge accepted")
	if not Pinch.is_inside_zone(center + Vector2(half.x, -half.y), center, half):
		_fail("exact corner rejected (border must be inclusive)")


# Only one of the two fingers has to be the anchor — the other can sit outside
# the zone (idle joystick thumb) and the pinch still forms.
func _test_pair_needs_zone_member() -> void:
	var viewport := Vector2(1080, 2400)
	var center := Vector2(540, 1200)
	var half := Vector2(162, 360)
	if not Pinch.pair_has_zone_member(center, center + Vector2(100, 0), center, half, viewport):
		_fail("pair with both fingers inside rejected")
	if not Pinch.pair_has_zone_member(
		center + Vector2(0, 100), Vector2(140, 2200), center, half, viewport
	):
		_fail("pair with one finger inside rejected")
	if Pinch.pair_has_zone_member(Vector2(140, 2300), Vector2(1000, 120), center, half, viewport):
		_fail("pair with both fingers outside accepted")


# Horizontal pinch: the fingers straddle the avatar and neither is inside the
# narrow zone, so the pair is judged by its midpoint (QA: the vertical gesture
# worked and the horizontal one never did).
func _test_horizontal_pinch_pair() -> void:
	var viewport := Vector2(1080, 2400)
	var center := Vector2(540, 1200)
	var half := Pinch.zone_half_size(viewport)
	var left := Vector2(250, 1200)
	var right := Vector2(830, 1200)
	if Pinch.is_inside_zone(left, center, half) or Pinch.is_inside_zone(right, center, half):
		_fail("horizontal fingers were expected to straddle the zone")
	if not Pinch.pair_has_zone_member(left, right, center, half, viewport):
		_fail("horizontal pinch on the avatar rejected")
	# A pair spanning most of the screen is walk+look, not a pinch.
	if Pinch.pair_has_zone_member(Vector2(120, 2200), Vector2(1000, 200), center, half, viewport):
		_fail("wide diagonal pair accepted by the midpoint rule")


# The side bands are excluded even for a finger that is geometrically inside the
# zone, so a pinch can never be anchored next to the joystick or the opposite
# edge (the joystick's own rect already claims the left band for movement/look).
func _test_side_deadzone() -> void:
	var viewport := Vector2(1080, 2400)
	var margin: float = viewport.x * Pinch.SIDE_DEADZONE_FRACTION
	var center := Vector2(100, 1200)  # avatar drifted into the left band
	var half := Pinch.zone_half_size(viewport)
	if not Pinch.is_in_side_deadzone(Vector2(margin - 1.0, 1200), viewport):
		_fail("left band rejected as dead")
	if not Pinch.is_in_side_deadzone(Vector2(viewport.x - margin + 1.0, 1200), viewport):
		_fail("right band rejected as dead")
	if Pinch.is_in_side_deadzone(Vector2(margin + 1.0, 1200), viewport):
		_fail("point just inside the left band counted as dead")
	if not Pinch.is_inside_zone(center, center, half):
		_fail("avatar centre should be inside the zone")
	if Pinch.can_anchor_pinch(center, center, half, viewport):
		_fail("touch inside the zone but in the side band anchored a pinch")
	if Pinch.pair_has_zone_member(center, Vector2(900, 1200), center, half, viewport):
		_fail("pair anchored in the side band accepted")


# Issue #3317: a thumb in the joystick rect plus a free finger anywhere else must
# NOT read as a pinch — no pair member lands inside the zone around the avatar.
# The legitimate thumb-anchored pinch (second finger on the avatar) still forms.
func _test_issue_3317_pair() -> void:
	var viewport := Vector2(1080, 2400)
	var center := Vector2(540, 1200)  # avatar, mid screen
	var half := Pinch.zone_half_size(viewport)
	var joystick_thumb := Vector2(140, 2100)  # bottom-left, inside the joystick rect
	var free_finger_corner := Vector2(1020, 180)  # top-right, consumed no UI
	if Pinch.pair_has_zone_member(joystick_thumb, free_finger_corner, center, half, viewport):
		_fail("joystick thumb + free corner finger accepted as a pinch (#3317)")
	var pinch_finger := center + Vector2(80, 40)
	if not Pinch.pair_has_zone_member(joystick_thumb, pinch_finger, center, half, viewport):
		_fail("thumb-anchored pinch with the second finger on the avatar rejected")


# The gesture finishes when the fingers are brought together, not when one lifts
# mid-spread and not on the first drag of a pair that simply started close.
func _test_end_on_close() -> void:
	var near: float = Pinch.end_spread(VIEWPORT)
	if not Pinch.should_end(400.0, near - 1.0, near):
		_fail("fingers closed under the end spacing did not end the pinch")
	if Pinch.should_end(400.0, near + 10.0, near):
		_fail("pinch ended while the fingers were still apart")
	if Pinch.should_end(400.0, 300.0, near):
		_fail("pinch ended while merely narrowing")
	if Pinch.should_end(near - 10.0, near - 10.0, near):
		_fail("a pair that started close ended on its first drag (zoom-in would be dead)")
	if not Pinch.should_end(near + 50.0, near - 20.0, near):
		_fail("pair started slightly open and closed did not end")


# A closing pinch commits only when the fingers are already together, so that
# travel has to count toward the 1p<->3p toggle: from a normal spread the closing
# gesture must be able to reach MODE_TOGGLE_SPREAD on its own.
func _test_closing_gesture_can_toggle() -> void:
	var start := 420.0
	var near: float = Pinch.end_spread(VIEWPORT)
	# The commit fires COMMIT_SPREAD in; the travel left before the fingers are
	# together still has to reach the 1p<->3p toggle.
	var acc := near - (start - Pinch.COMMIT_SPREAD)
	if Pinch.toggle_direction(acc) != -1:
		_fail("closing travel left after the commit does not reach the toggle")


func _fail(msg: String) -> void:
	_failures.append(msg)


func _finish() -> void:
	if _failures.is_empty():
		print("[test_pinch_gesture] PASS")
		quit(0)
		return
	for f in _failures:
		printerr(f)
	printerr("[test_pinch_gesture] FAIL: %d case(s)" % _failures.size())
	quit(1)
