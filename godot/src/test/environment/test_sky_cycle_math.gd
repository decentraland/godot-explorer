extends SceneTree

# Unit tests for SkyCycleMath — the pure day/night math extracted from
# SkyBase for issue #2516 (moon crossfade, sun energy falloff, minimum sun
# elevation clamp).
#
# Run headless:
#   .bin/godot/godot4_bin --headless --path godot \
#     --script res://src/test/environment/test_sky_cycle_math.gd

const M := preload("res://assets/environment/sky_cycle_math.gd")

var failures := 0


func _init() -> void:
	_test_moon_factor()
	_test_sun_energy_factor()
	_test_clamp_direction_elevation()

	if failures == 0:
		print("test_sky_cycle_math: ALL PASS")
	else:
		printerr("test_sky_cycle_math: %d FAILURE(S)" % failures)
	quit(failures)


func _test_moon_factor() -> void:
	_expect_near(M.compute_moon_factor(0.0), 1.0, "midnight is full moon")
	_expect_near(M.compute_moon_factor(0.1), 1.0, "02:24 is full moon")
	_expect_near(M.compute_moon_factor(0.5), 0.0, "midday has no moon")
	_expect_near(M.compute_moon_factor(0.875), 0.0, "moon still off during the whip tail")
	_expect_near(M.compute_moon_factor(0.878), 0.0, "moon fade-in starts at 21:04")
	_expect_near(M.compute_moon_factor(0.905), 1.0, "moon fully in at 21:43")
	_expect_near(M.compute_moon_factor(0.12), 1.0, "moon full before moonset fade")
	_expect_near(M.compute_moon_factor(0.181), 0.0, "moon fully out at the horizon ~04:21")
	_expect_near(M.compute_moon_factor(0.22), 0.0, "moon out at 05:17")
	_expect_near(M.compute_moon_factor(0.8915), 0.5, "dusk crossfade midpoint", 0.05)
	_expect_near(M.compute_moon_factor(0.1505), 0.5, "moonset fade midpoint", 0.05)
	_expect(not M.is_lights_out(0.5), "lights on at midday")
	_expect(not M.is_lights_out(0.0), "lights on at midnight")
	_expect(not M.is_lights_out(0.83), "lights still on at 19:55")
	_expect(M.is_lights_out(0.85), "lights out starts 20:24")
	_expect(M.is_lights_out(0.861), "lights out during the whip")
	_expect(not M.is_lights_out(0.878), "lights back on after the whip")
	_expect_near(M.compute_sun_gate(0.0), 0.0, "sun gated at midnight")
	_expect_near(M.compute_sun_gate(0.17), 0.0, "sun gated during moonset (no 4am flash)")
	_expect_near(M.compute_sun_gate(0.235), 0.0, "sun gate starts ~05:38")
	_expect_near(M.compute_sun_gate(0.265), 1.0, "sun gate full ~06:21")
	_expect_near(M.compute_sun_gate(0.5), 1.0, "sun ungated at midday")
	_expect_near(M.compute_sun_gate(0.785), 1.0, "sun ungated until sunset done")
	_expect_near(M.compute_sun_gate(0.8), 0.0, "sun gated for the night ~19:12")
	_expect_near(M.compute_sun_gate(0.878), 0.0, "sun gated when the night arc rises")
	_expect_near(M.compute_sun_gate(0.25), 0.5, "sunrise gate midpoint", 0.05)
	_expect_near(M.compute_sun_gate(0.7925), 0.5, "sunset gate midpoint", 0.05)


func _test_sun_energy_factor() -> void:
	_expect_near(M.compute_sun_energy_factor(-1.0), 0.0, "deep night sun gives no energy")
	_expect_near(M.compute_sun_energy_factor(-0.05), 0.0, "fade low bound is zero")
	_expect_near(M.compute_sun_energy_factor(0.0), 0.0554, "horizon sun is nearly off", 0.001)
	_expect_near(M.compute_sun_energy_factor(0.3), 1.0, "fade high bound is full")
	_expect_near(M.compute_sun_energy_factor(0.99), 1.0, "zenith sun is full energy")


func _test_clamp_direction_elevation() -> void:
	var min_el := deg_to_rad(12.0)

	# Below the clamp: lifted to exactly min_elevation, azimuth preserved.
	var low := Vector3(0.0, sin(deg_to_rad(2.0)), cos(deg_to_rad(2.0))).normalized()
	var clamped := M.clamp_direction_elevation(low, min_el)
	_expect_near(clamped.y, sin(min_el), "low sun lifted to min elevation", 0.001)
	_expect_near(clamped.x, 0.0, "azimuth preserved (x stays 0)", 0.0001)
	_expect(clamped.z > 0.0, "azimuth preserved (z sign)")
	_expect_near(clamped.length(), 1.0, "clamped direction stays unit", 0.0001)

	# Above the clamp: untouched.
	var high := Vector3(0.3, 0.8, 0.5).normalized()
	_expect(M.clamp_direction_elevation(high, min_el) == high, "high sun direction untouched")

	# Below the horizon: untouched (produces no light anyway).
	var under := Vector3(0.0, -0.3, 0.9).normalized()
	_expect(
		M.clamp_direction_elevation(under, min_el) == under, "below-horizon direction untouched"
	)

	# Degenerate horizontal (looking straight up): no NaN, returned as-is.
	var zenith := Vector3(0.0, 1.0, 0.0)
	var zenith_clamped := M.clamp_direction_elevation(zenith, deg_to_rad(80.0))
	_expect(not is_nan(zenith_clamped.y), "zenith direction doesn't NaN")


func _expect(condition: bool, label: String) -> void:
	if condition:
		print("  PASS: ", label)
	else:
		failures += 1
		printerr("  FAIL: ", label)


func _expect_near(actual: float, expected: float, label: String, tolerance := 0.0001) -> void:
	if abs(actual - expected) <= tolerance:
		print("  PASS: ", label, " (", actual, ")")
	else:
		failures += 1
		printerr("  FAIL: ", label, " — expected ", expected, ", got ", actual)
