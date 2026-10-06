extends Node

# Unit tests for the pure logic behind the controls overlay (issue #3014):
# ControlsFtueCoordinator.decide() and pending_elements(). Runs as a scene so the project's
# global classes resolve.
#
# Run headless:
#   .bin/godot/godot4_bin --headless --path godot \
#     res://src/test/ftue/test_controls_ftue_gate.tscn --quit

const SHOW := ControlsFtueCoordinator.Decision.SHOW
const DEFER := ControlsFtueCoordinator.Decision.DEFER
const NEVER := ControlsFtueCoordinator.Decision.NEVER

var failures := 0


func _ready() -> void:
	_check("fresh install, controls visible", {}, SHOW)
	_check("test mode", {"test_mode": true}, DEFER)
	_check("flag off", {"enabled": false}, DEFER)
	_check("nothing new to label", {"new_elements": 0}, DEFER)
	_check("existing player, allowed", {"existing_player": true}, SHOW)
	_check(
		"existing player, not allowed",
		{"existing_player": true, "show_existing_players": false},
		NEVER
	)
	_check("no scene loaded", {"scene_loaded": false}, DEFER)
	_check("modal open", {"modal_open": true}, DEFER)
	_check("HUD not usable", {"hud_usable": false}, DEFER)
	_check("forced over the flag", {"forced": true, "enabled": false}, SHOW)
	_check("forced still needs the HUD", {"forced": true, "hud_usable": false}, DEFER)
	_check("forced never beats test mode", {"forced": true, "test_mode": true}, DEFER)

	_check_pending("nothing labelled yet", ["menu", "chat", "emotes"], [], false, 3)
	_check_pending("joystick is the only new one", ["menu", "movement"], ["menu"], false, 1)
	_check_pending("all labelled", ["menu", "chat"], ["chat", "menu", "emotes"], false, 0)
	_check_pending("forced relabels what is visible", ["menu", "chat"], ["menu", "chat"], true, 2)

	if failures == 0:
		print("[test_controls_ftue_gate] PASS")
	else:
		printerr("[test_controls_ftue_gate] %d FAILURE(S)" % failures)
	get_tree().quit(1 if failures > 0 else 0)


func _check(
	label: String, overrides: Dictionary, expected: ControlsFtueCoordinator.Decision
) -> void:
	var state := {
		"test_mode": false,
		"forced": false,
		"enabled": true,
		"existing_player": false,
		"show_existing_players": true,
		"scene_loaded": true,
		"modal_open": false,
		"hud_usable": true,
		"new_elements": 2,
	}
	state.merge(overrides, true)
	var actual := ControlsFtueCoordinator.decide(state)
	if actual != expected:
		failures += 1
		printerr("FAIL: %s — expected %d, got %d" % [label, expected, actual])


func _check_pending(
	label: String, visible: Array[String], shown: Array[String], forced: bool, expected: int
) -> void:
	var pending := ControlsFtueCoordinator.pending_elements(
		visible, PackedStringArray(shown), forced
	)
	if pending.size() != expected:
		failures += 1
		printerr("FAIL: %s — expected %d pending, got %s" % [label, expected, pending])
