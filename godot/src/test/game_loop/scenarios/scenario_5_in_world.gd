extends LoginFlowScenario

## Scenario 5: IN-WORLD load test. Logs in via the standard flow, joins the world
## `aesironline.dcl.eth`, waits until its scene is actually loaded (up to WORLD_LOAD_TIMEOUT_SEC),
## then captures the live 3D scene (world VISIBLE — unlike the other scenarios, which blank it).
## The scene is still frozen for the capture (scene_runner pause + avatars idle + time_scale=0),
## so a loaded world is a deterministic-ish golden target. Run with a fixed `--guest-seed` so
## the avatar in-frame is reproducible.

const TARGET_WORLD := "aesironline.dcl.eth"
## Max time to wait for the world scene to finish loading after the realm change.
const WORLD_LOAD_TIMEOUT_SEC := 30.0
## Settle after load before capturing (lets late assets/materials pop in).
const WORLD_SETTLE_SEC := 3.0


func _init() -> void:
	capture_prefix = "scenario5"


func async_run() -> GameLoopResult:
	# 1. Log in and reach the explorer (reuses the shared login flow).
	var login := await async_drive_login_flow()
	if not login.passed:
		return GameLoopResult.fail("login flow failed: %s" % login.detail)

	# 2. Join the target world.
	if Global.scene_fetcher == null:
		return GameLoopResult.fail("scene_fetcher unavailable")
	_log("scenario=5 joining world %s" % TARGET_WORLD)
	Global.async_join_world(TARGET_WORLD)

	# 3. Wait until the world scene is actually loaded.
	var loaded := await async_wait_until_scene_loaded(WORLD_LOAD_TIMEOUT_SEC)
	if not loaded:
		return GameLoopResult.fail(
			"world %s not loaded after %ss" % [TARGET_WORLD, WORLD_LOAD_TIMEOUT_SEC]
		)

	# 4. Let it settle, then capture the loaded world (VISIBLE — hide_world=false).
	await runner.get_tree().create_timer(WORLD_SETTLE_SEC).timeout
	var shot := await async_capture_viewport("scenario5_aesironline", -1, false)
	if not shot.succeeded:
		return GameLoopResult.fail("capture failed: %s" % shot.detail)

	return GameLoopResult.new(
		true,
		(
			"world=%s loaded, captured %s (%s, colors=%d)"
			% [TARGET_WORLD, shot.path, shot.size, shot.distinct_colors]
		)
	)
