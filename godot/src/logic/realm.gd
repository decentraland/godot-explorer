class_name Realm
extends DclRealm

signal realm_changed
signal realm_changing
signal realm_change_failed(new_realm_string: String, reason: String)
## The target world is private and the user is not on its allow-list (#1725).
## Kept apart from `realm_change_failed` so it surfaces the "is private" modal
## instead of the generic "World unavailable" toast.
signal realm_access_denied(new_realm_string: String, world_name: String)

var _has_realm = false


static func is_dcl_ens(str_param: String) -> bool:
	return DclRealmRules.is_dcl_ens(str_param)


static func is_genesis_city(_realm_name: String) -> bool:
	return DclRealmRules.is_genesis_city(_realm_name)


static func is_local_preview(_realm_name: String) -> bool:
	return DclRealmRules.is_local_preview(_realm_name)


static func dcl_world_url(dcl_name: String) -> String:
	return DclRealmRules.dcl_world_url(dcl_name)


static func ensure_reduce_url(url):
	return DclRealmRules.ensure_reduce_url(url)


static func ensure_dcl_ens(url: String) -> String:
	return DclRealmRules.ensure_dcl_ens(url)


static func ensure_remove_slash(str_param: String) -> String:
	return DclRealmRules.ensure_remove_slash(str_param)


static func ensure_ends_with_slash(str_param: String) -> String:
	return DclRealmRules.ensure_ends_with_slash(str_param)


static func ensure_starts_with_https(str_param: String) -> String:
	return DclRealmRules.ensure_starts_with_https(str_param)


static func remove_scheme(str_param: String) -> String:
	return DclRealmRules.remove_scheme(str_param)


static func resolve_realm_url(value: String) -> String:
	return DclRealmRules.resolve_realm_url(value)


## The canonical url a realm string resolves to — the form async_set_realm connects with and
## stores in `realm_url`. Two spellings of the same destination ("SpaceRunner.dcl.eth" and
## "spacerunner.dcl.eth", with or without a trailing slash or scheme) normalise to one string,
## so callers can tell "somewhere else" from "where we already are" (#2816: the realm now comes
## from arbitrary scene input, where `realm_string` equality is not enough).
static func normalize_realm_url(value: String) -> String:
	return DclRealmRules.normalize_realm_url(value)


static func get_params(url: String) -> Dictionary:
	return DclRealmRules.get_params(url)


static func parse_urn(urn: String):
	return DclRealmRules.parse_urn(urn)


func has_realm():
	return _has_realm


func async_clear_realm():
	_has_realm = false
	realm_url = ""
	realm_string = ""
	realm_about = {}
	realm_scene_urns.clear()
	realm_global_scene_urns.clear()
	realm_city_loader_content_base_url = ""
	realm_name = ""
	network_id = 0
	content_base_url = ""
	Global.scene_runner.kill_all_scenes()


## Private on purpose (#2948): a public string entry point is the door that let #2666
## reopen. What is left are the callers that are not navigations -- a portable
## experience's transient realm, and the offline scene renderer.
func _async_set_realm(new_realm_string: String) -> bool:
	var candidate_realm_url := Realm.normalize_realm_url(new_realm_string)

	prints("[REALM] _async_set_realm", new_realm_string, "resolved", candidate_realm_url)

	# Private worlds (#1725): refuse before emitting, fetching or mutating anything, so a
	# world the user can't enter never starts loading.
	var world_name := WorldPermissionsHelper.world_name_from_realm(
		new_realm_string, candidate_realm_url
	)
	if not world_name.is_empty() and not await WorldPermissionsHelper.async_is_allowed(world_name):
		# Expected, handled outcome (not an error) — use print so it does not ship to Sentry.
		print("[REALM] Access denied for private world: ", world_name)
		# Close the analytics loading episode without raising the generic failure toast.
		Global.scene_runner.loading_realm_change_failed(
			new_realm_string, "private_world_access_denied"
		)
		realm_access_denied.emit(new_realm_string, world_name)
		return false

	realm_changing.emit()

	var promise: Promise = Global.http_requester.request_json(
		candidate_realm_url + "about", HTTPClient.METHOD_GET, "", {}
	)

	var res = await PromiseUtils.async_awaiter(promise)
	if res is PromiseError:
		var reason: String = res.get_error()
		printerr(
			"[REALM] Rejected request change realm to: ",
			new_realm_string,
			" error message: ",
			reason
		)
		_emit_realm_change_failed(new_realm_string, reason)
		return false

	if not res is RequestResponse:
		_emit_realm_change_failed(new_realm_string, "invalid response")
		return false

	var response: RequestResponse = res
	Global.scene_runner.loading_mark_about_end()
	var json = response.get_string_response_as_json()
	# Checked BEFORE committing any realm state: a malformed `content` would otherwise throw
	# part-way through the commit, with no `realm_change_failed`, leaving a stuck loading screen.
	var about_problem := DclRealmRules.about_problem(json)
	if not about_problem.is_empty():
		printerr("[REALM] ", about_problem, " for ", new_realm_string)
		_emit_realm_change_failed(new_realm_string, about_problem)
		return false

	return await _async_commit_realm(new_realm_string, candidate_realm_url, json, false)


## No access gate and no /about fetch: the destination carries the one it was resolved
## with (#2948).
func async_apply_destination(dest: Destination) -> bool:
	if not dest.is_ready():
		push_error("Realm.async_apply_destination called with a %s destination" % dest)
		return false

	realm_changing.emit()
	# Empty unless the world is password-protected, which is what keeps `secret` off
	# the wire for every other realm (#2651).
	realm_credential = dest.credential
	# Only a join asks for the spawn point: a named parcel lands there, and a reload
	# leaves the player standing where they are.
	var wants_spawn := dest.is_intent and dest.target_parcel == Destination.UNSPECIFIED
	return await _async_commit_realm(
		dest.realm_string, dest.realm_url, dest.about, wants_spawn, dest.world_scenes
	)


