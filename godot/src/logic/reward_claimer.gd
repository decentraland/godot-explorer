class_name RewardClaimer
extends RefCounted

## Pure claim logic shared by reward_modal.gd and the streak modal (issue #3018) — no UI, no node
## state. Client-side equivalent of Genesis' claimToken/requestToken: a signed POST to the rewards
## server that assigns the wearable to the player's wallet.

## Two-minute window (in seconds) used to tell a fresh claim apart from an item the
## wallet already owned, mirroring Genesis' claim.ts `assigned_at` check.
const ALREADY_CLAIMED_WINDOW_SEC := 120.0

## Why a claim failed, as the `reason` of async_claim's result, mapped to the plain-language copy
## the player sees. Also the reason reported to metrics and Sentry.
const REASON_MESSAGE_KEYS := {
	"wallet_required": "REWARD_ERROR_WALLET_REQUIRED",
	"not_available": "REWARD_NOT_AVAILABLE",
	"already_claimed": "REWARD_ALREADY_OWNED",
	"out_of_stock": "REWARD_OUT_OF_STOCK",
	"nothing_granted": "REWARD_NOTHING_GRANTED",
	"session_unverified": "REWARD_SESSION_UNVERIFIED",
	"network": "REWARD_ERROR_SERVER_UNREACHABLE",
	"server_error": "REWARD_ERROR_INVALID_RESPONSE",
	"rejected": "REWARD_ERROR_REJECTED",
	"unknown": "REWARD_CLAIM_FAILED",
}

## One Sentry event per campaign + reason per session; every attempt still goes to metrics.
static var _sentry_reported := {}


## @param campaign: { campaign_id, campaign_key, assign_target?, assign_value? } — see
## RewardCampaigns.CAMPAIGNS.
## @return { ok: bool, already_owned: bool, reason: String, error_message: String, code: String }
## — reason/error_message/code are only set when ok is false; already_owned only matters when ok
## is true. `reason` is a REASON_MESSAGE_KEYS key and error_message its translated copy; `code` is
## the raw rewards-server detail behind it, for QA and logs only.
static func async_claim(campaign: Dictionary) -> Dictionary:
	var started_ms := Time.get_ticks_msec()
	var result := await _async_claim(campaign)
	_track(campaign, result, Time.get_ticks_msec() - started_ms)
	return result


## True for a thirdweb guest with nothing linked. The cached "upgraded" flag is only known once
## thirdweb has been asked this session, so ask before sending an upgraded guest to Add Email.
static func async_is_unlinked_guest() -> bool:
	var identity = Global.player_identity
	if identity == null or not identity.is_thirdweb_guest():
		return false
	if identity.is_thirdweb_guest_upgraded():
		return false
	var result = await PromiseUtils.async_awaiter(
		identity.async_refresh_thirdweb_upgrade_state(Global.get_device_anchor_id())
	)
	if result is PromiseError:
		return not identity.is_thirdweb_guest_upgraded()
	return not bool(result)


static func _async_claim(campaign: Dictionary) -> Dictionary:
	var beneficiary := Global.player_identity.get_address_str()
	if beneficiary.is_empty():
		return _error("wallet_required", "wallet_required")

	var campaign_id := str(campaign.get("campaign_id", ""))
	var campaign_key := str(campaign.get("campaign_key", ""))
	if campaign_id.is_empty() or campaign_key.is_empty():
		return _error("not_available", "campaign_not_configured")

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
	# Campaigns that grant from a pool of possible items (vs. a single fixed one) need to say which
	# — the server rejects the claim with "Missing parameters assign_target and assign_value or
	# assign_group" otherwise. Omitted entirely when the campaign doesn't set them.
	var assign_target := str(campaign.get("assign_target", ""))
	var assign_value := str(campaign.get("assign_value", ""))
	if not assign_target.is_empty() and not assign_value.is_empty():
		body_dict["assign_target"] = assign_target
		body_dict["assign_value"] = assign_value
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
			# An HTML error page means the server answered badly; anything else never reached it.
			var answered := reason.strip_edges().begins_with("<")
			return _error("server_error" if answered else "network", "server_unreachable:" + reason)
	else:
		var raw_response := str(response.get_response_as_string())
		json = response.get_string_response_as_json()
		if not (json is Dictionary):
			return _error("server_error", "invalid_response:" + raw_response)

	return _handle_claim_json(json)


