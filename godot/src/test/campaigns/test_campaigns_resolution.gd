extends Node

# Unit tests for the pure resolution logic behind the campaign FTUE (issues #2670 / #2669):
# parsing the mobile-bff payload and turning a campaign target into the [position, realm]
# pair the teleport paths already speak.
#
# Everything here runs without a network, a profile or the FTUE, which is the point — the
# rest of the feature only reaches a device through a real install.
#
# Runs as a scene, not with --script: a SceneTree script does not register the project's
# global classes, so Realm/CampaignResolution would not resolve (the same reason
# Realm.new() in global.gd fails under --script).
#
# Run headless:
#   .bin/godot/godot4_bin --headless --path godot \
#     res://src/test/campaigns/test_campaigns_resolution.tscn --quit

const C := preload("res://src/logic/campaign_resolution.gd")

var failures := 0


func _ready() -> void:
	_test_parse_response()
	_test_target_position_and_realm()
	_test_is_valid_token()
	_test_metrics_context()

	if failures == 0:
		print("test_campaigns_resolution: ALL PASS")
	else:
		printerr("test_campaigns_resolution: %d FAILURE(S)" % failures)
	get_tree().quit(1 if failures > 0 else 0)


func _test_parse_response() -> void:
	var payload := {
		"ok": true,
		"data": {"campaigns": {"summer-26": {"target": {"type": "genesis", "position": "-9,-9"}}}}
	}
	var parsed := C.parse_response(payload)
	_expect(parsed.has("summer-26"), "parses a well-formed campaign map")
	_expect(parsed.get("summer-26", {}).has("target"), "keeps the campaign body")

	# Fail-open: anything that is not the expected envelope resolves to no campaigns, which
	# callers render as today's FTUE.
	_expect(
		C.parse_response({"ok": false, "data": {"campaigns": {"x": {}}}}).is_empty(), "ok:false"
	)
	_expect(C.parse_response({"ok": true}).is_empty(), "missing data")
	_expect(C.parse_response({"ok": true, "data": {}}).is_empty(), "missing campaigns")
	_expect(C.parse_response({"ok": true, "data": []}).is_empty(), "data is not a dictionary")
	_expect(
		C.parse_response({"ok": true, "data": {"campaigns": []}}).is_empty(),
		"campaigns is not a dictionary"
	)
	_expect(C.parse_response(null).is_empty(), "null payload")
	_expect(C.parse_response("nope").is_empty(), "string payload")


func _test_target_position_and_realm() -> void:
	var genesis := C.target_position_and_realm({"target": {"type": "genesis", "position": "-9,-9"}})
	_expect(genesis.size() == 2, "genesis target resolves")
	_expect(genesis[0] == Vector2i(-9, -9), "genesis parcel is parsed")

	# int() answers 0 for anything unparseable, so without an explicit check these would all
	# resolve to the valid parcel 0,0: the boot would "succeed", the token would be consumed,
	# and every install of that campaign would land on the Genesis spawn instead.
	for bad in ["abc,def", "10,def", "abc,20", "x=10,y=20", ",", " , "]:
		_expect(
			(
				C
				. target_position_and_realm({"target": {"type": "genesis", "position": bad}})
				. is_empty()
			),
			"unparseable genesis position rejected: '%s'" % bad
		)
	# is_valid_int() only checks characters, so a value too large for the int32 inside Vector2i
	# truncates to something else entirely — "4294967296" lands on 0,0, the exact failure the
	# character check was added to stop.
	# The negative twins matter as much as the positive ones: to_int() clamps out-of-range
	# input to INT64_MIN/INT64_MAX, and absi(INT64_MIN) is itself negative — so a bound written
	# with absi() lets those through and Vector2i truncates them to 0.
	var huge_values := [
		"4294967296,0",
		"0,4294967296",
		"2147483648,0",
		"99999999999999999999,0",
		"-99999999999999999999,0",
		"0,-99999999999999999999",
		"-9223372036854775808,0",
		"9223372036854775808,0",
		"10000,0",
		"0,-10000",
	]
	for huge in huge_values:
		_expect(
			(
				C
				. target_position_and_realm({"target": {"type": "genesis", "position": huge}})
				. is_empty()
			),
			"out-of-range genesis position rejected: '%s'" % huge
		)
	# The bound matches the BFF regex and the DB CHECK (1-4 digits), so the widest storable
	# position still resolves.
	var edge := C.target_position_and_realm(
		{"target": {"type": "genesis", "position": "-9999,9999"}}
	)
	_expect(edge.size() == 2 and edge[0] == Vector2i(-9999, 9999), "widest storable parcel")

	# Whitespace around real numbers is still accepted.
	var padded := C.target_position_and_realm(
		{"target": {"type": "genesis", "position": " 10 , -20 "}}
	)
	_expect(padded.size() == 2 and padded[0] == Vector2i(10, -20), "padded parcel is parsed")
	_expect(
		String(genesis[1]) == String(DclUrls.main_realm()), "genesis target uses the main realm"
	)
	_expect(
		not C.is_world_target({"target": {"type": "genesis", "position": "0,0"}}),
		"genesis is not a world target"
	)

	var world := C.target_position_and_realm(
		{"target": {"type": "world", "name": "myworld.dcl.eth"}}
	)
	_expect(world.size() == 2, "world target resolves")
	_expect(world[0] == Vector2i.ZERO, "world target has no parcel")
	_expect(String(world[1]) == "myworld.dcl.eth", "world target carries the realm")
	_expect(
		C.is_world_target({"target": {"type": "world", "name": "myworld.dcl.eth"}}),
		"world target is flagged"
	)

	# A name Realm.is_dcl_ens rejects would be treated as a realm URL by the teleport path and
	# never reach the intended world, so it must not resolve at all.
	for name in ["my-world.dcl.eth", "sub.myworld.dcl.eth", "myworld.eth", "myworld", ""]:
		_expect(
			C.target_position_and_realm({"target": {"type": "world", "name": name}}).is_empty(),
			"unroutable world name rejected: '%s'" % name
		)

	for bad in [
		{},
		{"target": null},
		{"target": "genesis"},
		{"target": {"type": "parcel", "position": "0,0"}},
		{"target": {"type": "genesis"}},
		{"target": {"type": "genesis", "position": "0"}},
		{"target": {"type": "genesis", "position": "0,0,0"}},
	]:
		_expect(C.target_position_and_realm(bad).is_empty(), "unusable target rejected: %s" % [bad])


