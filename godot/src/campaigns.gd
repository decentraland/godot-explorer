class_name Campaigns
extends Node

## Ad / referrer campaign resolver (issues #2670, #2669).
##
## An ad or referrer link carries an opaque token as `?c=<token>`. The token is NOT the
## destination: a deeplink carrying position/realm redirects straight to the explorer
## (see Lobby._should_go_to_explorer_from_deeplink), skipping the avatar creation flow the
## FTUE hangs off. The token is captured at boot and resolved here, against the map served
## by the mobile-bff, into either a personalized FTUE or a direct boot into the target.
##
## Same fetch shape as feature_flags.gd: fire-and-forget on _ready, raced against a
## timeout, fail-open. The difference is the cache — the FTUE only ever runs on a first
## launch, when there is no cached map yet, so a live map matters here and callers use
## async_resolve_pending() to give the in-flight fetch a bounded chance to land.
##
## Every failure path (no token, unknown token, expired token, already consumed, resolver
## unreachable) resolves to an empty campaign, which callers render as today's FTUE.

signal campaigns_loaded

const TIMEOUT_SECONDS := 5.0

## Bounded wait callers grant an in-flight fetch before falling back (the FTUE is reached
## tens of seconds after boot, so this is a backstop, not the normal path).
const RESOLVE_MAX_WAIT_SECONDS := 3.0

## A captured token stops being honored after this. The Play install referrer survives for
## 90 days and only changes on reinstall, so without a freshness bound a reinstall months
## later would replay a long-dead campaign.
const TOKEN_MAX_AGE_SECONDS := 7 * 86400

var _campaigns: Dictionary = {}
var _loaded := false


func _ready() -> void:
	# Last-known-good map first, so a resolve can answer before the network does.
	_campaigns = _parse_cache(Global.get_config().campaigns_cache)
	_async_load.call_deferred()


func is_loaded() -> bool:
	return _loaded


## Resolves a token against the current map. Returns {} when the token is unknown — which
## includes a campaign the server disabled or whose window closed, since the map only ever
## carries active ones.
func resolve(token: String) -> Dictionary:
	if token.is_empty():
		return {}
	var campaign = _campaigns.get(token)
	if typeof(campaign) != TYPE_DICTIONARY:
		return {}
	return campaign


## Resolves the token captured at boot into the campaign that should drive this launch.
##
## Returns {"token": String, "campaign": Dictionary, "fallback_reason": String}. The
## campaign is empty on every failure path, and `fallback_reason` says which one it was.
func async_resolve_pending() -> Dictionary:
	var config := Global.get_config()
	var token: String = config.campaign_token

	if token.is_empty():
		return _fallback("", CampaignResolution.FALLBACK_NO_TOKEN)

	if config.campaign_consumed:
		return _fallback(token, CampaignResolution.FALLBACK_ALREADY_CONSUMED)

	var age: int = int(Time.get_unix_time_from_system()) - config.campaign_token_captured_at
	if config.campaign_token_captured_at <= 0 or age > TOKEN_MAX_AGE_SECONDS:
		return _fallback(token, CampaignResolution.FALLBACK_EXPIRED_TOKEN)

	if not _loaded:
		await _async_wait_for_load()

	if not _loaded and _campaigns.is_empty():
		return _fallback(token, CampaignResolution.FALLBACK_RESOLVER_UNAVAILABLE)

	var campaign := resolve(token)
	if campaign.is_empty():
		return _fallback(token, CampaignResolution.FALLBACK_UNKNOWN_TOKEN)

	return {
		"token": token, "campaign": campaign, "fallback_reason": CampaignResolution.FALLBACK_NONE
	}


## One campaign per install: once a launch has acted on the token (personalized FTUE shown
## or target booted), it never runs again.
func mark_consumed() -> void:
	var config := Global.get_config()
	if config.campaign_consumed:
		return
	config.campaign_consumed = true
	config.save_to_settings_file()


func _fallback(token: String, reason: String) -> Dictionary:
	return {"token": token, "campaign": {}, "fallback_reason": reason}


func _async_wait_for_load() -> void:
	var loaded_fn := func() -> Promise:
		var p := Promise.new()
		if _loaded:
			p.resolve()
		else:
			campaigns_loaded.connect(func(): p.resolve(), CONNECT_ONE_SHOT)
		return p
	var timeout_fn := func() -> Promise:
		var p := Promise.new()
		get_tree().create_timer(RESOLVE_MAX_WAIT_SECONDS).timeout.connect(
			func(): p.reject("campaigns: resolve wait timeout")
		)
		return p

	await PromiseUtils.async_race([loaded_fn, timeout_fn])


func _async_load() -> void:
	var url := String(DclUrls.campaigns())
	var http_fn := func() -> Promise:
		return Global.http_requester.request_json(url, HTTPClient.METHOD_GET, "", {})
	var timeout_fn := func() -> Promise:
		var p := Promise.new()
		get_tree().create_timer(TIMEOUT_SECONDS).timeout.connect(
			func(): p.reject("campaigns: timeout")
		)
		return p

	var result = await PromiseUtils.async_race([http_fn, timeout_fn])
	if result is PromiseError:
		# Expected on offline/slow cold starts — the cached map (if any) stays in place
		# and an unresolved token just renders today's FTUE, so this is not error-level.
		push_warning("[Campaigns] fetch failed (keeping cache): " + str(result.get_error()))
	else:
		var fetched := CampaignResolution.parse_response(result.get_string_response_as_json())
		_campaigns = fetched
		_store_cache(fetched)
		print("[Campaigns] loaded: ", fetched.keys())

	_loaded = true
	campaigns_loaded.emit()


func _store_cache(campaigns: Dictionary) -> void:
	var config := Global.get_config()
	config.campaigns_cache = JSON.stringify(campaigns)
	config.save_to_settings_file()


func _parse_cache(raw: String) -> Dictionary:
	if raw.is_empty():
		return {}
	var parsed = JSON.parse_string(raw)
	if typeof(parsed) != TYPE_DICTIONARY:
		return {}
	return parsed
