class_name GameLoopScenario
extends RefCounted

## Base class for a Game Loop scenario (see game_loop_runner.gd). The runner sets `runner`
## then awaits `async_run()`. Shared machinery — deterministic viewport capture + the smoke
## metric, generic async polling, real-button presses — lives here so each scenario file
## under scenarios/ holds only its own flow. Every scenario returns {passed, detail}.

## The smoke metric downscales the frame (aspect-preserved, long side = this) and counts
## distinct colors: a "did the frame render real content, or is it one solid fill?" gate.
## Bigger only widens the count's dynamic range, not the pass/fail — a solid frame is 1
## color at any size. See smoke_distinct_colors().
const SCREENSHOT_SMOKE_MAX_DIM := 128
## Liveness floor: below this a frame is treated as a solid fill (black/magenta "never
## rendered"). solid=1, subtle-gradient≈24, real screens ≥900 — 10 has a huge margin.
const SCREENSHOT_MIN_DISTINCT_COLORS := 10
## Sample this many drawn frames per capture and keep the MAX distinct-color count —
## belt-and-suspenders against the few things time_scale=0 doesn't freeze (built-in
## AnimatedTexture, particle sim, video) tanking one unlucky tick.
const SCREENSHOT_SAMPLE_FRAMES := 3
## Generic async-poll cadence.
const POLL_INTERVAL_SEC := 0.5

## The GameLoopRunner autoload (a Node) — for get_tree()/get_viewport() and logging.
var runner: Node


## Runs the scenario. Override in each subclass; returns a GameLoopResult.
func async_run() -> GameLoopResult:
	push_error("GameLoopScenario.async_run() not overridden")
	return GameLoopResult.fail("scenario not implemented")


## Captures the root viewport to `user://gameloop_<name>.png` (full resolution). Returns a
## CaptureResult whose `distinct_colors` is the smoke metric (MAX over SCREENSHOT_SAMPLE_FRAMES
## drawn frames). Animation is frozen (CaptureMode) for the whole capture so both the PNG and
## the count are deterministic. `settle_frames` (<0 = CaptureMode default) sets how many real
## frames to advance before pinning time — raise it for a screen whose intro animation needs
## longer to reach its settled end state.
func async_capture_viewport(
	capture_name: String, settle_frames: int = -1, hide_world: bool = true
) -> CaptureResult:
	var viewport := runner.get_viewport()
	if viewport == null:
		return CaptureResult.failure("no viewport")

	# Freeze everything animatable to a still, canonical state (avatars -> idle, UI spinner
	# phase pinned, particles hidden). async_enter settles at normal time_scale then sets
	# time_scale=0 — after it returns we only await frame_post_draw (create_timer would hang
	# at time_scale=0). hide_world=false keeps a loaded in-world scene visible.
	var capture := CaptureMode.new()
	var frames := settle_frames if settle_frames >= 0 else CaptureMode.SETTLE_FRAMES
	await capture.async_enter(frames, hide_world)

	# Sample a few drawn frames; keep the richest one (max distinct colors). Under the freeze
	# the frames are ~identical, but the max shrugs off anything time_scale=0 doesn't halt
	# (built-in AnimatedTexture, particle sim, video) blanking one tick.
	var best_img: Image = null
	var best_colors := -1
	for _i in range(SCREENSHOT_SAMPLE_FRAMES):
		await RenderingServer.frame_post_draw
		var frame: Image = viewport.get_texture().get_image()
		if frame == null or frame.is_empty():
			continue
		var colors := smoke_distinct_colors(frame)
		if colors > best_colors:
			best_colors = colors
			best_img = frame

	capture.exit()

	if best_img == null:
		return CaptureResult.failure("null/empty image")

	var path := "user://gameloop_%s.png" % capture_name
	var err := best_img.save_png(path)
	if err != OK:
		return CaptureResult.failure("save_png err=%d" % err)

	var result := CaptureResult.new()
	result.succeeded = true
	result.path = ProjectSettings.globalize_path(path)
	result.size = "%dx%d" % [best_img.get_width(), best_img.get_height()]
	result.distinct_colors = best_colors
	return result


## Counts distinct colors on an aspect-preserved downscale (long side =
## SCREENSHOT_SMOKE_MAX_DIM, nearest-neighbor so no colors are interpolated into being).
## A solid "never rendered" fill collapses to 1; real screens are in the hundreds+.
func smoke_distinct_colors(img: Image) -> int:
	var w := img.get_width()
	var h := img.get_height()
	var longest := maxi(w, h)
	var scale := float(SCREENSHOT_SMOKE_MAX_DIM) / float(maxi(longest, 1))
	var sw := maxi(1, int(round(w * scale)))
	var sh := maxi(1, int(round(h * scale)))

	var small := img.duplicate() as Image
	small.resize(sw, sh, Image.INTERPOLATE_NEAREST)
	var seen := {}
	for y in range(sh):
		for x in range(sw):
			seen[small.get_pixel(x, y).to_rgba32()] = true
	return seen.size()


## Polls `condition` every POLL_INTERVAL_SEC until it returns true or `timeout_sec` elapses.
## Returns whether the condition was met.
func async_wait_for(condition: Callable, timeout_sec: float) -> bool:
	var elapsed := 0.0
	while elapsed < timeout_sec:
		if condition.call():
			return true
		await runner.get_tree().create_timer(POLL_INTERVAL_SEC).timeout
		elapsed += POLL_INTERVAL_SEC
	return condition.call()


## Waits until the SDK scene at the player's current parcel has finished loading, or
## `timeout_sec` elapses. Instruments the "world/scene is ready to capture" gate via
## Global.scene_fetcher.is_scene_loaded(x, y) at current_position — use it after joining a
## world / teleporting, before an in-world capture. Returns whether it loaded in time.
func async_wait_until_scene_loaded(timeout_sec: float) -> bool:
	return await async_wait_for(
		func() -> bool:
			if Global.scene_fetcher == null:
				return false
			var pos: Vector2i = Global.scene_fetcher.current_position
			return Global.scene_fetcher.is_scene_loaded(pos.x, pos.y),
		timeout_sec
	)


## Presses a button by its scene node name (recursing into instanced children), firing the
## real `pressed` signal exactly as a tap would — no private-handler calls. Returns whether
## a BaseButton was found and pressed.
func press_button(root: Node, button_name: String, what: String) -> bool:
	var node := root.find_child(button_name, true, false)
	if node is BaseButton:
		_log("pressing %s (%s)" % [button_name, what])
		(node as BaseButton).pressed.emit()
		return true
	_log("button %s NOT FOUND (%s)" % [button_name, what])
	return false


func _log(msg: String) -> void:
	print("[GAMELOOP] ", msg)
