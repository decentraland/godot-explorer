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
## Same fetch shape as feature_flags.gd: fire-and-forget on _ready, raced against a timeout,
## fail-open. Unlike the flags there is deliberately NO disk cache: the window that decides
## whether a campaign is still live is evaluated server-side, so a cached map is a map that
## can resurrect a campaign the server has already retired — and the payload carries no end
## date for the client to re-check locally. The FTUE only ever runs on a first launch, where
## a cache is empty by definition, so it would buy almost nothing for that risk. Callers use
## async_resolve_pending(), which gives the in-flight fetch a bounded chance to land.
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

## Bounded wait for install attribution to produce a token (Android). The Rust resolver
## already gives GA4F 10s before falling back to the referrer; this covers the tail of that
## plus the referrer round trip, and expires quietly on an organic install, which never
## produces a token at all.
const ATTRIBUTION_MAX_WAIT_SECONDS := 12.0
const ATTRIBUTION_POLL_SECONDS := 0.5

var _campaigns: Dictionary = {}
var _loaded := false
# Whether the fetch actually came back. Separate from `_loaded`, which only says the attempt
# finished: a failed fetch must resolve as "resolver unavailable", not as "unknown token".
var _fetch_ok := false


func _ready() -> void:
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

	# An ad-driven install has no deeplink to carry the token: the app did not exist when the
	# ad was clicked. On Android the token arrives instead through install attribution (GA4F
	# deferred deep link, or the Play install referrer), which resolves asynchronously and can
	# still be in flight by the time the FTUE is reached.
	if config.campaign_token.is_empty():
		await _async_capture_attribution_token()

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

	if not _fetch_ok:
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


## Polls the Rust attribution resolver for a campaign token and captures it like a deeplink
## one, so everything downstream (freshness bound, consumption, resolution) is shared.
##
## Bounded: the resolver itself waits on GA4F before falling back to the referrer, and an
## organic install never produces a token at all — so this must not block the FTUE waiting
## for something that will never arrive.
func _async_capture_attribution_token() -> void:
	if not Global.is_android() or Global.metrics == null:
		return

	var deadline := Time.get_ticks_msec() + int(ATTRIBUTION_MAX_WAIT_SECONDS * 1000.0)
	while Time.get_ticks_msec() < deadline:
		var token := String(Global.metrics.get_resolved_campaign_token()).strip_edges()
		if not token.is_empty():
			var config := Global.get_config()
			config.campaign_token = token
			config.campaign_token_captured_at = int(Time.get_unix_time_from_system())
			config.save_to_settings_file()
			print("[CAMPAIGN] captured token from install attribution: ", token)
			return
		await get_tree().create_timer(ATTRIBUTION_POLL_SECONDS).timeout


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
		# Expected on offline/slow cold starts. The launch simply gets today's FTUE, so this
		# is not error-level (Sentry quota).
		push_warning("[Campaigns] fetch failed: " + str(result.get_error()))
	else:
		_campaigns = CampaignResolution.parse_response(result.get_string_response_as_json())
		_fetch_ok = true
		print("[Campaigns] loaded: ", _campaigns.keys())

	_loaded = true
	campaigns_loaded.emit()
