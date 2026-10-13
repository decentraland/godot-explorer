class_name DestinationResolver

## Answers "is this place real, and may I enter it" before anything is offloaded.
##
## Existence is decided against the content server, never against the Places catalog:
## Places only knows what somebody registered, so a deployed-but-unlisted scene is a
## legitimate destination and an absent catalog entry can never produce a failure.

## The whole resolve, not per request: three sequential calls each under their own
## timeout is how the hang this replaces stayed alive.
const DEADLINE_MS := 8000

## A preview server on the LAN answers fast or is not reachable at all (#2684).
const PREVIEW_DEADLINE_MS := 3000

const ACCESS_SHARED_SECRET := "shared-secret"

## Genesis City bounds when /about carries no map sizes. Same fallback realm.gd uses.
const DEFAULT_MIN_BOUNDS := Vector2i(-150, -150)
const DEFAULT_MAX_BOUNDS := Vector2i(163, 158)


## Retries a bare coordinate against Genesis City when the realm the player is in has no
## such parcel. Sequential rather than raced: the common case pays nothing, and the
## fallback costs one round trip on a path that would otherwise be refused.
static func async_resolve(dest: Destination) -> Destination:
	var resolved := await _async_resolve_once(dest)
	if (
		resolved.failure == Destination.Failure.PARCEL_EMPTY
		and dest.fallback_to_genesis
		and dest.kind != Destination.Kind.GENESIS
	):
		return await _async_resolve_once(dest.as_genesis())
	return resolved


static func _async_resolve_once(dest: Destination) -> Destination:
	var budget := DEADLINE_MS
	if dest.kind == Destination.Kind.PREVIEW:
		budget = PREVIEW_DEADLINE_MS

	var result = await PromiseUtils.async_race(
		[
			func(): return DestinationResolver._resolve_promise(dest),
			func(): return DestinationResolver._timeout_promise(budget),
		]
	)
	if result is Destination:
		return result
	return dest.resolved_failed(Destination.Failure.TIMEOUT)


static func async_retry_with_credential(dest: Destination, secret: String) -> Destination:
	return await async_resolve(dest.with_credential(secret))


# ---- Pure verdicts ---------------------------------------------------------------


## `entity_id` is what the content server reports for the parcel; empty means no scene.
static func genesis_failure(
	parcel: Vector2i, min_bounds: Vector2i, max_bounds: Vector2i, entity_id: String
) -> Destination.Failure:
	if (
		parcel.x < min_bounds.x
		or parcel.x > max_bounds.x
		or parcel.y < min_bounds.y
		or parcel.y > max_bounds.y
	):
		return Destination.Failure.PARCEL_OUT_OF_BOUNDS
	if entity_id.is_empty():
		return Destination.Failure.PARCEL_EMPTY
	return Destination.Failure.NONE


## A world's /about names only its last deployed scene, so its /scenes listing is the
## only place the full parcel layout shows up.
static func world_parcel_index(scenes_json: Dictionary) -> Dictionary:
	var index := {}
	var scenes = scenes_json.get("scenes", [])
	if not scenes is Array:
		return index

	for scene in scenes:
		if not scene is Dictionary:
			continue
		var entity = scene.get("entity", {})
		if not entity is Dictionary:
			entity = {}
		var parcels = scene.get("parcels", [])
		if not parcels is Array or parcels.is_empty():
			parcels = entity.get("pointers", [])
		if not parcels is Array:
			continue
		for parcel in parcels:
			index[str(parcel)] = entity
	return index


## Where a world join lands: Realm spawns from the last entry of the listing.
static func _last_entity(scenes_json: Dictionary) -> Dictionary:
	var scenes = scenes_json.get("scenes", [])
	if not scenes is Array or scenes.is_empty():
		return {}
	var entity = scenes[scenes.size() - 1].get("entity", {})
	return entity if entity is Dictionary else {}


## Reshaped into the rows Realm stores, so applying a destination re-fetches nothing.
static func world_scene_urns(scenes_json: Dictionary, base_content_url: String) -> Array:
	var rows: Array = []
	var scenes = scenes_json.get("scenes", [])
	if not scenes is Array:
		return rows
	for scene in scenes:
		if not scene is Dictionary:
			continue
		var entity_id := str(scene.get("entityId", ""))
		if entity_id.is_empty():
			continue
		(
			rows
			. push_back(
				{
					"urn": "urn:decentraland:entity:" + entity_id,
					"entityId": entity_id,
					"baseUrl": base_content_url,
				}
			)
		)
	return rows


