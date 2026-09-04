extends Node

# Unit tests for graphic profile wiring (issue #2672): every profile key that
# apply_graphic_profile() owns must land on its target — config fields, light
# budget statics and the avatar-particles gate.
#
# Run headless (requires the Rust extension built for DclGlobal):
#   .bin/godot/godot4_bin --headless --path godot \
#     res://src/test/config/test_graphic_profiles.tscn

var failures := 0


func _ready() -> void:
	_run.call_deferred()


func _run() -> void:
	_test_profiles_apply_all_keys()
	_test_custom_is_noop()
	_test_invalid_index_is_noop()

	if failures == 0:
		print("test_graphic_profiles: ALL PASS")
	else:
		printerr("test_graphic_profiles: %d FAILURE(S)" % failures)
	get_tree().quit(failures)


func _test_profiles_apply_all_keys() -> void:
	for i in range(GraphicSettings.PROFILE_DEFINITIONS.size()):
		var profile: Dictionary = GraphicSettings.PROFILE_DEFINITIONS[i]
		GraphicSettings.apply_graphic_profile(i)
		var config := Global.get_config()
		_expect(config.graphic_profile == i, "profile %d: graphic_profile set" % i)
		_expect_near(
			config.view_distance, profile.view_distance, "profile %d: view_distance applied" % i
		)
		_expect(
			config.particle_quality == profile.particle_quality,
			"profile %d: particle_quality applied" % i
		)
		_expect(
			DclLightSourceComponent.max_active_dcl_lights == profile.dcl_max_lights,
			"profile %d: light budget applied" % i
		)
		_expect(
			AvatarAnimHelpers.particles_enabled == (profile.particle_quality > 0),
			"profile %d: avatar particles gate matches particle_quality" % i
		)
		# Rust-side mapping (would have caught apply_particle_quality not being
		# called from apply_graphic_profile).
		var budgets: Array = DclGlobal.get_particle_profile_budgets()
		var expected: Array = [
			[0, 0, true], [2000, 500, true], [15000, 2000, false], [50000, 5000, false]
		][i]
		_expect(
			budgets[0] == expected[0] and budgets[1] == expected[1] and budgets[2] == expected[2],
			"profile %d: rust particle budgets %s == %s" % [i, budgets, expected]
		)


func _test_custom_is_noop() -> void:
	GraphicSettings.apply_graphic_profile(0)
	var config := Global.get_config()
	var view_before: float = config.view_distance
	var particles_before: int = config.particle_quality
	var lights_before: int = DclLightSourceComponent.max_active_dcl_lights
	GraphicSettings.apply_graphic_profile(ConfigData.PROFILE_CUSTOM)
	_expect(config.view_distance == view_before, "Custom: view_distance untouched")
	_expect(config.particle_quality == particles_before, "Custom: particle_quality untouched")
	_expect(
		DclLightSourceComponent.max_active_dcl_lights == lights_before,
		"Custom: light budget untouched"
	)


func _test_invalid_index_is_noop() -> void:
	GraphicSettings.apply_graphic_profile(0)
	var config := Global.get_config()
	var view_before: float = config.view_distance
	GraphicSettings.apply_graphic_profile(-1)
	GraphicSettings.apply_graphic_profile(GraphicSettings.PROFILE_DEFINITIONS.size() + 1)
	_expect(config.view_distance == view_before, "invalid index: nothing applied")


func _expect(cond: bool, label: String) -> void:
	if not cond:
		failures += 1
		printerr("FAIL: " + label)


func _expect_near(actual: float, expected: float, label: String, eps: float = 0.001) -> void:
	if absf(actual - expected) > eps:
		failures += 1
		printerr("FAIL: %s (expected %s, got %s)" % [label, expected, actual])
