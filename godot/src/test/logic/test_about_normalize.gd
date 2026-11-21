extends Node

# Unit test for DestinationResolver.normalize_about.
#
# A realm's /about is whatever the server answers: worlds are user-deployed and `?realm=` is a deep link.
# `Realm._async_commit_realm` reads it with `.get(key, default)`, which only defaults
# when the key is ABSENT, so a present null or a wrong type aborted the commit half-way.
# The normaliser erases every wrong-typed key so those reads see their defaults.
#
# Runs as a scene, not --script: DestinationResolver reaches Global, see
# test_destination.gd for why that matters.
#
# Run headless:
#   .bin/godot/godot4_bin --headless --path godot \
#     res://src/test/logic/test_about_normalize.tscn --quit

const EXPECTED_CHECKS := 21

var _failures: Array[String] = []
var _checks: int = 0


func _ready() -> void:
	_test_wrong_types_are_dropped()
	_test_well_formed_about_is_untouched()
	_test_usable_verdict()
	_finish()


func _test_wrong_types_are_dropped() -> void:
	var about := (
		DestinationResolver
		. normalize_about(
			{
				"content": {"publicUrl": "https://peer.decentraland.org/content/"},
				"configurations": null,
				"lambdas": "x",
				"comms": [],
			}
		)
	)
	_expect(not about.has("configurations"), "present-null configurations is dropped")
	_expect(not about.has("lambdas"), "string lambdas is dropped")
	_expect(not about.has("comms"), "array comms is dropped")
	_expect(
		about.get("configurations", {}).get("scenesUrn", []).is_empty(),
		"the commit's defaults apply once the key is gone"
	)

	var nested := (
		DestinationResolver
		. normalize_about(
			{
				"content": {"publicUrl": 7},
				"configurations":
				{
					"scenesUrn": [1, null, "urn:decentraland:entity:bafy?=&baseUrl=https://x/"],
					"globalScenesUrn": "not-an-array",
					"realmName": ["a"],
					"networkId": "1",
					"localSceneParcels": {"a": 1},
					"cityLoaderContentServer": null,
					"map":
					{
						"sizes":
						[null, "a", {"left": "1", "top": 2}, {"left": -150, "bottom": 5.0}],
					},
				},
				"lambdas": {"publicUrl": null},
			}
		)
	)
	_expect(not nested["content"].has("publicUrl"), "non-string content.publicUrl is dropped")
	var conf: Dictionary = nested["configurations"]
	_expect(
		conf["scenesUrn"].size() == 1 and conf["scenesUrn"][0] is String,
		"non-string urns are dropped"
	)
	_expect(not conf.has("globalScenesUrn"), "string globalScenesUrn is dropped")
	_expect(not conf.has("realmName"), "array realmName is dropped")
	_expect(not conf.has("networkId"), "string networkId is dropped")
	_expect(not conf.has("localSceneParcels"), "dict localSceneParcels is dropped")
	_expect(not conf.has("cityLoaderContentServer"), "null cityLoaderContentServer is dropped")
	var sizes: Array = conf["map"]["sizes"]
	_expect(sizes.size() == 2, "non-dict sizes are dropped")
	_expect(not sizes[0].has("left") and sizes[0]["top"] == 2, "non-number bounds are dropped")
	_expect(sizes[1]["left"] == -150 and sizes[1]["bottom"] == 5.0, "numeric bounds survive")
	_expect(not nested["lambdas"].has("publicUrl"), "null lambdas.publicUrl is dropped")


func _test_well_formed_about_is_untouched() -> void:
	var source := {
		"content": {"publicUrl": "https://peer.decentraland.org/content/"},
		"lambdas": {"publicUrl": "https://peer.decentraland.org/lambdas/"},
		"configurations":
		{
			"realmName": "main",
			"networkId": 1,
			"scenesUrn": ["urn:a", "urn:b"],
			"map": {"sizes": [{"left": -150, "top": 158, "right": 163, "bottom": -150}]},
		},
		"comms": {"protocol": "v3", "fixedAdapter": "livekit:wss://x"},
	}
	var about := DestinationResolver.normalize_about(source)
	_expect(about == source, "a well-formed /about round-trips unchanged")
	_expect(about["configurations"]["scenesUrn"] is Array, "arrays keep their type")
	source["configurations"]["realmName"] = "mutated"
	_expect(about["configurations"]["realmName"] == "main", "the copy is deep")


func _test_usable_verdict() -> void:
	_expect(
		DestinationResolver.about_is_usable(
			DestinationResolver.normalize_about({"content": {"publicUrl": "https://x/"}})
		),
		"content.publicUrl makes an about usable"
	)
	_expect(
		not DestinationResolver.about_is_usable(
			DestinationResolver.normalize_about({"content": {"publicUrl": null}})
		),
		"present-null publicUrl is unusable"
	)
	_expect(
		not DestinationResolver.about_is_usable(
			DestinationResolver.normalize_about({"content": "x"})
		),
		"string content is unusable"
	)


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
		print("[test_about_normalize] PASS")
		get_tree().quit(0)
		return
	for f in _failures:
		printerr(f)
	printerr("[test_about_normalize] FAIL: %d case(s)" % _failures.size())
	get_tree().quit(1)
