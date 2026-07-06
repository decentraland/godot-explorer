class_name LoginFlowScenario
extends GameLoopScenario

## Shared driver for the standard guest-login UI flow. Waits for the lobby, presses the REAL
## on-screen buttons (Play as Guest -> NEXT -> LETS_GO -> jump-in), and captures a PNG per
## screen it passes through, all the way into the world. Faithful E2E: exercises EULA + guest
## wallet + profile deploy + navigation, not just the identity API.
##
## Subclasses only differ in WHICH guest they use, which decides the path taken:
##   * scenario 3 (fixed --guest-seed) recovers the same wallet  -> COMEBACK path,
##   * scenario 4 (random seed)        mints a brand-new wallet   -> AVATAR_CREATE onboarding.
## Each sets `capture_prefix` so its screenshots get their own names.

## Max time to follow the flow, and the screen poll cadence.
const LOGIN_FLOW_TIMEOUT_SEC := 120.0
const SCREEN_POLL_SEC := 0.5
## Time to wait for the lobby to become the current scene at boot.
const LOBBY_WAIT_TIMEOUT_SEC := 25.0

## Screenshot filename prefix, e.g. "scenario3" -> user://gameloop_scenario3_01_<screen>.png.
## Subclasses set this in _init().
var capture_prefix := "login"


## Drives the flow end to end (auth home -> onboarding/comeback -> world), capturing each
## screen. Returns a GameLoopResult; passed = reached >=2 screens AND ended logged in.
func async_drive_login_flow() -> GameLoopResult:
	var lobby := await _async_wait_for_lobby()
	if lobby == null:
		return GameLoopResult.fail("lobby never became current scene")

	var captured: Array[String] = []

	# Initial auth-home screen (let it settle/render first).
	await runner.get_tree().create_timer(2.0).timeout
	var first_screen: String = lobby.current_screen_name if is_instance_valid(lobby) else "initial"
	await _async_capture_screen(captured, first_screen)

	# Press the real button — same path a tap triggers (EULA, notif perm, guest wallet).
	if not is_instance_valid(lobby):
		return GameLoopResult.fail("lobby invalid")
	if not press_button(lobby, "Button_PlayAsGuest", "PLAY AS GUEST"):
		return GameLoopResult.fail("play-as-guest button not available")

	# Follow the flow: on every screen change capture it, then drive it forward (press
	# NEXT / LETS_GO / jump-in) so the onboarding advances to the world.
	var last_screen: String = first_screen
	var elapsed := 0.0
	var reached_world := false
	while elapsed < LOGIN_FLOW_TIMEOUT_SEC:
		var scene := runner.get_tree().current_scene
		if scene is Lobby and is_instance_valid(scene):
			var s: String = (scene as Lobby).current_screen_name
			if not s.is_empty() and s != last_screen:
				last_screen = s
				# Small settle so the new screen is fully painted before capture.
				await runner.get_tree().create_timer(1.0).timeout
				await _async_capture_screen(captured, s)
				_drive_screen(scene as Lobby, s)
		else:
			reached_world = true
			break
		await runner.get_tree().create_timer(SCREEN_POLL_SEC).timeout
		elapsed += SCREEN_POLL_SEC

	# Reached the world/explorer — let it render, then capture it.
	if reached_world:
		await runner.get_tree().create_timer(10.0).timeout
		await _async_capture_screen(captured, "world")

	var logged_in := not Global.player_identity.get_address_str().is_empty()
	var passed := captured.size() >= 2 and logged_in
	return GameLoopResult.new(
		passed,
		(
			"screens=%d [%s] logged_in=%s reached_world=%s"
			% [captured.size(), ", ".join(captured), logged_in, reached_world]
		)
	)


## Polls until the lobby scene (class_name Lobby) is the current scene, or times out.
func _async_wait_for_lobby() -> Lobby:
	var elapsed := 0.0
	while elapsed < LOBBY_WAIT_TIMEOUT_SEC:
		var scene := runner.get_tree().current_scene
		if scene is Lobby:
			return scene as Lobby
		await runner.get_tree().create_timer(POLL_INTERVAL_SEC).timeout
		elapsed += POLL_INTERVAL_SEC
	return null


## Captures the current frame to a PNG named after the SCREEN (`<prefix>_<screen>.png`, no
## position index) and appends the label to `captured`. Naming by screen — not by position —
## keeps a screen's filename stable even when the sequence varies run to run (e.g. a
## transient DCL_SPLASH shifting every later index), so goldens stay aligned. `settle_frames`
## (<0 = default) lets an animation-heavy screen advance more real frames before the freeze.
func _async_capture_screen(captured: Array[String], label: String, settle_frames: int = -1) -> void:
	var safe := label.to_lower().strip_edges().replace(" ", "_")
	if safe.is_empty():
		safe = "unknown"
	var idx := captured.size() + 1
	var shot := await async_capture_viewport("%s_%s" % [capture_prefix, safe], settle_frames)
	captured.append(label)
	if shot.succeeded:
		_log(
			(
				"%s captured #%d '%s' -> %s (%s, colors=%d)"
				% [capture_prefix, idx, label, shot.path, shot.size, shot.distinct_colors]
			)
		)
	else:
		_log("%s capture #%d '%s' FAILED: %s" % [capture_prefix, idx, label, shot.detail])


## Advances an onboarding screen the way a tap would — by pressing the REAL on-screen button
## (its `pressed` signal), never a private handler. Called once per screen change.
func _drive_screen(lobby: Lobby, screen: String) -> void:
	if not is_instance_valid(lobby):
		return
	match screen:
		"AVATAR_CREATE":
			press_button(lobby, "Button_Next", "AVATAR_CREATE -> NEXT")
		"AVATAR_NAMING":
			# Name is auto-seeded random on screen entry; just confirm to enter.
			press_button(lobby, "Button_LetsGo", "AVATAR_NAMING -> LETS_GO")
		"DISCOVER_FTUE":
			# "Explore more" (Button_Skip) completes onboarding WITHOUT joining a live 3D
			# world — it emits only ftue_completed -> async_close_sign_in, whereas
			# Button_JumpIn_FTUE would _do_jump_in and teleport into a scene. Keeps the flow
			# out of the non-deterministic world (paired with CaptureMode's world-hide).
			press_button(lobby, "Button_Skip", "DISCOVER_FTUE -> Explore more (no world)")
		"COMEBACK":
			# Persisted/seeded guest: "Welcome back" -> LET'S GO (Button_JumpIn) enters world.
			press_button(lobby, "Button_JumpIn", "COMEBACK -> LET'S GO (world)")
