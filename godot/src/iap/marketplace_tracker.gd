extends Node

# Tracks a round-trip to the web marketplace (opened in an in-app browser) and, on
# return, polls the owned-wearables catalog to detect a purchase delivered to the
# inventory: a NEW owned urn (vs the pre-purchase baseline) means the item minted, and
# we emit `item_arrived` so the backpack can surface it live.
#
# NOTE: an in-app toast used to be shown on arrival (tap → open backpack + filter).
# It was removed because the toast UI isn't laid out for portrait. See the
# MARKETPLACE-IAP-TOAST markers here + in notifications_manager.gd / menu.gd for where
# to re-hook a portrait-aware toast. `item_arrived` still fires for the live auto-focus.
#
# Driven by open_and_track(), which arms the tracker right before the browser
# opens so we capture the pre-purchase baseline. Return is detected on iOS via the
# native DclGodotiOS.webview_closed signal (the in-app SFSafariViewController fires
# no reliable focus notification — a swipe-to-dismiss of the page sheet never resigns
# the app). On other platforms the browser is external, so we fall back to the app
# focus notification (see _ready / _notification).

# Emitted when a purchased wearable is detected as genuinely owned (the "arrived"
# moment). The backpack connects to this to refresh its inventory live, since the
# mint can land ~2 min after the purchase, well after the user left the webview.
# `category` is the marketplace category of the arrived item ("wearable" or
# "emote"), so the backpack can route it to the right list (wearable grid vs the
# emote editor) instead of dumping every arrival into the wearables grid.
signal item_arrived(urn: String, category: String)

enum State { IDLE, ARMED, POLLING }

# MARKETPLACE-IAP-TOAST (removed — pending a portrait-compatible toast):
# On each arrival we used to call NotificationsManager.show_system_toast(title, body,
# "marketplace_iap", "default", {"category": category, "urn": urn}); a tap routed to
# the backpack with the right tab + Collectibles filter (see menu.gd). We also showed
# an "on the way" toast on the first credit drop. To restore: re-add the toast call in
# _async_check where item_arrived is emitted, and the click routing in menu.gd.

# Log prefix; all lines grep-able as `[MktTracker]` on the --log-stream.
const _LOG := "[MktTracker]"

# Polling schedule, measured from when polling starts (1s after returning):
# Once armed we poll at a STEADY 5s cadence for the full window and do NOT stop on
# the first arrival — several items can still be minting — nor give up early when no
# credit drop is seen. The on-chain mint can lag minutes on testnet, so the window is
# kept generous (5 min). Each new urn fires its own toast + item_arrived as it lands.
const _INITIAL_DELAY_SEC := 1.0
const _POLL_INTERVAL_SEC := 5.0
const _MAX_TOTAL_POLL_SEC := 300.0
# The wearable baseline must be a reliable snapshot — comparisons are meaningless
# without it — so retry the initial fetch a few times before giving up.
const _BASELINE_FETCH_ATTEMPTS := 5
const _BASELINE_RETRY_SEC := 2.0
# A just-bought item is always the newest in its category, so we only need the top
# of each "recently added" list to spot it — no point pulling the whole inventory.
const _OWNED_FETCH_LIMIT := 20
# Categories the IAP marketplace can deliver into the backpack.
const _OWNED_CATEGORIES: PackedStringArray = ["wearable", "emote"]

var _state: State = State.IDLE
# Bumped on every arm()/stop() so a stale baseline capture or polling loop, which
# can't be cancelled mid-await, becomes a no-op when it resumes.
var _token: int = 0
var _baseline_urns: Dictionary = {}
var _baseline_ready: bool = false
# True once we've hooked the native DclGodotiOS.webview_closed signal (iOS). When
# set, the app-focus fallback in _notification is ignored — the in-app Safari fires
# no reliable focus notification (especially on a swipe-to-dismiss of the page
# sheet), so the native dismissal callback is the source of truth.
var _use_webview_signal: bool = false


