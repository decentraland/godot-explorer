extends Node

# Unit test for Destination / DestinationResolver (#2948).
#
# The refactor's premise is that a navigation is refused BEFORE the loading screen goes
# up and the current scene is offloaded, so every resolve must end in a state that has a
# UI. What is pinned here is that contract, not the plumbing:
#
#   1. classification -- an untrusted realm string lands in the right Kind
#   2. the genesis decision table, including the #2473 regression scenario
#   3. the parcel-check rule: intents are confirmed, restorations are not
#   4. immutability -- resolving returns a new value and keeps intent_id
#   5. world access -- the four worlds-content-server access types
#   6. shared secret -- where a password is proven, and what a rejection means
#   7. scene info -- what the loading screen gets to show before the load starts
#   8. world layout -- a parcel inside a world is checked against the world, not genesis
#   9. bare coordinates -- current realm first, genesis second, refused last
#
# The async half needs a content server and is covered on device, not here.
#
# Runs as a scene, not --script: Destination and DestinationResolver reach Global and
# Realm, and a --script run compiles before the autoloads exist, so every call silently
# became a no-op on a GDScript that had failed to compile -- and the suite still printed
# PASS. _checks below is the guard against that happening again.
#
# Run headless:
#   .bin/godot/godot4_bin --headless --path godot \
#     res://src/test/logic/test_destination.tscn --quit

## Assertions that actually evaluated. A script that fails to compile makes every call
## a no-op, which leaves _failures empty and reports a green suite that never ran.
const EXPECTED_CHECKS := 56

var _failures: Array[String] = []
var _checks: int = 0


func _ready() -> void:
	_test_classification()
	_test_genesis_decision_table()
	_test_parcel_check_rule()
	_test_immutability()
	_test_world_access()
	_test_shared_secret()
	_test_scene_info()
	_test_world_layout()
	_test_bare_coordinate()
	_finish()


func _test_classification() -> void:
	var world := Destination.from_input("myworld.dcl.eth", Vector2i(10, 10))
	_expect(world.kind == Destination.Kind.WORLD, "dcl.eth is a world")

	var preview := Destination.from_input("http://192.168.1.5:8000", Destination.UNSPECIFIED)
	_expect(preview.kind == Destination.Kind.PREVIEW, "a local IP realm is a preview")

	var genesis := Destination.from_input("https://peer-ec1.decentraland.org/", Vector2i(0, 0))
	_expect(genesis.kind == Destination.Kind.GENESIS, "a DAO server is genesis")

	var custom := Destination.from_input("https://my-catalyst.example.com/", Vector2i(0, 0))
	_expect(custom.kind == Destination.Kind.CUSTOM_REALM, "an unknown catalyst is a custom realm")

	_expect(
		Destination.genesis(Vector2i(1, 2)).target_parcel == Vector2i(1, 2),
		"the requested parcel survives construction"
	)


func _test_genesis_decision_table() -> void:
	var min_bounds := Vector2i(-150, -150)
	var max_bounds := Vector2i(163, 158)

	_expect(
		(
			DestinationResolver.genesis_failure(Vector2i(500, 0), min_bounds, max_bounds, "abc")
			== Destination.Failure.PARCEL_OUT_OF_BOUNDS
		),
		"a parcel past the realm bounds is out of bounds"
	)
	_expect(
		(
			DestinationResolver.genesis_failure(Vector2i(10, 10), min_bounds, max_bounds, "abc")
			== Destination.Failure.NONE
		),
		"an in-bounds parcel holding a scene resolves"
	)

	# Regression guard, #2473. Teleporting to a parcel the coordinator had already cached
	# as empty skipped the coordinator request; the empty-set dismissal was skipped with
	# it (scene_fetcher: "coordinator was not busy") and the loading screen never came
	# down. The resolver has to answer that exact input with a terminal verdict, so the
	# load is refused up front instead of started and left hanging.
	var empty_parcel := Vector2i(80, 80)
	var verdict := DestinationResolver.genesis_failure(empty_parcel, min_bounds, max_bounds, "")
	_expect(verdict == Destination.Failure.PARCEL_EMPTY, "an empty parcel is a failure, not a load")

	var dest := Destination.genesis(empty_parcel).resolved_failed(verdict)
	_expect(dest.is_terminal(), "the empty-parcel verdict is terminal, never left resolving")
	_expect(not dest.is_ready(), "a failed destination may not start a load")
	_expect(
		dest.failure_reason() == "parcel_empty", "the failure travels to analytics as parcel_empty"
	)


