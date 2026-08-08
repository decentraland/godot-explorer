extends SceneTree

# Unit test for the impostor capture queue's priority pick (issue #2206).
#
# The capturer drains its queue by ImpostorCapturePriority.best_index: in-frustum
# avatars before off-frustum ones, nearest camera first. This pins that ordering
# so a refactor can't silently start generating distant / off-screen avatars
# before the close visible ones.
#
# Run headless:
#   .bin/godot/godot4_bin --headless --path godot \
#     --script res://src/test/avatar/test_impostor_capture_queue.gd

const P := preload("res://src/decentraland_components/avatar/impostor/impostor_capture_priority.gd")

var _failures: Array[String] = []


func _initialize() -> void:
	_test_empty()
	_test_nearest_in_frustum_first()
	_test_in_frustum_beats_closer_off_frustum()
	_test_drain_order()
	_finish()


func _e(distance: float, off_frustum: bool) -> Dictionary:
	return {"distance": distance, "off_frustum": off_frustum}


func _test_empty() -> void:
	_expect("empty -> -1", -1, P.best_index([]))


func _test_nearest_in_frustum_first() -> void:
	var entries := [_e(30.0, false), _e(5.0, false), _e(12.0, false)]
	_expect("nearest in-frustum", 1, P.best_index(entries))


# A far in-frustum avatar must still beat a near off-frustum one — we never
# spend a generation on something the camera can't see.
func _test_in_frustum_beats_closer_off_frustum() -> void:
	var entries := [_e(2.0, true), _e(40.0, false)]
	_expect("in-frustum beats closer off-frustum", 1, P.best_index(entries))


# Repeatedly removing best_index reproduces the drain order the capturer uses.
func _test_drain_order() -> void:
	var entries := [_e(20.0, false), _e(8.0, true), _e(15.0, false), _e(3.0, true)]
	var order: Array = []
	while not entries.is_empty():
		var idx := P.best_index(entries)
		order.append(entries[idx])
		entries.remove_at(idx)
	# Expected: in-frustum nearest->farthest (15, 20), then off-frustum (3, 8).
	var dists: Array = []
	for e in order:
		dists.append(e.distance)
	_expect("drain order", str([15.0, 20.0, 3.0, 8.0]), str(dists))


func _expect(ctx: String, expected, actual) -> void:
	if str(expected) != str(actual):
		_failures.append("%s: expected %s, got %s" % [ctx, str(expected), str(actual)])


func _finish() -> void:
	if _failures.is_empty():
		print("[test_impostor_capture_queue] PASS")
		quit(0)
		return
	for f in _failures:
		printerr(f)
	printerr("[test_impostor_capture_queue] FAIL: %d case(s)" % _failures.size())
	quit(1)