func _test_is_valid_token() -> void:
	# Mirrors the BFF's save rule and the Rust attribution path. A token that would be
	# rejected there must not be stored from a deeplink either: a stored token is never
	# replaced, so a junk one blocks the real install-attribution token on that install.
	_expect(C.is_valid_token("summer2022"), "plain alphanumeric token")
	_expect(C.is_valid_token("launch-26"), "dash-separated token")
	_expect(C.is_valid_token("a"), "single character token")

	_expect(not C.is_valid_token(""), "empty token")
	_expect(not C.is_valid_token("Summer2022"), "uppercase rejected")
	_expect(not C.is_valid_token("-lead"), "leading dash rejected")
	_expect(not C.is_valid_token("trail-"), "trailing dash rejected")
	_expect(not C.is_valid_token("double--dash"), "consecutive dashes rejected")
	_expect(not C.is_valid_token("under_score"), "underscore rejected")
	_expect(not C.is_valid_token("with space"), "space rejected")
	_expect(not C.is_valid_token("a".repeat(65)), "over-long token rejected")


func _test_metrics_context() -> void:
	var resolved := C.metrics_context(
		{
			"token": "summer-26",
			"campaign": {"target": {"type": "genesis", "position": "0,0"}},
			"fallback_reason": C.FALLBACK_NONE
		}
	)
	_expect(resolved["campaign_token"] == "summer-26", "context carries the token")
	_expect(resolved["campaign_resolved"] == true, "resolved campaign is flagged")
	_expect(resolved["campaign_fallback_reason"] == "", "resolved campaign has no fallback reason")

	var fell_back := C.metrics_context(
		{"token": "ghost", "campaign": {}, "fallback_reason": C.FALLBACK_UNKNOWN_TOKEN}
	)
	_expect(fell_back["campaign_resolved"] == false, "fallback is flagged as unresolved")
	_expect(
		fell_back["campaign_fallback_reason"] == "unknown_token", "fallback reason is preserved"
	)

	# The default FTUE (no campaign at all) still produces a well-formed context.
	var absent := C.metrics_context({})
	_expect(absent["campaign_token"] == "", "empty resolution has no token")
	_expect(absent["campaign_resolved"] == false, "empty resolution is unresolved")
	_expect(absent["campaign_fallback_reason"] == "no_token", "empty resolution reports no_token")


func _expect(condition: bool, label: String) -> void:
	if condition:
		return
	failures += 1
	printerr("  FAIL: %s" % label)
