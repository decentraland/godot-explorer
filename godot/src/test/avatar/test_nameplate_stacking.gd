extends SceneTree

# Tests for NameplateLayer._stack_position (de-overlap solver, #2637).
# Pins: unconstrained plates stay on their anchor, an overlapping plate with the
# higher instance id moves clear of the lower one, and untracked plates reserve
# no space (no phantom no-go rects after despawn — review P1 on #2717).
#
# Run headless:
#   .bin/godot/godot4_bin --headless --path godot \
#     --script res://src/test/avatar/test_nameplate_stacking.gd

const SIZE := Vector2(200, 40)
const VIEW := Vector2(1600, 720)

var _failures: Array[String] = []


func _initialize() -> void:
	_test_no_overlap_stays_on_anchor()
	_test_overlap_separates()
	_test_untracked_plate_does_not_block()
	_finish()


func _test_no_overlap_stays_on_anchor() -> void:
	var a := Control.new()
	var desired := Vector2(100, 100)
	var pos: Vector2 = NameplateLayer._stack_position(a, desired, SIZE, VIEW)
	_expect("unconstrained plate stays on anchor", desired, pos)
	NameplateLayer._untrack_plate(a)
	a.free()


func _test_overlap_separates() -> void:
	var a := Control.new()
	var b := Control.new()
	NameplateLayer._stack_position(a, Vector2(100, 100), SIZE, VIEW)
	# b spawns overlapping a's collision rect; b has the higher instance id,
	# so b yields. The spring relaxes over frames — iterate to convergence.
	for _i in 200:
		NameplateLayer._stack_position(b, Vector2(120, 110), SIZE, VIEW)
	var rect_a: Rect2 = NameplateLayer._plate_rects[a.get_instance_id()]
	var rect_b: Rect2 = NameplateLayer._plate_rects[b.get_instance_id()]
	_expect("overlapping plates separate", false, rect_a.intersects(rect_b))
	NameplateLayer._untrack_plate(a)
	NameplateLayer._untrack_plate(b)
	a.free()
	b.free()


func _test_untracked_plate_does_not_block() -> void:
	var a := Control.new()
	NameplateLayer._stack_position(a, Vector2(100, 100), SIZE, VIEW)
	NameplateLayer._untrack_plate(a)
	var c := Control.new()
	var pos: Vector2 = NameplateLayer._stack_position(c, Vector2(100, 100), SIZE, VIEW)
	_expect("untracked plate reserves no space", Vector2(100, 100), pos)
	NameplateLayer._untrack_plate(c)
	a.free()
	c.free()


func _expect(ctx: String, expected: Variant, actual: Variant) -> void:
	if expected != actual:
		_failures.append("%s: expected %s, got %s" % [ctx, expected, actual])


func _finish() -> void:
	if _failures.is_empty():
		print("[test_nameplate_stacking] PASS")
		quit(0)
		return
	for f in _failures:
		printerr(f)
	printerr("[test_nameplate_stacking] FAIL: %d case(s)" % _failures.size())
	quit(1)
