class_name RewardClaimer
extends RefCounted

## Pure claim logic shared by reward_modal.gd and the streak modal (issue #3018) — no UI, no node
## state. Client-side equivalent of Genesis' claimToken/requestToken: a signed POST to the rewards
## server that assigns the wearable to the player's wallet.

## Two-minute window (in seconds) used to tell a fresh claim apart from an item the
## wallet already owned, mirroring Genesis' claim.ts `assigned_at` check.
const ALREADY_CLAIMED_WINDOW_SEC := 120.0


## @param campaign: { campaign_id, campaign_key } — see RewardCampaigns.CAMPAIGNS.
## @return { ok: bool, already_owned: bool, error_message: String } — error_message is only set
## when ok is false; already_owned only matters when ok is true.
static func async_claim(campaign: Dictionary) -> Dictionary:
	var beneficiary := Global.player_identity.get_address_str()
	if beneficiary.is_empty():
		return _error(TranslationServer.translate("REWARD_ERROR_WALLET_REQUIRED"))

	var campaign_id := str(campaign.get("campaign_id", ""))
	var campaign_key := str(campaign.get("campaign_key", ""))
	if campaign_id.is_empty() or campaign_key.is_empty():
		return _error(TranslationServer.translate("REWARD_NOT_AVAILABLE"))

	# The rewards server matches `catalyst` by ORIGIN against the DAO allow-list and then actually
	# reaches it — must be a REAL reachable catalyst peer, NOT the realm-provider load balancer.
	# content_base_url resolves to the connected peer; the fallback (claiming outside a realm) uses
	# a concrete DAO peer, never main_realm() — its origin passes the allow-list but can't be reached.
	var catalyst := _catalyst_origin(str(Global.realm.content_base_url))
	if catalyst.is_empty():
		catalyst = _catalyst_origin(Realm.DAO_SERVERS[0])
	var url := RewardCampaigns.get_rewards_server() + "/api/rewards?campaign_id=" + campaign_id
	var body_dict := {
		"campaign_key": campaign_key,
		"catalyst": catalyst,
		"beneficiary": beneficiary,
	}
	var body := JSON.stringify(body_dict)
	print("[RewardClaimer] claim POST ", url, " catalyst=", catalyst, " beneficiary=", beneficiary)

	var response = await Global.async_signed_fetch(url, HTTPClient.METHOD_POST, body)
	return _process_claim_response(response)


## Reduces a URL to its origin (scheme://host), dropping any path/query. The rewards server's
## catalyst allow-list matches by origin, so a trailing "/main/" or "/content/" gets rejected.
## Returns "" for an empty input so the caller's fallback chain keeps working.
static func _catalyst_origin(url: String) -> String:
	if url.is_empty():
		return ""
	var scheme := "https://"
	var rest := url
	if url.begins_with("https://"):
		rest = url.substr(scheme.length())
	elif url.begins_with("http://"):
		scheme = "http://"
		rest = url.substr(scheme.length())
	var slash := rest.find("/")
	if slash != -1:
		rest = rest.substr(0, slash)
	if rest.is_empty():
		return ""
	return scheme + rest


## Interprets the rewards-server response, mirroring claim.ts `processResponse`.
static func _process_claim_response(response) -> Dictionary:
	var json
	if response is PromiseError:
		# request_json rejects on any non-2xx, but the rewards server still returns a JSON error
		# body (e.g. {"ok":false,"code":"campaign_disabled"}). Parse it so business errors get a
		# friendly message instead of raw JSON; a non-JSON reason is a real transport error.
		var reason := str(response.get_error())
		printerr("RewardClaimer: claim failed — ", reason)
		json = JSON.parse_string(reason)
		if not (json is Dictionary):
			return _error(TranslationServer.translate("REWARD_ERROR_SERVER_UNREACHABLE"))
	else:
		json = response.get_string_response_as_json()
		if not (json is Dictionary):
			return _error(TranslationServer.translate("REWARD_ERROR_INVALID_RESPONSE"))

	return _handle_claim_json(json)


## Interprets a parsed rewards-server JSON body, mirroring claim.ts `processResponse`.
static func _handle_claim_json(json: Dictionary) -> Dictionary:
	var ok = json.get("ok", null)
	var code := str(json.get("code", ""))

	# Campaign not currently active (uninitiated / disabled / finished).
	if (
		ok == false
		and code in ["campaign_key_uninitiated", "campaign_disabled", "campaign_finished"]
	):
		return _error(TranslationServer.translate("REWARD_NOT_AVAILABLE"))

	# Any other non-ok response: friendly copy, never the raw code (already logged above).
	if ok == false:
		return _error(_friendly_claim_error(code))

	var data = json.get("data", [])
	# ok but nothing granted → out of stock.
	if not (data is Array) or data.is_empty():
		return _error(TranslationServer.translate("REWARD_OUT_OF_STOCK"))

	# Already owned: assigned more than the window ago (a fresh claim echoes back a recent date).
	var item = data[0]
	var assigned_at = item.get("assigned_at", null)
	if assigned_at != null:
		# Godot's ISO parser chokes on fractional seconds / trailing 'Z'; keep YYYY-MM-DDTHH:MM:SS.
		var assigned_unix := Time.get_unix_time_from_datetime_string(str(assigned_at).substr(0, 19))
		if (
			assigned_unix > 0
			and assigned_unix < Time.get_unix_time_from_system() - ALREADY_CLAIMED_WINDOW_SEC
		):
			return {"ok": true, "already_owned": true, "error_message": ""}

	return {"ok": true, "already_owned": false, "error_message": ""}


## Maps a rewards-server error code to user-facing copy. Unknown codes fall through to a
## generic message so a raw code (e.g. "catalyst_unreachable") never reaches the UI — the
## real code is still logged by _process_claim_response for debugging.
static func _friendly_claim_error(code: String) -> String:
	match code:
		"campaign_not_found", "campaign_key_not_found", "campaign_not_owner":
			return TranslationServer.translate("REWARD_NOT_AVAILABLE")
		"already_claimed", "user_already_claimed":
			return TranslationServer.translate("REWARD_ALREADY_OWNED")
		"supply_reached", "out_of_stock":
			return TranslationServer.translate("REWARD_OUT_OF_STOCK")
		"catalyst_invalid", "catalyst_unreachable", "user_address_not_connected", "user_address_position":
			return TranslationServer.translate("REWARD_SESSION_UNVERIFIED")
	return TranslationServer.translate("REWARD_CLAIM_FAILED")


static func _error(message: String) -> Dictionary:
	return {"ok": false, "already_owned": false, "error_message": message}
