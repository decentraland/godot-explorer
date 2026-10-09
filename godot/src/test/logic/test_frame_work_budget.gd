extends SceneTree

# FrameWorkBudget: at most MAX_STEPS non-priority steps per frame, priority steps are not
# capped, every waiter finishes, and a waiter whose owner is freed is dropped.
#
# Run headless:
#   .bin/godot/godot4_bin --headless --path godot \
#     --script res://src/test/logic/test_frame_work_budget.gd

const B := preload("res://src/logic/frame_work_budget.gd")

var _steps_by_frame := {}
var _done := 0


func _initialize() -> void:
	_async_run()


func _async_worker(who: String, owner: Object, steps: int, priority: bool) -> void:
	if owner != null and not priority:
		B.begin_build(owner)
	for i in steps:
		await B.async_acquire(who, owner, priority)
		var frame := Engine.get_process_frames()
		_steps_by_frame[frame] = _steps_by_frame.get(frame, []) + [who]
		OS.delay_usec(500)
	if owner != null:
		B.end_build(owner)
	_done += 1


func _async_run() -> void:
	await process_frame
	var nodes: Array[Node] = []
	for i in 5:
		nodes.append(Node.new())
		_async_worker("avatar%d" % i, nodes[i], 4, false)
	var doomed := Node.new()
	_async_worker("doomed", doomed, 4, false)
	_async_worker("pump", null, 3, false)
	await process_frame
	nodes.append(Node.new())
	_async_worker("local", nodes[-1], 3, true)
	doomed.free()

	var frames := 0
	while _done < 7 and frames < 200:
		await process_frame
		frames += 1

	var ok := _done == 7
	for frame in _steps_by_frame:
		var capped: Array = _steps_by_frame[frame].filter(func(w): return w != "local")
		if capped.size() > B.MAX_STEPS:
			ok = false
			printerr("frame ", frame, " ran ", capped)
		if _steps_by_frame[frame].has("doomed") and frame > 1:
			ok = false
	for node in nodes:
		node.free()
	print("[test_frame_work_budget] ", "PASS" if ok else "FAIL", " (frames=%d)" % frames)
	quit(0 if ok else 1)
