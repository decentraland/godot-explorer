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
	_test_metrics_context()

	if failures == 0:
		print("test_campaigns_resolution: ALL PASS")
	else:
		printerr("test_campaigns_resolution: %d FAILURE(S)" % failures)
	get_tree().quit(1 if failures > 0 else 0)


func _test_parse_response() -> void:
	var payload := {
		"ok": true,
		"data":
		{
			"campaigns":
			{
				"summer-26":
				{
					"mode": "ftue",
					"target": {"type": "genesis", "position": "-9,-9"},
					"title": "Summer is here",
					"cta": "Jump into Summer",
					"placeIds": []
				}
			}
		}
	}
	var parsed := C.parse_response(payload)
	_expect(parsed.has("summer-26"), "parses a well-formed campaign map")
	_expect(
		String(parsed.get("summer-26", {}).get("mode", "")) == "ftue", "keeps the campaign body"
	)

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


func _test_metrics_context() -> void:
	var resolved := C.metrics_context(
		{
			"token": "summer-26",
			"campaign": {"mode": "bypass", "target": {"type": "genesis", "position": "0,0"}},
			"fallback_reason": C.FALLBACK_NONE
		}
	)
	_expect(resolved["campaign_token"] == "summer-26", "context carries the token")
	_expect(resolved["campaign_mode"] == "bypass", "context carries the mode")
	_expect(resolved["campaign_resolved"] == true, "resolved campaign is flagged")
	_expect(resolved["campaign_fallback_reason"] == "", "resolved campaign has no fallback reason")

	var fell_back := C.metrics_context(
		{"token": "ghost", "campaign": {}, "fallback_reason": C.FALLBACK_UNKNOWN_TOKEN}
	)
	_expect(fell_back["campaign_resolved"] == false, "fallback is flagged as unresolved")
	_expect(fell_back["campaign_mode"] == "", "fallback has no mode")
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