## Both `allow-list` and `nft-ownership` deliberately fail open: #1725 decided the
## first, and the client cannot resolve ownership for the second (#2963).
static func access_state(permissions_json: Dictionary, address: String) -> Destination.State:
	var permissions = permissions_json.get("permissions")
	if not permissions is Dictionary:
		return Destination.State.READY
	var access = permissions.get("access")
	if not access is Dictionary:
		return Destination.State.READY

	match str(access.get("type", "")):
		ACCESS_SHARED_SECRET:
			return Destination.State.NEEDS_PASSWORD
		WorldPermissionsHelper.ACCESS_ALLOW_LIST:
			if WorldPermissionsHelper.is_access_allowed(permissions_json, address):
				return Destination.State.READY
			return Destination.State.NOT_ALLOWED
		_:
			return Destination.State.READY


## Whether an empty parcel must be REFUSED -- a separate job from locating it (see
## has_target). A restoration never refuses: an empty parcel just lands on grass.
static func should_check_parcel(dest: Destination) -> bool:
	return dest.is_intent and has_target(dest)


## Whether there is a parcel to look up. A READY with no scene metadata is normal.
static func has_target(dest: Destination) -> bool:
	return dest.target_parcel != Destination.UNSPECIFIED


## Worlds advertise `adapter: "fixed-adapter:signed-login:<url>"`; `fixedAdapter` is the
## older key and wins when both are present, matching parse_comms_adapter_value in Rust.
## Any other shape means there is nowhere to prove a secret against.
static func comms_handshake_url(about: Dictionary) -> String:
	var comms = about.get("comms")
	if not comms is Dictionary:
		return ""
	var adapter := str(comms.get("fixedAdapter", ""))
	if adapter.is_empty():
		adapter = str(comms.get("adapter", ""))
	adapter = adapter.trim_prefix("fixed-adapter:")
	if not adapter.begins_with("signed-login:"):
		return ""
	return adapter.trim_prefix("signed-login:")


## Mirrors Rust's SignedLoginMeta, `isGuest` included: proving the secret against a
## different shape than the real connect sends would prove nothing.
static func handshake_metadata(realm_url: String, secret: String) -> String:
	return (
		JSON
		. stringify(
			{
				"intent": "dcl:explorer:comms-handshake",
				"signer": "dcl:explorer",
				"isGuest": true,
				"origin": origin_of(realm_url),
				"secret": secret,
			}
		)
	)


## The `origin` the comms handler expects: scheme + authority.
static func origin_of(url: String) -> String:
	var scheme := "http://" if url.begins_with("http://") else "https://"
	var rest := Realm.remove_scheme(url)
	var slash := rest.find("/")
	return scheme + (rest if slash < 0 else rest.substr(0, slash))


## Rate limiter or wrong secret? The status code is lost in the rejection, so the
## message is all there is to tell them apart.
static func is_rate_limited(error_message: String) -> bool:
	var lower := error_message.to_lower()
	return lower.contains("too many") or lower.contains("rate limit")


## What the loading screen shows, from the payload the existence check already read, so
## it costs no request of its own (#2698).
static func scene_info(entity: Dictionary, content_base_url: String) -> Dictionary:
	return _scene_info(entity, content_base_url + "contents/")


## Same, for a base that already addresses content files (a scene urn's baseUrl).
static func _scene_info(entity: Dictionary, contents_base_url: String) -> Dictionary:
	var metadata = entity.get("metadata", {})
	if not metadata is Dictionary:
		metadata = {}
	var display = metadata.get("display", {})
	if not display is Dictionary:
		display = {}
	var content = entity.get("content", [])
	if not content is Array:
		content = []

	var thumbnail := str(display.get("navmapThumbnail", ""))
	var image_url := ""
	if not thumbnail.is_empty():
		for file in content:
			if file is Dictionary and str(file.get("file", "")) == thumbnail:
				image_url = contents_base_url + str(file.get("hash", ""))
				break

	return {
		"id": str(entity.get("id", "")),
		"title": str(display.get("title", "")),
		"creator": _creator_of(metadata),
		"image_url": image_url,
		"asset_count": content.size(),
	}


