class_name ContentPolicy

## The iOS block list (#3224): the parcels and worlds the mobile-bff keeps out of the iOS app
## (casino, adult and restricted places, places tagged `banned_ios`). Discover is already
## filtered by the bff; this is the entry gate for teleports, /goto, deep links and Jump In.
##
## Fails OPEN: an unreachable or malformed policy blocks nothing. With the bff's
## `ios-blocklist` flag off the lists come back empty, which is the old allow-list behaviour.

## A `banned_ios` tag set in the backoffice reaches a running app within this window.
const CACHE_TTL_MS := 30 * 60 * 1000

## After a failed fetch, keep what we had and ask again no sooner than this.
const RETRY_AFTER_MS := 60 * 1000

const FETCH_TIMEOUT_S := 3.0

## Sets: "x,y" -> true, and lowercased "<name>.dcl.eth" -> true.
static var _parcels: Dictionary = {}
static var _worlds: Dictionary = {}
static var _expires_ms: int = 0

## Shared by concurrent callers: the boot warm and a fast first Jump In ask once.
static var _inflight: Promise = null


## Starts the first fetch at boot, so the first navigation finds the answer cached.
static func warm() -> void:
	if Global.is_ios_or_emulating():
		_async_refresh()


## Whether iOS must refuse `dest`. Resolves without suspending while the policy is fresh.
static func async_blocks(dest: Destination) -> bool:
	if not Global.is_ios_or_emulating():
		return false
	await _async_refresh()
	return blocks(dest, _parcels, _worlds)


## Pure: whether `dest` lands on a blocked Genesis City parcel or in a blocked world. A preview
## or a custom realm is never blocked, and neither is Genesis City without a parcel.
static func blocks(dest: Destination, parcels: Dictionary, worlds: Dictionary) -> bool:
	match dest.kind:
		Destination.Kind.WORLD:
			var world_name := WorldPermissionsHelper.world_name_from_realm(
				dest.realm_string, Realm.resolve_realm_url(dest.realm_string)
			)
			return worlds.has(world_name)
		Destination.Kind.GENESIS:
			if dest.target_parcel == Destination.UNSPECIFIED:
				return false
			return parcels.has("%d,%d" % [dest.target_parcel.x, dest.target_parcel.y])
	return false


## `{"ok": true, "data": {"parcels": [...], "worlds": [...]}}` -> {"parcels": set, "worlds": set};
## {} on any other shape.
static func parse(json: Variant) -> Dictionary:
	if not json is Dictionary or json.get("ok") != true:
		return {}
	var data = json.get("data")
	if not data is Dictionary:
		return {}
	var parcels = data.get("parcels")
	var worlds = data.get("worlds")
	if not parcels is Array or not worlds is Array:
		return {}
	var parcel_set := {}
	for parcel in parcels:
		parcel_set[str(parcel)] = true
	var world_set := {}
	for world in worlds:
		world_set[str(world).to_lower()] = true
	return {"parcels": parcel_set, "worlds": world_set}


static func _async_refresh() -> void:
	if Time.get_ticks_msec() < _expires_ms:
		return
	if _inflight != null:
		await PromiseUtils.async_awaiter(_inflight)
		return

	var done := Promise.new()
	_inflight = done
	var policy := await _async_fetch()
	if policy.is_empty():
		_expires_ms = Time.get_ticks_msec() + RETRY_AFTER_MS
	else:
		_parcels = policy["parcels"]
		_worlds = policy["worlds"]
		_expires_ms = Time.get_ticks_msec() + CACHE_TTL_MS
	_inflight = null
	done.resolve_with_data(true)


static func _async_fetch() -> Dictionary:
	var url := DclUrls.mobile_bff() + "/content-policy"
	var promise: Promise = Global.http_requester.request_json_with_timeout(
		url, HTTPClient.METHOD_GET, "", {}, FETCH_TIMEOUT_S
	)
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		push_warning("Content policy request failed: " + result.get_error())
		return {}
	if not result is RequestResponse:
		return {}
	var policy := parse(result.get_string_response_as_json())
	if policy.is_empty():
		push_warning("Content policy returned an unexpected payload")
	return policy