func _test_parcel_check_rule() -> void:
	var intent := Destination.genesis(Vector2i(80, 80))
	_expect(
		DestinationResolver.should_check_parcel(intent),
		"a teleport intent has its parcel confirmed"
	)

	var restoration := Destination.genesis(Vector2i(80, 80))
	restoration.is_intent = false
	_expect(
		not DestinationResolver.should_check_parcel(restoration),
		"a restoration is not checked: no scene to protect, no intent to cancel"
	)

	_expect(
		not DestinationResolver.should_check_parcel(Destination.world("myworld.dcl.eth")),
		"with no parcel asked for there is nothing to confirm"
	)

	# Locating and gating are separate jobs: a cold start is still described, just not
	# refused. Collapsing the two is what left the boot loading screen blank.
	_expect(
		DestinationResolver.has_target(restoration),
		"a restoration still names the parcel it lands on, so it can be described"
	)
	_expect(DestinationResolver.has_target(intent), "an intent names its parcel too")
	_expect(
		not DestinationResolver.has_target(Destination.world("myworld.dcl.eth")),
		"with no parcel there is nothing to look up"
	)


func _test_immutability() -> void:
	var dest := Destination.world("myworld.dcl.eth")
	var blocked := dest.resolved_needs_password()
	_expect(dest.state == Destination.State.RESOLVING, "resolving does not mutate the original")
	_expect(blocked.state == Destination.State.NEEDS_PASSWORD, "the copy carries the new state")
	_expect(blocked.intent_id == dest.intent_id, "one intent_id spans every copy of the intent")

	var retry := blocked.with_credential("hunter2")
	_expect(retry.attempt == 1, "a credential retry counts as an attempt")
	_expect(retry.intent_id == dest.intent_id, "a retry stays inside the same analytics episode")
	_expect(blocked.credential.is_empty(), "the secret never leaks back into the earlier copy")
	_expect(
		Destination.world("other.dcl.eth").intent_id != dest.intent_id,
		"a separate navigation gets its own intent_id"
	)


func _test_world_access() -> void:
	var address := "0xabc"
	_expect(
		DestinationResolver.access_state({}, address) == Destination.State.READY,
		"an unrestricted world is entered"
	)
	_expect(
		(
			DestinationResolver.access_state(
				{"permissions": {"access": {"type": "shared-secret"}}}, address
			)
			== Destination.State.NEEDS_PASSWORD
		),
		"a shared-secret world asks for a password"
	)
	_expect(
		(
			DestinationResolver.access_state(
				{"permissions": {"access": {"type": "allow-list", "wallets": ["0xdef"]}}}, address
			)
			== Destination.State.NOT_ALLOWED
		),
		"an allow-list without our address refuses entry"
	)
	# Deliberate, not an oversight: the client cannot resolve ownership, and guessing
	# would lock out legitimate owners. Tracked in #2963.
	_expect(
		(
			DestinationResolver.access_state(
				{"permissions": {"access": {"type": "nft-ownership"}}}, address
			)
			== Destination.State.READY
		),
		"nft-ownership still fails open"
	)
	_expect(
		not DestinationResolver.about_is_usable({"content": {}}),
		"an /about without content.publicUrl is refused before any realm state is touched"
	)


func _test_shared_secret() -> void:
	var adapter := "signed-login:https://worlds.example.com/get-comms-adapter/world-x"
	_expect(
		(
			DestinationResolver.comms_handshake_url({"comms": {"fixedAdapter": adapter}})
			== "https://worlds.example.com/get-comms-adapter/world-x"
		),
		"the handshake url comes off the fixedAdapter that /about already carries"
	)
	_expect(
		(
			DestinationResolver
			. comms_handshake_url({"comms": {"fixedAdapter": "offline:offline"}})
			. is_empty()
		),
		"a realm without a signed-login adapter has nowhere to prove a secret"
	)
	_expect(
		DestinationResolver.comms_handshake_url({}).is_empty(),
		"a missing comms block is not a handshake url"
	)

	var world_url := "https://worlds.example.com/world/x.dcl.eth/"
	_expect(
		DestinationResolver.origin_of(world_url) == "https://worlds.example.com",
		"origin is scheme plus authority, without the world path"
	)

	var meta: Dictionary = JSON.parse_string(
		DestinationResolver.handshake_metadata(world_url, "hunter2")
	)
	_expect(meta.get("secret") == "hunter2", "the secret travels in the handshake metadata")
	_expect(
		meta.get("intent") == "dcl:explorer:comms-handshake",
		"the metadata mirrors SignedLoginMeta, so the pre-flight proves what connect sends"
	)

	# The status code does not survive the promise rejection, so these two are told apart
	# by message alone -- and they mean opposite things to the user.
	_expect(
		DestinationResolver.is_rate_limited("Too many shared-secret attempts"),
		"the world rate limiter is a dead end, not another try"
	)
	_expect(
		not DestinationResolver.is_rate_limited("InvalidAccessError"), "a wrong password asks again"
	)


