class_name WorldPermissionsHelper

## Client-side gate for private worlds (issue #1725).
##
## Worlds can restrict who may enter through an allow-list configured in Builder /
## Creator Hub. The worlds content server enforces it only for comms, so without this
## gate a non-allowed user still loads and walks the world with comms silently dead.
##
## Reads `GET <worlds-content-server>/world/<name>/permissions` (public, unsigned) and
## evaluates `permissions.access` the same way the server's own access checker does.
## `deployment` / `streaming` are irrelevant to entry and are ignored.
##
## Fails OPEN everywhere: a network error, a malformed response, an unknown access type
## or a missing address all resolve to "allowed". Comms remains the real enforcement, so
## a false negative here costs nothing while a false positive would lock users out.

const ACCESS_ALLOW_LIST := "allow-list"

## How long a resolved access answer stays valid. Long enough that the prefetch at the
## navigation entry point and the safety-net gate in Realm.async_set_realm share one fetch,
## short enough that an allow-list edit is picked up on the next attempt.
const CACHE_TTL_MS := 30000

## world_name -> { "address": String, "allowed": bool, "expires_ms": int }. Only definitive
## server answers are cached; fail-open results (network/parse errors) are not, so a retry
## re-attempts. An address change invalidates an entry (the stored address won't match).
static var _access_cache: Dictionary = {}

## world_name -> Promise<bool> for a request currently in flight. The card-open warm and a fast
## JUMP IN would otherwise each fire their own GET; the second caller awaits this shared promise
## instead. Cleared as soon as the request settles — the value cache above takes over from there.
static var _inflight: Dictionary = {}


## Returns the bare `<name>.dcl.eth` when the realm being entered is a world, or "" when
## it is not (genesis city, a local preview, a custom catalyst...).
##
## Detection is done on the resolved URL rather than on `Realm.is_dcl_ens()`: every world
## resolves to `<worlds-content-server>/world/<name>` through `Realm.resolve_realm_url()`,
## while `is_dcl_ens()`'s regex rejects world names containing anything but alphanumerics.
## It also covers the resume path, where `last_realm_joined` holds the full URL, not the ENS.
static func world_name_from_realm(realm_string: String, resolved_url: String) -> String:
	var worlds_base := DclUrls.worlds_content_server()
	if resolved_url.begins_with(worlds_base):
		var name := Realm.ensure_remove_slash(resolved_url.substr(worlds_base.length()))
		return name.uri_decode().to_lower()

	if Realm.is_dcl_ens(realm_string):
		return realm_string.to_lower()

	return ""


## Whether the current user may enter `world_name`. A fresh value-cache hit returns synchronously;
## otherwise a single GET is shared across concurrent callers (see _inflight) and its definitive
## answer is cached for CACHE_TTL_MS. See the fail-open note above.
static func async_is_allowed(world_name: String) -> bool:
	var address := Global.player_identity.get_address_str().to_lower()
	var now := Time.get_ticks_msec()

	var cached = _access_cache.get(world_name)
	if cached != null and cached.get("address") == address and cached.get("expires_ms", 0) > now:
		return cached.get("allowed")

	# Dedup: if a request for this world is already in flight, await its result instead of firing
	# a second GET (the card-open warm followed by a fast JUMP IN is the common case).
	var pending = _inflight.get(world_name)
	if pending is Promise:
		return await PromiseUtils.async_awaiter(pending)

	var decision := Promise.new()
	_inflight[world_name] = decision
	var allowed := await _async_fetch_access(world_name, address, now)
	_inflight.erase(world_name)
	decision.resolve_with_data(allowed)
	return allowed


## Performs the actual GET + decision and caches definitive answers. Fail-open results
## (network/parse errors) are intentionally left uncached so a later attempt retries.
static func _async_fetch_access(world_name: String, address: String, now: int) -> bool:
	var url := Realm.dcl_world_url(world_name) + "/permissions"
	var promise: Promise = Global.http_requester.request_json(url, HTTPClient.METHOD_GET, "", {})
	var result = await PromiseUtils.async_awaiter(promise)

	if result is PromiseError:
		push_warning(
			"World permissions request failed for %s: %s" % [world_name, result.get_error()]
		)
		return true

	if not result is RequestResponse:
		push_warning("World permissions returned an unexpected response for " + world_name)
		return true

	var response: RequestResponse = result
	var json = response.get_string_response_as_json()
	if not json is Dictionary:
		push_warning("World permissions returned invalid JSON for " + world_name)
		return true

	var allowed := is_access_allowed(json, address)
	_access_cache[world_name] = ({
		"address": address, "allowed": allowed, "expires_ms": now + CACHE_TTL_MS
	})
	return allowed


## Pure decision over a `/permissions` payload. Denies only a plain wallet allow-list the
## address is absent from; every other shape is allowed.
##
## Community-based allow-lists are deliberately let through: resolving membership needs the
## social service, and letting a non-member in (today's behaviour) is far cheaper than
## locking out a legitimate member.
static func is_access_allowed(permissions_json: Dictionary, address: String) -> bool:
	var permissions = permissions_json.get("permissions")
	if not permissions is Dictionary:
		return true

	var access = permissions.get("access")
	if not access is Dictionary:
		return true

	if str(access.get("type", "")) != ACCESS_ALLOW_LIST:
		return true

	# Without an identity there is nothing to match against the list.
	var lower_address := address.to_lower()
	if lower_address.is_empty():
		return true

	var wallets = access.get("wallets", [])
	if wallets is Array:
		for wallet in wallets:
			if str(wallet).to_lower() == lower_address:
				return true

	if str(permissions_json.get("owner", "")).to_lower() == lower_address:
		return true

	var communities = access.get("communities", [])
	if communities is Array and not communities.is_empty():
		return true

	return false
