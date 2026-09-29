extends SceneTree

## Parsing the asset-renderer input file. A malformed entry has to be reported
## in invalid_items with a reason naming its actual problem: raising instead
## would take down the whole batch, and one stand-in reason for every failure
## sends the caller after the wrong thing.

const InputHelper = preload("res://src/tool/asset_renderer/asset_renderer_input_helper.gd")

var failures := 0


func _check(what: String, actual, expected) -> void:
	if actual == expected:
		return
	printerr("%s: expected %s, got %s" % [what, expected, actual])
	failures += 1


func _shot(overrides: Dictionary) -> Dictionary:
	var shot := {"name": "y0", "destPath": "user://a.png", "width": 768, "height": 768}
	shot.merge(overrides, true)
	return shot


func _parse(payload: Array):
	var path := "user://test_asset_renderer_input.json"
	var file := FileAccess.open(path, FileAccess.WRITE)
	file.store_string(
		JSON.stringify(
			{
				"baseUrl": "https://peer.decentraland.org/content",
				"outputJsonPath": "user://report.json",
				"payload": payload
			}
		)
	)
	file.close()
	return InputHelper.AssetInputFile.from_file_path(path)


## Each entry is rejected for the reason it actually has.
func _test_invalid_entries_are_reported() -> void:
	var cases := [
		["a string, not an object", "payload[0]", "entry is not an object"],
		[{"id": "no-id-urn", "shots": [_shot({})]}, "no-id-urn", "item needs a urn"],
		[{"urn": "urn:a", "shots": [_shot({})]}, "payload[0]", "item needs an id"],
		[
			{"id": "bad-kind", "kind": "scene", "urn": "urn:a", "shots": [_shot({})]},
			"bad-kind",
			"invalid kind scene"
		],
		[{"id": "no-shots", "urn": "urn:a", "shots": []}, "no-shots", "no shots"],
		[
			{"id": "shots-not-array", "urn": "urn:a", "shots": "nope"},
			"shots-not-array",
			"shots has to be an array"
		],
		[
			{"id": "shot-not-object", "urn": "urn:a", "shots": [7]},
			"shot-not-object",
			"a shot is not an object"
		],
		[
			{"id": "no-dest", "urn": "urn:a", "shots": [_shot({"destPath": ""})]},
			"no-dest",
			"shot y0 is missing destPath"
		],
		[
			{"id": "camera-not-object", "urn": "urn:a", "shots": [_shot({"camera": "front"})]},
			"camera-not-object",
			"shot y0 has a camera that is not an object"
		],
		[
			{
				"id": "degenerate-camera",
				"urn": "urn:a",
				"shots":
				[
					_shot(
						{
							"camera":
							{
								"autoFit": false,
								"position": {"x": 1, "y": 1, "z": 1},
								"target": {"x": 1, "y": 1, "z": 1}
							}
						}
					)
				]
			},
			"degenerate-camera",
			"shot y0 needs a distinct camera position and target"
		]
	]

	for case in cases:
		var parsed = _parse([case[0]])
		if parsed == null:
			printerr("%s: the whole file was rejected" % case[1])
			failures += 1
			continue
		_check("%s items" % case[1], parsed.items.size(), 0)
		_check("%s invalid_items" % case[1], parsed.invalid_items.size(), 1)
		if parsed.invalid_items.size() == 1:
			_check("%s id" % case[1], parsed.invalid_items[0].id, case[1])
			_check("%s error" % case[1], parsed.invalid_items[0].error, case[2])


## One bad entry does not stop the entries around it from being rendered.
func _test_a_bad_entry_does_not_lose_the_others() -> void:
	var parsed = _parse(
		[
			{"id": "good-one", "urn": "urn:a", "shots": [_shot({})]},
			12345,
			{"id": "good-two", "urn": "urn:b", "shots": [_shot({})]}
		]
	)
	_check("mixed items", parsed.items.size(), 2)
	_check("mixed invalid_items", parsed.invalid_items.size(), 1)
	if parsed.items.size() == 2:
		_check("mixed first id", parsed.items[0].id, "good-one")
		_check("mixed second id", parsed.items[1].id, "good-two")


## A render size outside what a viewport can serve is corrected, not obeyed.
func _test_render_sizes_are_bounded() -> void:
	var parsed = _parse(
		[
			{
				"id": "sizes",
				"urn": "urn:a",
				"shots":
				[
					_shot({"width": 0, "height": 999999}),
					_shot({"name": "y1", "width": "big", "height": null})
				]
			}
		]
	)
	_check("sizes items", parsed.items.size(), 1)
	if parsed.items.size() != 1:
		return
	var shots = parsed.items[0].shots
	_check("clamped width", shots[0].width, InputHelper.MIN_SHOT_SIDE)
	_check("clamped height", shots[0].height, InputHelper.MAX_SHOT_SIDE)
	_check("width fallback", shots[1].width, 1024)
	_check("height fallback", shots[1].height, 1024)


## A field of the wrong type falls back to its default instead of raising.
func _test_wrongly_typed_fields_fall_back() -> void:
	var parsed = _parse(
		[
			{
				"id": "weird",
				"urn": "urn:a",
				"avatar": "not an object",
				"shots":
				[
					_shot(
						{
							"camera":
							{"autoFit": "yes", "fov": "wide", "position": "origin", "target": []}
						}
					)
				]
			}
		]
	)
	_check("weird items", parsed.items.size(), 1)
	if parsed.items.size() != 1:
		return
	var item = parsed.items[0]
	_check("avatar overrides", item.avatar_overrides, {})
	var camera = item.shots[0].camera
	_check("auto_fit", camera.auto_fit, true)
	_check("fov", camera.fov, 40.0)
	_check("position", camera.position, Vector3.ZERO)
	_check("target", camera.target, Vector3.ZERO)


func _init():
	_test_invalid_entries_are_reported()
	_test_a_bad_entry_does_not_lose_the_others()
	_test_render_sizes_are_bounded()
	_test_wrongly_typed_fields_fall_back()

	if failures > 0:
		print("[test_asset_renderer_input] FAIL: %d case(s)" % failures)
		quit(1)
		return
	print("[test_asset_renderer_input] PASS")
	quit(0)