## `contact.name` is what Places reports as contact_name; `owner` is the fallback.
static func _creator_of(metadata: Dictionary) -> String:
	var contact = metadata.get("contact", {})
	if contact is Dictionary:
		var name := str(contact.get("name", ""))
		if not name.is_empty():
			return name
	return str(metadata.get("owner", ""))


## Same, from the coordinator's typed definition. It does not parse `contact`, so the
## creator still comes from Places on this path.
static func scene_info_from_definition(definition: DclSceneEntityDefinition) -> Dictionary:
	if definition == null:
		return {}
	var mapping := definition.get_content_mapping()
	var thumbnail := String(definition.get_navmap_thumbnail())
	var image_url := ""
	if not thumbnail.is_empty():
		var file_hash := String(mapping.get_hash(thumbnail))
		if not file_hash.is_empty():
			image_url = String(mapping.get_base_url()) + file_hash
	return {
		"title": String(definition.get_title()),
		"image_url": image_url,
		"asset_count": mapping.get_files().size(),
	}


## Used to run inside async_set_realm, after realm state was partly committed and with
## no failure emitted. As a verdict it runs before anything is touched.
static func about_is_usable(about: Dictionary) -> bool:
	var content = about.get("content")
	return content is Dictionary and content.get("publicUrl") is String


## [min, max] parcel bounds advertised by /about, with the genesis fallback.
static func parse_bounds(about: Dictionary) -> Array:
	var sizes = about.get("configurations", {}).get("map", {}).get("sizes", [])
	if not sizes is Array or sizes.is_empty():
		return [DEFAULT_MIN_BOUNDS, DEFAULT_MAX_BOUNDS]

	var first = sizes[0]
	var min_bounds := Vector2i(first.get("left", 0), first.get("bottom", 0))
	var max_bounds := Vector2i(first.get("right", 0), first.get("top", 0))
	for size_dict in sizes:
		min_bounds.x = mini(min_bounds.x, size_dict.get("left", 0))
		min_bounds.y = mini(min_bounds.y, size_dict.get("bottom", 0))
		max_bounds.x = maxi(max_bounds.x, size_dict.get("right", 0))
		max_bounds.y = maxi(max_bounds.y, size_dict.get("top", 0))
	return [min_bounds, max_bounds]


# ---- Resolution ------------------------------------------------------------------


## Two lookup strategies, not four kinds: a catalyst is asked for the scene at a
## pointer, a world and a preview name their scenes themselves.
static func _async_resolve_kind(dest: Destination) -> Destination:
	match dest.kind:
		Destination.Kind.WORLD:
			return await _async_resolve_world(dest)
		Destination.Kind.PREVIEW:
			return await _async_resolve_preview(dest)
		_:
			return await _async_resolve_catalyst(dest)


## GENESIS and CUSTOM_REALM are one lookup. They differ only in the verdict: Genesis
## City's bounds are known and gated, a custom catalyst's are whatever it advertises.
static func _async_resolve_catalyst(dest: Destination) -> Destination:
	var about = await _async_about(dest)
	if about == null or not about_is_usable(about):
		return dest.resolved_failed(Destination.Failure.FETCH_FAILED)
	if not has_target(dest):
		return dest.resolved_ready(about)

	var content_url := Realm.ensure_ends_with_slash(about.get("content").get("publicUrl"))
	var found := await _async_scene_at(content_url, dest.target_parcel)
	if dest.kind == Destination.Kind.GENESIS and should_check_parcel(dest):
		var bounds := parse_bounds(about)
		var reason := genesis_failure(
			dest.target_parcel, bounds[0], bounds[1], str(found.get("id", ""))
		)
		if reason != Destination.Failure.NONE:
			return dest.resolved_failed(reason)
	return dest.resolved_ready(about, found)