## Interprets a parsed rewards-server JSON body, mirroring claim.ts `processResponse`.
static func _handle_claim_json(json: Dictionary) -> Dictionary:
	var ok = json.get("ok", null)
	var code := str(json.get("code", ""))

	if ok == false:
		# A 400 is the server refusing the request itself (e.g. "Missing parameters"): a setup
		# problem on our side, not something the player can change.
		if code == "bad_request" or (code.is_empty() and json.has("error")):
			return _error("rejected", "rejected:" + str(json.get("error", code)))
		return _error(_reason_for_code(code), code)

	var data = json.get("data", [])
	# ok but nothing granted: the key's per-wallet limit was already used or the stock ran out —
	# the server doesn't say which, so the copy covers both.
	if not (data is Array) or data.is_empty():
		return _error("nothing_granted", "empty_data:" + str(json))

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
			return {"ok": true, "already_owned": true, "error_message": "", "code": ""}

	return {"ok": true, "already_owned": false, "error_message": "", "code": ""}


## Maps a rewards-server error code to a REASON_MESSAGE_KEYS reason, so a raw code (e.g.
## "catalyst_unreachable") never reaches the UI.
static func _reason_for_code(code: String) -> String:
	match code:
		"campaign_key_uninitiated", "campaign_disabled", "campaign_finished":
			return "not_available"
		"campaign_not_found", "campaign_key_not_found", "campaign_not_owner":
			return "not_available"
		"already_claimed", "user_already_claimed":
			return "already_claimed"
		"supply_reached", "out_of_stock":
			return "out_of_stock"
		"catalyst_invalid", "catalyst_unreachable", "user_address_not_connected", "user_address_position":
			return "session_unverified"
	return "unknown"


static func _error(reason: String, code: String = "") -> Dictionary:
	var message_key: String = REASON_MESSAGE_KEYS.get(reason, "REWARD_CLAIM_FAILED")
	return {
		"ok": false,
		"already_owned": false,
		"reason": reason,
		"error_message": TranslationServer.translate(message_key),
		"code": code,
	}


## Every attempt goes to metrics (Request Result, context "reward_claim"); a failure also goes to
## Sentry once per campaign + reason per session, outside the remote sample rate (event_kind
## "reward_claim" in ProjectMainLoop._before_send), tagged with the wallet that failed.
static func _track(campaign: Dictionary, result: Dictionary, duration_ms: int) -> void:
	var campaign_name := RewardCampaigns.name_of(str(campaign.get("campaign_id", "")))
	var reason := str(result.get("reason", ""))
	var identity = Global.player_identity
	var guest_state := "wallet"
	if identity != null and identity.is_thirdweb_guest():
		guest_state = "upgraded_guest" if identity.is_thirdweb_guest_upgraded() else "guest"
	if Global.metrics != null:
		(
			Global
			. metrics
			. track_request_result(
				"reward_claim",
				RewardCampaigns.get_rewards_server() + "/api/rewards",
				"POST",
				result.ok,
				0,
				reason,
				duration_ms,
				(
					JSON
					. stringify(
						{
							"campaign": campaign_name,
							"already_owned": result.already_owned,
							"account": guest_state,
						}
					)
				)
			)
		)
	if result.ok:
		return
	printerr("[RewardClaimer] claim failed: ", campaign_name, " reason=", reason, " ", result.code)
	var report_key := campaign_name + ":" + reason
	if reason == "already_claimed" or _sentry_reported.has(report_key):
		return
	if not SentrySDK.is_enabled():
		return
	_sentry_reported[report_key] = true
	var event := SentrySDK.create_event()
	event.level = SentrySDK.LEVEL_ERROR
	event.message = "Reward claim failed: %s: %s" % [campaign_name, reason]
	event.set_fingerprint(PackedStringArray(["reward-claim-failed", campaign_name, reason]))
	event.set_tag("event_kind", "reward_claim")
	event.set_tag("reward_campaign", campaign_name)
	event.set_tag("reward_reason", reason)
	event.set_tag("reward_account", guest_state)
	event.set_tag("wallet", identity.get_address_str() if identity != null else "")
	(
		event
		. set_context(
			"reward_claim",
			{
				"campaign": campaign_name,
				"campaign_id": str(campaign.get("campaign_id", "")),
				"reason": reason,
				"detail": str(result.code).left(500),
				"duration_ms": duration_ms,
			}
		)
	)
	SentrySDK.capture_event(event)
