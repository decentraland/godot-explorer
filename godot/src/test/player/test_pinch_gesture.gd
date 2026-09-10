extends SceneTree

# Unit test for the pinch recognizer math (issue #2709), extracted from
# MobileCameraInput into PinchGestureHelpers so it runs headless:
#   .bin/godot/godot4_bin --headless --path godot \
#     --script res://src/test/player/test_pinch_gesture.gd

const Pinch := preload("res://src/logic/player/pinch_gesture_helpers.gd")

var _failures: Array[String] = []


func _initialize() -> void:
	_test_commit_threshold()
	_test_thumb_anchored_pinch()
	_test_angled_pinch()
	_test_spread_sign()
	_finish()


# The spread must change past COMMIT_SPREAD — smaller jitters never commit.
func _test_commit_threshold() -> void:
	var start := 100.0
	if Pinch.should_commit(start, start + Pinch.COMMIT_SPREAD - 0.5):
		_fail("committed below the threshold")
	if not Pinch.should_commit(start, start + Pinch.COMMIT_SPREAD):
		_fail("did not commit at the threshold (pinch-out)")
	if not Pinch.should_commit(start, start - Pinch.COMMIT_SPREAD):
		_fail("did not commit at the threshold (pinch-in)")


# One finger anchored (thumb), the other travels: the spread still changes past
# the threshold, so it must commit (no "both fingers must move" gate).
func _test_thumb_anchored_pinch() -> void:
	var anchor := Vector2(100, 400)
	var moving := Vector2(300, 400)
	var start := Pinch.spread(anchor, moving)
	moving.x += Pinch.COMMIT_SPREAD + 10
	if not Pinch.should_commit(start, Pinch.spread(anchor, moving)):
		_fail("thumb-anchored pinch not caught")


# Diagonal spread (angled pinch) counts by distance, not by axis.
func _test_angled_pinch() -> void:
	var a := Vector2.ZERO
	var b := Vector2(50, 50)
	var start := Pinch.spread(a, b)
	b += Vector2(20, 20)  # ~28px more spread, diagonally
	if not Pinch.should_commit(start, Pinch.spread(a, b)):
		_fail("angled pinch not caught")


# Sign convention fed to Player.apply_pinch_zoom: spreading grows the spread
# (positive delta = zoom out), closing shrinks it (negative = zoom in).
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
