extends RefCounted

# Regression test for #2960: the joypad Jump icon flipped to Glider when
# running over bumpy ground. The icon reads get_jump_action(), so it has to
# reach the same verdict as the jump branches in _physics_process.

var suite_name := "player_glide_icon"
var method_name := "test_jump_icon_matches_press"
var errors: Array[String] = []
var execution_time_seconds := 0.0


func run() -> bool:
	var start := Time.get_ticks_usec()
	var ok := true

	# --- coyote window the grounded flag is built from ------------------------
	# A supported capsule is never in the window.
	ok = (
		_expect_bool(Player.in_coyote_window(true, 0.0, -0.1), false, "supported is never coyote")
		and ok
	)
	# 1-tick is_on_floor() drop at 30 Hz: still a ground press, not a glide.
	ok = (
		_expect_bool(Player.in_coyote_window(false, 0.033, -0.1), true, "1-tick drop is in coyote")
		and ok
	)
	ok = (
		_expect_bool(Player.in_coyote_window(false, 0.149, -0.5), true, "inside coyote window")
		and ok
	)
	ok = (
		_expect_bool(
			Player.in_coyote_window(false, 0.151, -0.5), false, "past coyote window airborne"
		)
		and ok
	)
	# Rising after a jump is never coyote (a jump-pad launch stays cancellable).
	ok = (
		_expect_bool(Player.in_coyote_window(false, 0.033, 3.0), false, "rising is not coyote")
		and ok
	)

	# --- ground ray miss is not altitude -------------------------------------
	# Standing near the realm floor with nothing solid below (the clamp floor is
	# not a body): that is 0.12 m of ground, not a 20 m drop.
	ok = (
		_expect_float(
			Player.resolve_ground_distance(false, 0.0, 0.12), 0.12, "ray miss near the floor"
		)
		and ok
	)
	ok = (
		_expect_float(
			Player.resolve_ground_distance(false, 0.0, 10.1), 10.1, "ray miss high up is altitude"
		)
		and ok
	)
	ok = (
		_expect_float(
			Player.resolve_ground_distance(true, -0.2, 0.1), 0.3, "ray hit measures the hit"
		)
		and ok
	)

	# --- action decision -----------------------------------------------------
	# #2960: grounded must win over any ground reading, including a failed ray.
	ok = (
		_expect_action(
			_action(true, Player.GLIDE_CLOSED, 0, 2.0),
			Player.JUMP_ACTION_JUMP,
			"grounded ignores ground distance"
		)
		and ok
	)
	ok = (
		_expect_action(
			_action(true, Player.GLIDE_CLOSED, 0, INF),
			Player.JUMP_ACTION_JUMP,
			"grounded ignores failed ray"
		)
		and ok
	)
	# Truly airborne with the glider closed and altitude: offer the glider.
	ok = (
		_expect_action(
			_action(false, Player.GLIDE_CLOSED, 0, 2.0),
			Player.JUMP_ACTION_GLIDE_TOGGLE,
			"airborne offers glider"
		)
		and ok
	)
	# #2960: a step-up spends a frame unsupported with vy > 0, so the coyote
	# window is closed and the ground below the step is past the altitude gate.
	# That single tick must not offer the glider.
	ok = (
		_expect_action(
			_action(false, Player.GLIDE_CLOSED, 0, 2.0, 0.0167),
			Player.JUMP_ACTION_NONE,
			"step-up tick offers nothing"
		)
		and ok
	)
	ok = (
		_expect_action(
			_action(false, Player.GLIDE_CLOSED, 0, 2.0, 0.167),
			Player.JUMP_ACTION_GLIDE_TOGGLE,
			"offered once past the coyote window"
		)
		and ok
	)
	# Air jump still owns the press while it is available.
	ok = (
		_expect_action(
			_action(false, Player.GLIDE_CLOSED, 1, 2.0), Player.JUMP_ACTION_JUMP, "air jump wins"
		)
		and ok
	)
	ok = (
		_expect_action(
			_action(false, Player.GLIDE_CLOSED, 0, 0.5),
			Player.JUMP_ACTION_NONE,
			"too low offers nothing"
		)
		and ok
	)
	# Open glider toggles closed even when the scene disables gliding.
	ok = (
		_expect_action(
			_action(false, Player.GLIDE_GLIDING, 1, 0.2),
			Player.JUMP_ACTION_GLIDE_TOGGLE,
			"open glider toggles"
		)
		and ok
	)
	ok = (
		_expect_action(
			_action(false, Player.GLIDE_CLOSING, 1, 2.0),
			Player.JUMP_ACTION_NONE,
			"closing offers nothing"
		)
		and ok
	)

	execution_time_seconds = (Time.get_ticks_usec() - start) / 1_000_000.0
	return ok


# Worst-case-permissive gates except the ones under test: nothing disabled, no
# cooldowns pending, double jump allowed, glider allowed. `fall_elapsed` defaults
# to "falling for a while" (past the coyote window).
func _action(
	grounded: bool, glide_state: int, jump_count: int, ground_distance: float, fall_elapsed := 10.0
) -> int:
	return Player.resolve_jump_action(
		false,
		false,
		false,
		grounded,
		glide_state,
		jump_count,
		false,
		false,
		ground_distance,
		fall_elapsed,
		10.0,
		10.0
	)


func _expect_float(actual: float, expected: float, label: String) -> bool:
	if not is_equal_approx(actual, expected):
		errors.append("%s: expected %f, got %f" % [label, expected, actual])
		return false
	return true


func _expect_bool(actual: bool, expected: bool, label: String) -> bool:
	if actual != expected:
		errors.append("%s: expected %s, got %s" % [label, expected, actual])
		return false
	return true


func _expect_action(actual: int, expected: int, label: String) -> bool:
	if actual != expected:
		errors.append("%s: expected %d, got %d" % [label, expected, actual])
		return false
	return true