## Commits validated realm state. Shared by the string entry point and by
## async_apply_destination, which prove `json` in different ways.
func _async_commit_realm(
	new_realm_string: String,
	candidate_realm_url: String,
	json: Dictionary,
	search_new_pos: bool,
	known_scenes: Dictionary = {}
) -> bool:
	realm_string = new_realm_string
	realm_url = candidate_realm_url
	realm_about = json

	var configuration: Dictionary = realm_about.get("configurations", {})

	# For DCL worlds, the /about endpoint only returns the last deployed scene.
	# Fetch all scenes from the /scenes endpoint to support multi-scene worlds.
	var world_scenes: Array = []
	var resolved_realm_name = configuration.get("realmName", "")
	if Realm.is_dcl_ens(resolved_realm_name):
		# Use the worlds content server /contents/ URL for baseUrl,
		# not content.publicUrl (which points to Genesis City peer)
		# worlds_content_server() returns ".../world/", we need ".../contents/"
		var worlds_base = DclUrls.worlds_content_server().replace("/world/", "/")
		var world_content_url = worlds_base + "contents/"
		# The resolve already read this listing; asking again is a second round trip.
		world_scenes = DestinationResolver.world_scene_urns(known_scenes, world_content_url)
		if world_scenes.is_empty():
			world_scenes = await _async_fetch_world_scenes(resolved_realm_name, world_content_url)

	var commit := DclRealmRules.commit_from_about(realm_about, world_scenes)

	realm_scene_urns.clear()
	for urn_data in commit.scene_urns:
		realm_scene_urns.push_back(urn_data)
	if commit.scenes_urn_override != null:
		# So scene_fetcher reads the full list from realm_about.
		configuration["scenesUrn"] = commit.scenes_urn_override

	realm_global_scene_urns.clear()
	for urn_data in commit.global_scene_urns:
		realm_global_scene_urns.push_back(urn_data)

	realm_city_loader_content_base_url = commit.city_loader_content_base_url
	self.set_lambda_server_base_url(commit.lambda_server_base_url)
	realm_name = commit.realm_name
	network_id = commit.network_id  # 1=Ethereum

	# A local preview realm advertises the parcels it serves locally. This is the only signal
	# available on the QR/deeplink preview path (`decentraland://open?preview=http://…`), which
	# never sets the `--preview` CLI flag — Pulse uses it to apply the same explicit-opt-in rule
	# there as on desktop, and to announce the LSD realm key instead of `LocalPreview`.
	Global.comms.set_local_scene_development(commit.serves_local_scenes)

	set_realm_min_bounds(commit.min_bounds)
	set_realm_max_bounds(commit.max_bounds)

	content_base_url = commit.content_base_url

	# For DCL worlds, always resolve spawn position from scene metadata
	# since the caller may not know the correct parcel coordinates.
	# For other realms, only resolve if explicitly requested.
	var should_resolve_pos = search_new_pos or Realm.is_dcl_ens(resolved_realm_name)
	if not realm_scene_urns.is_empty() and should_resolve_pos:
		await async_request_set_position(realm_scene_urns.back())

	Global.get_config().last_realm_joined = realm_url
	Global.get_config().save_to_settings_file()

	Global.metrics.update_realm(realm_url)

	_has_realm = true
	realm_changed.emit()
	return true


func _emit_realm_change_failed(new_realm_string: String, reason: String) -> void:
	Global.scene_runner.loading_realm_change_failed(new_realm_string, reason)
	realm_change_failed.emit(new_realm_string, reason)


func _async_fetch_world_scenes(world_name: String, base_content_url: String) -> Array:
	var scenes_url = Realm.dcl_world_url(world_name) + "/scenes"
	var promise: Promise = Global.http_requester.request_json(
		scenes_url, HTTPClient.METHOD_GET, "", {}
	)

	var res = await PromiseUtils.async_awaiter(promise)
	if res is PromiseError:
		printerr("[REALM] Failed to fetch world scenes: ", res.get_error())
		return []

	if res is RequestResponse:
		var response: RequestResponse = res
		var json = response.get_string_response_as_json()
		if json == null or not json is Dictionary:
			printerr("[REALM] Invalid response from /scenes endpoint")
			return []

		var result: Array = DclDestinationRules.world_scene_urns(json, base_content_url)
		if result.size() > 1:
			prints("[REALM] Loaded", result.size(), "scenes for world", world_name)

		return result

	return []


func async_request_set_position(scene_urn):
	prints(scene_urn)
	var url = scene_urn.baseUrl + scene_urn.entityId

	var promise: Promise = Global.http_requester.request_json(url, HTTPClient.METHOD_GET, "", {})

	var res = await PromiseUtils.async_awaiter(promise)
	if res is PromiseError:
		printerr(
			"Rejected request async_request_set_position: ",
			scene_urn,
			" error message: ",
			res.get_error()
		)
	elif res is RequestResponse:
		var response: RequestResponse = res
		var json: Dictionary = response.get_string_response_as_json()
		if json == null:
			printerr("do_request_json failed because json_string is not a valid json")
			return

		var base_pos = json.get("metadata", {}).get("scene", {}).get("base", "0,0")
		var coord = base_pos.split(",")
		var x = int(coord[0])
		var y = int(coord[1])
		var explorer = Global.get_explorer()
		if is_instance_valid(explorer):
			explorer.teleport_to(Vector2i(x, y))