static func _async_resolve_world(dest: Destination) -> Destination:
	var about = await _async_about(dest)
	# No status code survives the rejection, so an /about that does not come back is
	# reported as not found -- the dominant cause, and the copy is the same anyway.
	if about == null:
		return dest.resolved_failed(Destination.Failure.WORLD_NOT_FOUND)
	if not about_is_usable(about):
		return dest.resolved_failed(Destination.Failure.FETCH_FAILED)

	var world_name := WorldPermissionsHelper.world_name_from_realm(
		dest.realm_string, Realm.resolve_realm_url(dest.realm_string)
	)
	match access_state(await _async_permissions(world_name), _address()):
		Destination.State.NEEDS_PASSWORD:
			if dest.credential.is_empty():
				return dest.resolved_needs_password()
			# A proven secret is not the end: the world still has to describe itself.
			var refused = await _async_verify_credential(dest, about)
			if refused != null:
				return refused
		Destination.State.NOT_ALLOWED:
			return dest.resolved_not_allowed()

	# Realm gets this payload afterwards, so the listing is read once per navigation.
	var listing := await _async_world_scenes(world_name)
	# /about points content at the Genesis peer, not where a world's files live.
	var worlds_base := DclUrls.worlds_content_server().replace("/world/", "/")

	# The scene at the parcel asked for, or the spawn scene when there is no parcel.
	var entity := _last_entity(listing)
	if has_target(dest):
		var index := world_parcel_index(listing)
		var key := "%d,%d" % [dest.target_parcel.x, dest.target_parcel.y]
		if index.has(key):
			entity = index[key]
		elif should_check_parcel(dest) and not index.is_empty():
			# Or the player walks into empty space inside a world that loaded fine. An
			# unreadable listing lets it through rather than blocking entry.
			return dest.resolved_failed(Destination.Failure.PARCEL_EMPTY)
	return dest.resolved_ready(about, scene_info(entity, worlds_base), listing)


static func _async_resolve_preview(dest: Destination) -> Destination:
	var about = await _async_about(dest)
	if about == null:
		return dest.resolved_failed(Destination.Failure.PREVIEW_UNREACHABLE)

	# A local preview serves its own content, so the pointer lookup works on it like any
	# catalyst. Its /about lists `localSceneParcels` and leaves `scenesUrn` empty, which
	# is why the urn read below is the fallback and not the other way round.
	if has_target(dest) and about_is_usable(about):
		var content_url := Realm.ensure_ends_with_slash(about.get("content").get("publicUrl"))
		var found := await _async_scene_at(content_url, dest.target_parcel)
		if not found.is_empty():
			return dest.resolved_ready(about, found)
	return dest.resolved_ready(about, await _async_urn_scene(about))


# ---- I/O -------------------------------------------------------------------------


## The comms handshake is the only place a secret can be checked: /permissions strips it
## and it is bcrypt-hashed anyway. Returns null when it checked out. Costs no rate-limiter
## budget: the server clears the counter on success and only records a rejected secret.
static func _async_verify_credential(dest: Destination, about: Dictionary):
	var url := comms_handshake_url(about)
	# Nowhere to prove the secret is not the same answer as proven -- letting it through
	# would enter READY on any password and fail later, at the real connect.
	if url.is_empty():
		return dest.resolved_failed(Destination.Failure.FETCH_FAILED)

	var metadata := handshake_metadata(
		Realm.normalize_realm_url(dest.realm_string), dest.credential
	)
	var res = await Global.async_signed_fetch(url, HTTPClient.METHOD_POST, "", metadata)
	if res is RequestResponse:
		return null

	var message: String = ""
	if res is PromiseError:
		message = res.get_error()
	if is_rate_limited(message):
		return dest.resolved_failed(Destination.Failure.RATE_LIMITED)
	return dest.resolved_needs_password()


## Reuses the copy in memory when this is the realm we are in, and is handed to Realm
## afterwards, so a navigation costs one /about rather than three.
static func _async_about(dest: Destination):
	var url := Realm.normalize_realm_url(dest.realm_string)
	if is_instance_valid(Global.realm) and Global.realm.get_realm_url() == url:
		var in_memory: Dictionary = Global.realm.realm_about
		if not in_memory.is_empty():
			return in_memory

	var promise: Promise = Global.http_requester.request_json(
		url + "about", HTTPClient.METHOD_GET, "", {}
	)
	var res = await PromiseUtils.async_awaiter(promise)
	if not res is RequestResponse:
		return null
	var json = res.get_string_response_as_json()
	if not json is Dictionary:
		return null
	return json