func _ready() -> void:
	# iOS: the in-app SFSafariViewController fires no reliable focus/lifecycle
	# notification on dismissal, so hook the native dismissal signal. Other platforms
	# open an external browser where app focus works → fall back to _notification.
	if Engine.has_singleton("DclGodotiOS"):
		var ios := Engine.get_singleton("DclGodotiOS")
		if ios.has_signal("webview_closed"):
			ios.connect("webview_closed", _on_webview_closed)
			_use_webview_signal = true
		else:
			printerr(
				_LOG,
				" WARNING: DclGodotiOS has no webview_closed (rebuild plugin) — focus fallback"
			)


# Native dismissal of the in-app browser (iOS). Fires for every open_webview_url
# dismissal regardless of how it was closed; we only act if we armed it.
func _on_webview_closed() -> void:
	if _state == State.ARMED:
		_state = State.POLLING
		_async_poll(_token)


# Single entry point: arm the tracker (snapshot the pre-purchase baseline) and
# open the web marketplace in the in-app browser. `raw_url` is a plain marketplace
# URL — the mobile-IAP view flag is appended here so every entry point matches.
func open_and_track(raw_url: String) -> void:
	_arm()
	Global.open_webview_url(MarketplaceUrl.with_mobile_iap(raw_url))


# Capture the pre-purchase baseline (credits + owned wearables). No-op without a
# wallet, since both the credits balance and the wearable catalog are per-wallet.
func _arm() -> void:
	var wallet := ""
	if Global.player_identity != null:
		wallet = Global.player_identity.get_address_str()
	if wallet.is_empty():
		return
	_token += 1
	_state = State.ARMED
	_baseline_ready = false
	_async_capture_baseline(_token)


func stop() -> void:
	_token += 1
	_state = State.IDLE


# App-focus fallback for non-iOS (external browser): the native webview_closed
# signal handles iOS. Ignored when that signal is hooked.
func _notification(what: int) -> void:
	if _use_webview_signal:
		return
	if what == NOTIFICATION_APPLICATION_FOCUS_IN and _state == State.ARMED:
		_state = State.POLLING
		_async_poll(_token)


# gdlint:ignore = async-function-name
func _async_capture_baseline(token: int) -> void:
	# Retry the wearable snapshot until it succeeds; a failed (null) baseline would
	# make every existing wearable look "new" on the first poll. Bounded by the
	# token (re-arm/stop) so it can't spin forever.
	for _attempt in range(_BASELINE_FETCH_ATTEMPTS):
		var urns = await _async_fetch_owned_urns()
		if token != _token:
			return
		if urns != null:
			_baseline_urns = urns
			_baseline_ready = true
			return
		await get_tree().create_timer(_BASELINE_RETRY_SEC).timeout
		if token != _token:
			return
	printerr(_LOG, " could not capture wearable baseline; tracking disabled")


# gdlint:ignore = async-function-name
func _async_poll(token: int) -> void:
	await get_tree().create_timer(_INITIAL_DELAY_SEC).timeout
	if token != _token:
		return
	var elapsed := 0.0
	while token == _token:
		await _async_check(token)
		if token != _token:
			return
		# Keep polling the full window regardless of arrivals — more items may still be
		# minting. We only stop at the safety cap.
		if elapsed >= _MAX_TOTAL_POLL_SEC:
			stop()
			return
		await get_tree().create_timer(_POLL_INTERVAL_SEC).timeout
		elapsed += _POLL_INTERVAL_SEC


