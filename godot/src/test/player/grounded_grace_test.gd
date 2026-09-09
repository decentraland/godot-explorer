extends RefCounted

# Regression test for #2732 (bounce after landing): exercises the pure
# grounded-flag resolver extracted in player.gd. Before the fix the grounded
# flag was raw `is_on_floor()`, so a 1-tick flicker replayed the landing
# animation (the "bounce"); after the fix a 150 ms coyote window debounces
# flicker WITHOUT masking real takeoffs (tap-jumps included).

var suite_name := "player_grounded"
var method_name := "test_grounded_grace_window"
var errors: Array[String] = []
var execution_time_seconds := 0.0


func run() -> bool:
	var start := Time.get_ticks_usec()
	var ok := true

	# Steady ground contact.
	ok = (
		_expect(Player.resolve_is_grounded(true, 0.0, false, 10.0), true, "grounded stays grounded")
		and ok
	)
	# The actual #2732 case: 1-tick is_on_floor() flicker at 30 Hz physics.
	ok = (
		_expect(
			Player.resolve_is_grounded(false, 0.033, false, 10.0), true, "1-tick flicker debounced"
		)
		and ok
	)
	ok = (
		_expect(
			Player.resolve_is_grounded(false, 0.149, false, 10.0),
			true,
			"flicker inside window debounced"
		)
		and ok
	)
	# Sustained airborne (real fall) must still unground right after the window.
	ok = (
		_expect(
			Player.resolve_is_grounded(false, 0.151, false, 10.0),
			false,
			"sustained airborne ungrounds"
		)
		and ok
	)
	# Jump held: takeoff ungrounds immediately even inside the window.
	ok = (
		_expect(
			Player.resolve_is_grounded(false, 0.033, true, 10.0),
			false,
			"jump held ungrounds immediately"
		)
		and ok
	)
	# Tap-jump released within one tick (jump_pressed already false, fresh
	# _time_since_last_jump): the cooldown guard must still unground.
	ok = (
		_expect(
			Player.resolve_is_grounded(false, 0.033, false, 0.05),
			false,
			"tap-jump ungrounds via cooldown guard"
		)
		and ok
	)
	# Ledge walk-off long after the last jump: grace applies (no anim pop).
	ok = (
		_expect(
			Player.resolve_is_grounded(false, 0.1, false, 5.0), true, "ledge walk-off gets grace"
		)
		and ok
	)

	execution_time_seconds = (Time.get_ticks_usec() - start) / 1_000_000.0
	return ok


func _expect(actual: bool, expected: bool, label: String) -> bool:
	if actual != expected:
		errors.append("%s: expected %s, got %s" % [label, expected, actual])
		return false
	return true
