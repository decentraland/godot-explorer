extends Node

# Unit tests for the pure logic behind the guided tutorial (issue #2767):
# FtueTutorialCoordinator.should_offer() and parse_scenes(). Runs as a scene so the project's
# global classes resolve.
#
# Run headless:
#   .bin/godot/godot4_bin --headless --path godot \
#     res://src/test/ftue/test_ftue_tutorial_gate.tscn --quit

var failures := 0


func _ready() -> void:
	_check("new player on a tutorial scene", {}, true)
	_check("flag off", {"enabled": false}, false)
	_check("another scene", {"tutorial_scene": false}, false)
	_check("already offered", {"offered": true}, false)
	_check("test mode", {"test_mode": true}, false)
	_check("no scene loaded", {"scene_loaded": false}, false)
	_check("modal open", {"modal_open": true}, false)
	_check("HUD not ready", {"hud_ready": false}, false)
	_check(
		"forced anywhere, even if offered",
		{"forced": true, "enabled": false, "tutorial_scene": false, "offered": true},
		true
	)
	_check("forced still needs the HUD", {"forced": true, "hud_ready": false}, false)
	_check("forced never beats test mode", {"forced": true, "test_mode": true}, false)

	_check_scenes("default", "-3,-2", [Vector2i(-3, -2)])
	_check_scenes("several, with spaces", " -3,-2 ; 10,20;", [Vector2i(-3, -2), Vector2i(10, 20)])
	_check_scenes("malformed entries dropped", "a,b;1;2,3,4;5,6", [Vector2i(5, 6)])
	_check_scenes("empty", "", [])

	if failures == 0:
		print("[test_ftue_tutorial_gate] PASS")
	else:
		printerr("[test_ftue_tutorial_gate] %d FAILURE(S)" % failures)
	get_tree().quit(1 if failures > 0 else 0)


func _check(label: String, overrides: Dictionary, expected: bool) -> void:
	var state := {
		"test_mode": false,
		"forced": false,
		"enabled": true,
		"offered": false,
		"tutorial_scene": true,
		"scene_loaded": true,
		"modal_open": false,
		"hud_ready": true,
	}
	state.merge(overrides, true)
	if FtueTutorialCoordinator.should_offer(state) != expected:
		failures += 1
		printerr("FAIL: %s — expected %s" % [label, expected])


func _check_scenes(label: String, text: String, expected: Array) -> void:
	var scenes := FtueTutorialCoordinator.parse_scenes(text)
	if Array(scenes) != expected:
		failures += 1
		printerr("FAIL: %s — expected %s, got %s" % [label, expected, scenes])