# Detects every newly-owned item since the baseline and emits item_arrived for each
# one exactly once. Non-terminal: it never stops the poll, so multiple items minting
# over several minutes are all caught. Each detected urn is folded into the baseline so
# it isn't re-reported on the next tick.
# gdlint:ignore = async-function-name
func _async_check(token: int) -> void:
	if not _baseline_ready:
		return

	var urns = await _async_fetch_owned_urns()
	if token != _token:
		return
	if urns == null:
		return
	for urn in urns:
		if not _baseline_urns.has(urn):
			var category: String = urns[urn]
			# Fold into the baseline so the next tick doesn't re-report it.
			_baseline_urns[urn] = category
			# MARKETPLACE-IAP-TOAST: a clickable arrival toast was shown here (see the
			# marker near the top of this file). Removed pending a portrait-aware toast.
			item_arrived.emit(urn, category)


# Public: the most-recently-obtained owned urns of one category ("wearable"/"emote"),
# as a Dictionary keyed by token-instance urn, or an empty dict on failure / no wallet.
# The backpack and emote editor call this to surface a just-bought item on open — fast
# (marketplace API, top _OWNED_FETCH_LIMIT), without waiting for the catalyst lambda.
# Augments the lambda list; never the sole source.
# gdlint:ignore = async-function-name
func async_fetch_recent_owned(category: String) -> Dictionary:
	var wallet := ""
	if Global.player_identity != null:
		wallet = Global.player_identity.get_address_str()
	if wallet.is_empty():
		return {}
	var result = await _async_fetch_owned_urns_for(wallet, category)
	return result if result != null else {}


# Returns a Dictionary of the player's most-recently-added owned wearables + emotes,
# keyed by token-instance urn with the marketplace category ("wearable"/"emote") as
# the value, or null on a fetch failure — callers must treat null as "unknown", never
# as "empty". A failure in ANY category fails the whole snapshot, since a missing
# category would otherwise look like "nothing owned there" and break the baseline
# comparison.
# gdlint:ignore = async-function-name
func _async_fetch_owned_urns():
	var wallet := ""
	if Global.player_identity != null:
		wallet = Global.player_identity.get_address_str()
	if wallet.is_empty():
		return null
	var owned := {}
	for category in _OWNED_CATEGORIES:
		var partial = await _async_fetch_owned_urns_for(wallet, category)
		if partial == null:
			return null
		owned.merge(partial)
	return owned


# Fetches the _OWNED_FETCH_LIMIT most-recently-added owned NFTs of one category, as a
# Dictionary keyed by token-instance urn with `category` as the value, or null on a
# transport/shape error.
#
# Uses the marketplace API (subgraph-backed) the web's "My Assets" uses — it
# reflects a mint in ~seconds, vs the catalyst lambda's minutes. No auth needed for
# a public owner read. The API returns item-level urns + tokenId separately; we
# rebuild the catalyst/backpack token-instance urn (`<item_urn>:<tokenId>`) so the
# urn matches what the backpack/avatar use. `sortBy=newest` orders by creation date
# (most recent first), so the just-bought item lands at the top.
# gdlint:ignore = async-function-name
func _async_fetch_owned_urns_for(wallet: String, category: String):
	var url := (
		"%s/v1/nfts?first=%d&skip=0&sortBy=newest&category=%s&owner=%s"
		% [DclUrls.marketplace_api(), _OWNED_FETCH_LIMIT, category, wallet]
	)
	var promise: Promise = Global.http_requester.request_json(url, HTTPClient.METHOD_GET, "", {})
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		printerr(_LOG, " owned-nfts fetch error (", category, "): ", result.get_error())
		return null
	var json = result.get_string_response_as_json()
	if not (json is Dictionary):
		return null
	var data = json.get("data", null)
	if not (data is Array):
		return null
	var owned := {}
	for entry in data:
		if not (entry is Dictionary):
			continue
		var nft = entry.get("nft", null)
		if not (nft is Dictionary):
			continue
		var item_urn := str(nft.get("urn", ""))
		if item_urn.is_empty():
			continue
		var token_id := str(nft.get("tokenId", ""))
		var urn := item_urn + ":" + token_id if not token_id.is_empty() else item_urn
		owned[urn] = category
	return owned
