extends Node

# Unit test for ModalTransitionGate's pure orientation predicate (#2867).
#
# Runs as a scene, not --script: ModalTransitionGate reaches Global elsewhere in the same
# class, and a --script run compiles before the autoloads exist, so referencing Global
# anywhere in the class fails the whole script's compile -- silently, since Godot still
# reaches the final print and reports a false PASS (the same trap test_destination.gd's
# _checks guard exists for). Run headless:
#   .bin/godot/godot4_bin --headless --path godot \
#     res://src/test/utils/test_modal_transition_gate.tscn --quit

const EXPECTED_CHECKS := 5

var _failures: Array[String] = []
var _checks: int = 0


func _ready() -> void:
	_test_is_orientation_settled()
	_finish()


func _test_is_orientation_settled() -> void:
	_expect(
		ModalTransitionGate._is_orientation_settled(Vector2i(720, 1280), true),
		"a taller-than-wide window matches a portrait request"
	)
	_expect(
		not ModalTransitionGate._is_orientation_settled(Vector2i(720, 1280), false),
		"a taller-than-wide window does not match a landscape request"
	)
	_expect(
		ModalTransitionGate._is_orientation_settled(Vector2i(1280, 720), false),
		"a wider-than-tall window matches a landscape request"
	)
	_expect(
		not ModalTransitionGate._is_orientation_settled(Vector2i(1280, 720), true),
		"a wider-than-tall window does not match a portrait request"
	)
	_expect(
		ModalTransitionGate._is_orientation_settled(Vector2i(800, 800), true),
		"a square window reads as portrait -- documented quirk, not a real device case"
	)


func _expect(condition: bool, ctx: String) -> void:
	_checks += 1
	if not condition:
		_failures.append(ctx)


func _finish() -> void:
	if _checks != EXPECTED_CHECKS:
		_failures.append(
			"ran %d checks, expected %d -- did the script compile?" % [_checks, EXPECTED_CHECKS]
		)
	if _failures.is_empty():
		print("[test_modal_transition_gate] PASS")
		get_tree().quit(0)
		return
	for f in _failures:
		printerr(f)
	printerr("[test_modal_transition_gate] FAIL: %d case(s)" % _failures.size())
	get_tree().quit(1)
