extends GameLoopScenario

## Scenario 2: let the app boot to its first rendered screen, capture the viewport to a PNG,
## and assert the frame actually rendered content. A solid-color frame (≈1 distinct color)
## means a black/magenta "never rendered" failure — a common mobile GPU/boot bug this
## smoke-tests for. The PNG is saved for visual review.

## How long to let the app boot + render before capturing.
const SCREENSHOT_SETTLE_SEC := 12.0


func async_run() -> GameLoopResult:
	# Give the engine time to boot and paint the lobby before capturing.
	await runner.get_tree().create_timer(SCREENSHOT_SETTLE_SEC).timeout

	var shot := await async_capture_viewport("scenario2_boot")
	if not shot.succeeded:
		return GameLoopResult.fail("capture failed: %s" % shot.detail)

	var passed := shot.distinct_colors >= SCREENSHOT_MIN_DISTINCT_COLORS
	return GameLoopResult.new(
		passed,
		(
			"saved=%s size=%s distinct_colors=%d (min=%d)"
			% [shot.path, shot.size, shot.distinct_colors, SCREENSHOT_MIN_DISTINCT_COLORS]
		)
	)
