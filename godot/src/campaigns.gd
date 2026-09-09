class_name Campaigns
extends Node

## Ad / referrer campaign resolver (issues #2670, #2669).
##
## An ad link carries an opaque token as `?c=<token>`, captured at boot and resolved here
## against the map served by the mobile-bff into the target the install boots into. The token
## is NOT the destination: a deeplink carrying position/realm would redirect straight to the
## explorer, skipping the avatar creation flow the FTUE hangs off.
##
## Fetched lazily, raced against a timeout, fail-open — every failure path resolves to an
## empty campaign, which callers render as today's FTUE. No disk cache on purpose: a campaign
## is retired by deleting it, so a cached map can still resolve a token the server no longer
## knows, and the FTUE only runs on a first launch where a cache is empty anyway.

signal campaigns_loaded

const TIMEOUT_SECONDS := 5.0

## Must outlast TIMEOUT_SECONDS: a shorter wait would give up on a fetch still within its own
## timeout, losing the campaign on the slow cold starts where the map arrives a second later.
const RESOLVE_MAX_WAIT_SECONDS := TIMEOUT_SECONDS + 1.0

## Freshness bound. The Play install referrer survives 90 days and only changes on reinstall,
## so without one a reinstall months later would replay a long-dead campaign.
const TOKEN_MAX_AGE_SECONDS := 7 * 86400

## Bounded wait for install attribution to settle (Android). The Rust resolver bounds itself
## (10s for GA4F, 25s overall), so this only has to outlast that.
const ATTRIBUTION_MAX_WAIT_SECONDS := 30.0
const ATTRIBUTION_POLL_SECONDS := 0.5

var _campaigns: Dictionary = {}
var _loaded := false
# Whether the fetch actually came back. Separate from `_loaded`, which only says the attempt
# finished: a failed fetch must resolve as "resolver unavailable", not as "unknown token".
var _fetch_ok := false
# Whether install attribution has finished resolving, with or without a token. Distinct from
# "there is a token": an organic install settles with none, and waiting on the token alone
# would stall every campaign-less launch for the whole timeout.
var _attribution_settled := false


func _ready() -> void:
	_async_watch_attribution.call_deferred()


func is_loaded() -> bool:
	return _loaded


## Resolves a token against the current map. {} when unknown, which includes a deleted
## campaign — the BFF has no enabled flag, so deletion is how one is retired.
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

	# An ad-driven install has no deeplink — the app did not exist when the ad was clicked — so
	# the token arrives through install attribution, which can still be in flight here.
	if config.campaign_token.is_empty():
		await _async_wait_for_attribution()

	var token: String = config.campaign_token

	if token.is_empty():
		return _fallback("", CampaignResolution.FALLBACK_NO_TOKEN)

	if config.campaign_consumed:
		return _fallback(token, CampaignResolution.FALLBACK_ALREADY_CONSUMED)

	var age: int = int(Time.get_unix_time_from_system()) - config.campaign_token_captured_at
	if config.campaign_token_captured_at <= 0 or age > TOKEN_MAX_AGE_SECONDS:
		return _fallback(token, CampaignResolution.FALLBACK_EXPIRED_TOKEN)

	# Lazy: fetching on every boot would put every install on the campaigns endpoint.
	if not _loaded:
		_async_load.call_deferred()
		await _async_wait_for_load()

	if not _fetch_ok:
		return _fallback(token, CampaignResolution.FALLBACK_RESOLVER_UNAVAILABLE)

	var campaign := resolve(token)
	if campaign.is_empty():
		return _fallback(token, CampaignResolution.FALLBACK_UNKNOWN_TOKEN)

	return {
		"token": token, "campaign": campaign, "fallback_reason": CampaignResolution.FALLBACK_NONE
	}


## One campaign per install: once a launch has booted into the target, it never runs again.
func mark_consumed() -> void:
	var config := Global.get_config()
	if config.campaign_consumed:
		return
	config.campaign_consumed = true
	config.save_to_settings_file()


func _fallback(token: String, reason: String) -> Dictionary:
	return {"token": token, "campaign": {}, "fallback_reason": reason}


## Persists the token install attribution resolves to, as soon as it resolves.
##
## Runs from _ready, not from the FTUE: the resolved token lives only in Rust memory and
## attribution starts once per install, so a process killed before the FTUE is reached would
## lose it for good. Residual gap: a kill before attribution settles still loses it.
func _async_watch_attribution() -> void:
	if OS.get_name() != "Android" or Global.metrics == null:
		_attribution_settled = true
		return

	var deadline := Time.get_ticks_msec() + int(ATTRIBUTION_MAX_WAIT_SECONDS * 1000.0)
	while true:
		var token := String(Global.metrics.get_resolved_campaign_token()).strip_edges()
		if not token.is_empty():
			var config := Global.get_config()
			# The deeplink path wins if it already captured one: it is the more specific
			# signal, and _capture_campaign_token never replaces a stored token either.
			if config.campaign_token.is_empty():
				config.campaign_token = token
				config.campaign_token_captured_at = int(Time.get_unix_time_from_system())
				config.save_to_settings_file()
				print("[CAMPAIGN] captured token from install attribution: ", token)
			_attribution_settled = true
			return
		if not Global.metrics.is_install_attribution_pending():
			_attribution_settled = true
			return
		if Time.get_ticks_msec() >= deadline:
			push_warning("[CAMPAIGN] install attribution never settled")
			_attribution_settled = true
			return
		await get_tree().create_timer(ATTRIBUTION_POLL_SECONDS).timeout


## Bounded wait for the watcher above, for callers that need the answer now.
func _async_wait_for_attribution() -> void:
	var deadline := Time.get_ticks_msec() + int(ATTRIBUTION_MAX_WAIT_SECONDS * 1000.0)
	while not _attribution_settled and Time.get_ticks_msec() < deadline:
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
