extends Node

## Firebase Test Lab **Android Game Loop** runner.
##
## Test Lab (and, for local dev, `adb shell am start`) launches the app with the intent
## action `com.google.intent.action.TEST_LOOP` and an integer `scenario` extra. This
## autoload detects that launch, runs the requested scenario, prints a machine-parseable
## result to logcat (which Test Lab captures), and ends the run by finishing the Activity.
##
## The launch intent is bridged to GDScript natively:
##   DclAndroidPlugin.get_deeplink_args()  ->  { action, data, extras{ scenario, guest-seed } }
## Finishing prefers the native `gameLoopFinish` (writes the Test Lab results fd +
## Activity.finish()), else falls back to `get_tree().quit()`.
##
## Local dev (no device / desktop): pass `--game-loop-scenario=N` to run the same scenario
## logic without the Android intent.
##
## ADD A SCENARIO: drop `scenarios/scenario_N_<name>.gd` (extends GameLoopScenario, overrides
## async_run() -> {passed, detail}) and register its script number in SCENARIOS below.

## The action Firebase Test Lab / the Game Loop contract launches us with.
const TEST_LOOP_ACTION := "com.google.intent.action.TEST_LOOP"
## Android plugin singleton (see plugins/dcl-godot-android). Used to finish the run.
const ANDROID_SINGLETON := "dcl-godot-android"
## CLI override for desktop/local runs: `--game-loop-scenario=1`.
const CLI_SCENARIO_ARG := "--game-loop-scenario="
## Golden mode (opt-in per run): a `guest-seed` forces a deterministic guest via a
## seed-derived anchor (the STANDARD Play-as-Guest flow, NOT the device's native anchor) so
## the avatar/name/wallet are reproducible across runs AND devices. Sources: the TEST_LOOP
## intent extra `guest-seed`, or `--game-loop-guest-seed=N` on desktop. Scoped to this
## harness on purpose — there is NO deeplink/UI path, so a production install can't be
## driven into it. When absent, the real (random) guest is used.
const CLI_GUEST_SEED_ARG := "--game-loop-guest-seed="

## Scenario registry: number -> scenario script (each a GameLoopScenario subclass).
const SCENARIOS := {
	1: preload("res://src/test/game_loop/scenarios/scenario_1_guest_login.gd"),
	2: preload("res://src/test/game_loop/scenarios/scenario_2_screenshot.gd"),
	3: preload("res://src/test/game_loop/scenarios/scenario_3_login_flow.gd"),
	4: preload("res://src/test/game_loop/scenarios/scenario_4_fresh_guest.gd"),
	5: preload("res://src/test/game_loop/scenarios/scenario_5_in_world.gd"),
}


func _ready() -> void:
	# Set the golden-mode guest seed BEFORE the lobby boots (autoloads run first), so a
	# seeded guest launch is deterministic from the very first frame.
	_apply_guest_seed_override()
	var scenario := _resolve_scenario()
	if scenario < 0:
		# Not a Game Loop launch — stay dormant, zero overhead for normal runs.
		return
	_log("launch scenario=%d" % scenario)
	# Defer so the first frame (and Global/player_identity wiring) settles first.
	_async_run.call_deferred(scenario)


## Returns the scenario number for this launch, or -1 when this isn't a Game Loop run.
func _resolve_scenario() -> int:
	# CLI override first — lets us exercise scenarios on desktop without a device.
	for arg in OS.get_cmdline_args():
		if arg.begins_with(CLI_SCENARIO_ARG):
			return arg.trim_prefix(CLI_SCENARIO_ARG).to_int()

	# Android: read the launching intent through the native bridge.
	if not DclAndroidPlugin.is_available():
		return -1
	var intent: Dictionary = DclAndroidPlugin.get_deeplink_args()
	if String(intent.get("action", "")) != TEST_LOOP_ACTION:
		return -1
	var extras: Dictionary = intent.get("extras", {})
	# Extras arrive stringified from the Kotlin side (value.toString()).
	return String(extras.get("scenario", "0")).to_int()


## Golden mode: if the Game Loop launch carries a `guest-seed`, force a deterministic guest
## via Global.forced_guest_seed_override — read by Global.get_device_anchor_id(), which turns
## the seed into a stable anchor so the STANDARD Play-as-Guest flow mints a reproducible
## thirdweb guest. Sources: `--game-loop-guest-seed=N` (desktop) or the TEST_LOOP intent's
## `guest-seed` extra. No-op (real guest) when absent. Reachable ONLY through this harness.
func _apply_guest_seed_override() -> void:
	var seed_str := ""
	for arg in OS.get_cmdline_args():
		if arg.begins_with(CLI_GUEST_SEED_ARG):
			seed_str = arg.trim_prefix(CLI_GUEST_SEED_ARG)
			break
	if seed_str.is_empty() and DclAndroidPlugin.is_available():
		var intent: Dictionary = DclAndroidPlugin.get_deeplink_args()
		if String(intent.get("action", "")) == TEST_LOOP_ACTION:
			var extras: Dictionary = intent.get("extras", {})
			seed_str = String(extras.get("guest-seed", extras.get("guest_seed", "")))
	if seed_str.is_valid_int():
		Global.forced_guest_seed_override = seed_str.to_int()
		_log("golden mode: guest-seed override = %d" % Global.forced_guest_seed_override)


## Instantiates the registered scenario, runs it, logs a machine-parseable result, finishes.
func _async_run(scenario: int) -> void:
	var passed := false
	var detail := ""
	var script: GDScript = SCENARIOS.get(scenario)
	if script == null:
		detail = "unknown scenario %d" % scenario
	else:
		var runnable: GameLoopScenario = script.new()
		runnable.runner = self
		var res: GameLoopResult = await runnable.async_run()
		passed = res.passed
		detail = res.detail

	var status := "PASS" if passed else "FAIL"
	# Machine-parseable summary line — Test Lab captures logcat, so this is the
	# authoritative result even without the structured results file.
	_log("RESULT scenario=%d status=%s detail=%s" % [scenario, status, detail])
	_finish(0 if passed else 1, scenario, passed, detail)


## Ends the Game Loop run. Prefers the native finish (writes the Test Lab results fd +
## Activity.finish()); falls back to a plain quit when the AAR predates it. NOTE: Godot
## Android plugin (@UsedByGodot) methods are NOT reported by has_method()/get_method_list(),
## but call() still dispatches them via the JNI bridge — so guard only on has_singleton, no
## method pre-check (which would silently fall through to quit).
func _finish(exit_code: int, scenario: int, passed: bool, detail: String) -> void:
	var results_json := JSON.stringify({"scenario": scenario, "passed": passed, "detail": detail})
	if Engine.has_singleton(ANDROID_SINGLETON):
		Engine.get_singleton(ANDROID_SINGLETON).call("gameLoopFinish", exit_code, results_json)
		return
	get_tree().quit(exit_code)


func _log(msg: String) -> void:
	print("[GAMELOOP] ", msg)