## The scene at `parcel`, or {} when it holds none; `id` doubles as the existence answer.
##
## The coordinator answers "" for both "no scene there" and "never asked", so a miss
## always falls through to the network -- reading it as an answer is #2473.
static func _async_scene_at(content_base_url: String, parcel: Vector2i) -> Dictionary:
	if is_instance_valid(Global.realm) and Global.realm.content_base_url == content_base_url:
		var coordinator = Global.scene_fetcher.scene_entity_coordinator
		var cached: String = coordinator.get_scene_entity_id(parcel)
		if not cached.is_empty():
			var known := scene_info_from_definition(coordinator.get_scene_definition(cached))
			known["id"] = cached
			return known

	var url := content_base_url.trim_suffix("/") + "/entities/active"
	var body := JSON.stringify({"pointers": ["%d,%d" % [parcel.x, parcel.y]]})
	var promise: Promise = Global.http_requester.request_json(
		url, HTTPClient.METHOD_POST, body, {"Content-Type": "application/json"}
	)
	var res = await PromiseUtils.async_awaiter(promise)
	if not res is RequestResponse:
		return {}
	var json = res.get_string_response_as_json()
	if json is Array and not json.is_empty() and json[0] is Dictionary:
		return scene_info(json[0], content_base_url)
	return {}


## For a realm that advertises scene urns instead of serving pointers. One that will not
## describe itself still loads: READY without metadata is normal.
static func _async_urn_scene(about: Dictionary) -> Dictionary:
	var urns = about.get("configurations", {}).get("scenesUrn", [])
	if not urns is Array or urns.is_empty():
		return {}
	var parsed = Realm.parse_urn(str(urns[0]))
	if parsed == null or str(parsed.baseUrl).is_empty():
		return {}

	var promise: Promise = Global.http_requester.request_json(
		str(parsed.baseUrl) + str(parsed.entityId), HTTPClient.METHOD_GET, "", {}
	)
	var res = await PromiseUtils.async_awaiter(promise)
	if not res is RequestResponse:
		return {}
	var json = res.get_string_response_as_json()
	if not json is Dictionary:
		return {}
	# A urn baseUrl already addresses content files.
	return _scene_info(json, str(parsed.baseUrl))


## The world's layout and per-scene metadata, in one request.
static func _async_world_scenes(world_name: String) -> Dictionary:
	if world_name.is_empty():
		return {}
	var url := Realm.dcl_world_url(world_name) + "/scenes"
	var promise: Promise = Global.http_requester.request_json(url, HTTPClient.METHOD_GET, "", {})
	var res = await PromiseUtils.async_awaiter(promise)
	if not res is RequestResponse:
		return {}
	var json = res.get_string_response_as_json()
	return json if json is Dictionary else {}


static func _async_permissions(world_name: String) -> Dictionary:
	if world_name.is_empty():
		return {}
	var url := Realm.dcl_world_url(world_name) + "/permissions"
	var promise: Promise = Global.http_requester.request_json(url, HTTPClient.METHOD_GET, "", {})
	var res = await PromiseUtils.async_awaiter(promise)
	if not res is RequestResponse:
		return {}
	var json = res.get_string_response_as_json()
	return json if json is Dictionary else {}


static func _address() -> String:
	if not is_instance_valid(Global.player_identity):
		return ""
	return Global.player_identity.get_address_str().to_lower()


static func _resolve_promise(dest: Destination) -> Promise:
	var promise := Promise.new()
	DestinationResolver._async_fill(dest, promise)
	return promise


static func _async_fill(dest: Destination, promise: Promise) -> void:
	var resolved := await _async_resolve_kind(dest)
	if not promise.is_resolved():
		promise.resolve_with_data(resolved)


static func _timeout_promise(budget_ms: int) -> Promise:
	var promise := Promise.new()
	DestinationResolver._async_expire(budget_ms, promise)
	return promise


static func _async_expire(budget_ms: int, promise: Promise) -> void:
	await Global.get_tree().create_timer(budget_ms / 1000.0).timeout
	if not promise.is_resolved():
		promise.resolve_with_data(null)
