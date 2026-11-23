extends Node

# Unit test for ContentPolicy, the iOS block list entry gate (#3224).
#
#   1. parse -- the bff payload becomes two sets; any other shape blocks nothing
#   2. blocks -- a Genesis parcel, a world by name (any spelling of its realm), and the
#      destinations that are never blocked
#   3. resolved_blocked -- a new value that keeps intent_id
#
# The fetch needs the bff and is covered on device, not here.
#
# Runs as a scene: Destination and WorldPermissionsHelper reach Global and DclUrls, which
# a --script run compiles before the autoloads exist.
#
# Run headless:
#   .bin/godot/godot4_bin --headless --path godot \
#     res://src/test/logic/test_content_policy.tscn --quit

## Assertions that actually evaluated: a script that fails to compile reports green otherwise.
const EXPECTED_CHECKS := 17

var _failures: Array[String] = []
var _checks: int = 0


func _ready() -> void:
	_test_parse()
	_test_blocks()
	_test_resolved_blocked()
	_finish()


func _test_parse() -> void:
	var policy := ContentPolicy.parse(
		{"ok": true, "data": {"mode": "blocklist", "parcels": ["1,2"], "worlds": ["Sin.dcl.eth"]}}
	)
	_expect(policy["parcels"].has("1,2"), "parcels become a set")
	_expect(policy["worlds"].has("sin.dcl.eth"), "worlds are lowercased")

	var allowlist := ContentPolicy.parse(
		{"ok": true, "data": {"mode": "allowlist", "parcels": [], "worlds": []}}
	)
	_expect(allowlist["parcels"].is_empty(), "the allow-list mode lists nothing")

	_expect(ContentPolicy.parse({"ok": false}).is_empty(), "a failed answer is no policy")
	_expect(ContentPolicy.parse({"ok": true, "data": []}).is_empty(), "data must be an object")
	_expect(
		ContentPolicy.parse({"ok": true, "data": {"parcels": "1,2", "worlds": []}}).is_empty(),
		"parcels must be a list"
	)
	_expect(ContentPolicy.parse(null).is_empty(), "null is no policy")


func _test_blocks() -> void:
	var parcels := {"1,2": true}
	var worlds := {"sin.dcl.eth": true}

	_expect(
		ContentPolicy.blocks(Destination.genesis(Vector2i(1, 2)), parcels, worlds),
		"a blocked Genesis parcel"
	)
	_expect(
		not ContentPolicy.blocks(Destination.genesis(Vector2i(1, 3)), parcels, worlds),
		"the parcel next to it"
	)
	_expect(
		not ContentPolicy.blocks(Destination.genesis(Destination.UNSPECIFIED), parcels, worlds),
		"Genesis City without a parcel"
	)
	_expect(
		ContentPolicy.blocks(Destination.from_input("Sin.dcl.eth"), parcels, worlds),
		"a blocked world, whatever its case"
	)
	_expect(
		ContentPolicy.blocks(
			Destination.from_input(DclUrls.worlds_content_server() + "sin.dcl.eth"), parcels, worlds
		),
		"a blocked world given as its full url (the resume path)"
	)
	_expect(
		ContentPolicy.blocks(
			Destination.from_input("sin.dcl.eth", Vector2i(5, 5)), parcels, worlds
		),
		"a parcel inside a blocked world"
	)
	_expect(
		not ContentPolicy.blocks(Destination.from_input("fine.dcl.eth"), parcels, worlds),
		"another world"
	)
	_expect(
		not ContentPolicy.blocks(
			Destination.from_input("http://192.168.1.5:8000", Vector2i(1, 2)), parcels, worlds
		),
		"a preview, even on a blocked parcel"
	)


func _test_resolved_blocked() -> void:
	var dest := Destination.genesis(Vector2i(1, 2))
	var blocked := dest.resolved_blocked()
	_expect(blocked.state == Destination.State.BLOCKED, "resolved_blocked is BLOCKED")
	_expect(blocked.intent_id == dest.intent_id and blocked != dest, "a copy of the same intent")


func _expect(condition: bool, ctx: String) -> void:
	_checks += 1
	if not condition:
		_failures.append(ctx)


func _finish() -> void:
	if _checks != EXPECTED_CHECKS:
		_failures.append(
			"ran %d checks, expected %d -- did the scripts compile?" % [_checks, EXPECTED_CHECKS]
		)
	if _failures.is_empty():
		print("[test_content_policy] PASS")
		get_tree().quit(0)
		return
	for f in _failures:
		printerr(f)
	printerr("[test_content_policy] FAIL: %d case(s)" % _failures.size())
	get_tree().quit(1)