func _test_scene_info() -> void:
	var payload := {
		"id": "bafk",
		"content": [{"file": "scene.json", "hash": "h1"}, {"file": "thumb.png", "hash": "h2"}],
		"metadata": {"display": {"title": "Candy Sheep Meadow", "navmapThumbnail": "thumb.png"}}
	}
	var info := DestinationResolver.scene_info(payload, "https://peer.example.com/content/")
	_expect(info.get("title") == "Candy Sheep Meadow", "the title comes off the scene metadata")
	_expect(
		info.get("image_url") == "https://peer.example.com/content/contents/h2",
		"the thumbnail resolves through the content hash, not the file name"
	)
	_expect(
		info.get("asset_count") == 2,
		"the file count is the denominator the screen shows before the loader has one"
	)

	# Both are normal: plenty of scenes ship no thumbnail, and the screen has to cope.
	var bare := DestinationResolver.scene_info({"content": []}, "https://peer.example.com/content/")
	_expect(
		bare.get("image_url") == "" and bare.get("asset_count") == 0,
		"a payload with nothing showable yields nothing, not an error"
	)

	var dest := Destination.genesis(Vector2i(10, 10)).resolved_ready({}, info)
	_expect(dest.asset_count == 2, "the count travels on the destination to the screen")
	_expect(
		dest.scene_id == "bafk", "and the entity id, which is what the boot bundle is named after"
	)
	_expect(dest.scene_title == "Candy Sheep Meadow", "so does the title, before any Places lookup")


func _test_world_layout() -> void:
	# Shape of /world/<name>/scenes: parcels on the scene, the entity nested beside them.
	var listing := {
		"scenes":
		[
			{"parcels": ["0,2", "0,3"], "entity": {"id": "bafk1", "content": []}},
			{"entity": {"id": "bafk2", "pointers": ["-11,3"], "content": []}},
		]
	}
	var index := DestinationResolver.world_parcel_index(listing)
	_expect(index.has("0,2") and index.has("0,3"), "every parcel of a scene is indexed")
	_expect(index.has("-11,3"), "a scene that lists its parcels only on the entity is indexed too")
	_expect(not index.has("123,123"), "a parcel the world does not have is absent")
	_expect(
		index.get("0,2", {}).get("id") == "bafk1",
		"the index keeps the entity, so the scene at the target describes itself"
	)

	# An unreadable listing must not become "this world has no parcels": that would refuse
	# every teleport into it.
	_expect(
		DestinationResolver.world_parcel_index({}).is_empty(),
		"an empty listing yields an empty index, which the resolver reads as unknown"
	)

	# Realm stores the listing in this shape; reshaping it here is what saves the second
	# /scenes call every world navigation used to make.
	var rows := DestinationResolver.world_scene_urns(
		{"scenes": [{"entityId": "bafk1"}, {"entityId": ""}]},
		"https://worlds.example.com/contents/"
	)
	_expect(rows.size() == 1, "a scene with no entity id is not a row Realm can load")
	_expect(
		rows[0].get("urn") == "urn:decentraland:entity:bafk1",
		"the urn is built the way Realm builds it, so the reused rows are interchangeable"
	)


func _test_bare_coordinate() -> void:
	var explicit := Destination.world("kuruk.dcl.eth", Vector2i(123, 123))
	_expect(
		not explicit.fallback_to_genesis,
		"a realm named on purpose is not second-guessed: that world is where it was asked for"
	)

	var retry := explicit.as_genesis()
	_expect(retry.kind == Destination.Kind.GENESIS, "the retry reads the coordinate as genesis")
	_expect(retry.target_parcel == Vector2i(123, 123), "at the same coordinate that was asked for")
	_expect(
		retry.intent_id == explicit.intent_id,
		"and inside the same intent, so one navigation stays one funnel episode"
	)
	_expect(
		not retry.fallback_to_genesis, "the retry does not fall back again -- genesis is the end"
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
		print("[test_destination] PASS")
		get_tree().quit(0)
		return
	for f in _failures:
		printerr(f)
	printerr("[test_destination] FAIL: %d case(s)" % _failures.size())
	get_tree().quit(1)
