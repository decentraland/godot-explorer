extends Node

# Tracks a round-trip to the web marketplace (opened in an in-app browser) and,
# on return, polls the credits balance and the owned-wearables catalog to surface
# the result of a purchase made on the web:
#   - credits went DOWN  → the purchase went through → "your wearable is on the way"
#   - a NEW wearable urn  → it has been delivered to the inventory → "it has arrived"
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

# In-app toast copy. Kept as constants so the wording (and language) is easy to
# tweak in one place.
const _TOAST_ON_THE_WAY_TITLE := "Your purchase is on the way"
const _TOAST_ON_THE_WAY_BODY := "We're processing your purchase."
# %s is the item kind ("wearable"/"emote"), filled in once we know what arrived.
const _TOAST_ARRIVED_TITLE_FMT := "Your %s has arrived"
const _TOAST_ARRIVED_BODY := "It's now available in your backpack."
const _TOAST_TYPE := "marketplace_iap"

# Log prefix; all lines grep-able as `[MktTracker]` on the --log-stream.
const _LOG := "[MktTracker]"

# Polling schedule, measured from when polling starts (1s after returning):
#   - every 2s for the first 15s
#   - then every 5s thereafter (until a new wearable arrives or the safety cap)
# The data-source lag (the lambda indexer catching up to the on-chain mint) is the
# dominant wait — usually minutes — so the interval mainly bounds how quickly we
# notice once the catalog finally updates; 5s keeps that tail tight.
const _INITIAL_DELAY_SEC := 1.0
const _FAST_INTERVAL_SEC := 2.0
const _FAST_PHASE_SEC := 15.0
const _MEDIUM_INTERVAL_SEC := 5.0
const _MEDIUM_PHASE_SEC := 60.0
const _SLOW_INTERVAL_SEC := 5.0
# Safety cap: the open-ended slow phase would otherwise poll forever if the user
# browsed without buying (no credit drop, no new wearable). Generous because an
# on-chain mint can take several minutes on testnet (observed 2–5+ min); past this
# we give up the live watch (reopening the backpack still fetches fresh).
const _MAX_TOTAL_POLL_SEC := 900.0
# Give-up window when no purchase signal appears. A web checkout spends credits,
# which marketplace-api/balance reflect within ~a minute; if no credit drop is seen
# by the end of the medium phase the user almost certainly didn't buy, so we stop
# rather than poll the safety cap for nothing. A confirmed spend keeps polling.
const _NO_PURCHASE_GIVEUP_SEC := _FAST_PHASE_SEC + _MEDIUM_PHASE_SEC
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
var _baseline_credits: int = 0
var _baseline_urns: Dictionary = {}
var _baseline_ready: bool = false
var _credits_consumed_notified: bool = false
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
	_credits_consumed_notified = false
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
	_baseline_credits = await Iap.async_refresh_balance()
	if token != _token:
		return
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
		var arrived := await _async_check(token)
		if token != _token:
			return
		if arrived:
			stop()
			return
		# No credit drop by the end of the medium phase ⇒ almost certainly no purchase
		# (a web checkout spends credits, reflected within ~a minute via marketplace-api).
		# Give up early rather than polling the safety cap for nothing. A confirmed spend
		# keeps polling up to _MAX_TOTAL_POLL_SEC, since the on-chain mint can lag minutes.
		if not _credits_consumed_notified and elapsed >= _NO_PURCHASE_GIVEUP_SEC:
			stop()
			return
		if elapsed >= _MAX_TOTAL_POLL_SEC:
			stop()
			return
		var interval := _interval_for(elapsed)
		await get_tree().create_timer(interval).timeout
		elapsed += interval


func _interval_for(elapsed: float) -> float:
	if elapsed < _FAST_PHASE_SEC:
		return _FAST_INTERVAL_SEC
	if elapsed < _FAST_PHASE_SEC + _MEDIUM_PHASE_SEC:
		return _MEDIUM_INTERVAL_SEC
	return _SLOW_INTERVAL_SEC


# Returns true once an item has arrived (terminal). Arrival is checked first so it
# takes precedence over the credit-drop signal when both land on the same tick.
# Fires the "on the way" toast the first time credits drop.
#
# "Arrived" = a genuinely-owned wearable or emote that wasn't owned at baseline (the
# one bought on the web, once it mints on-chain). The detected category is passed
# along so the backpack routes it to the right list.
# gdlint:ignore = async-function-name
func _async_check(token: int) -> bool:
	if not _baseline_ready:
		return false

	var urns = await _async_fetch_owned_urns()
	if token != _token:
		return false
	if urns != null:
		for urn in urns:
			if not _baseline_urns.has(urn):
				var category: String = urns[urn]
				var kind := "emote" if category == "emote" else "wearable"
				NotificationsManager.show_system_toast(
					_TOAST_ARRIVED_TITLE_FMT % kind, _TOAST_ARRIVED_BODY, _TOAST_TYPE
				)
				item_arrived.emit(urn, category)
				return true

	if not _credits_consumed_notified:
		var credits: int = await Iap.async_refresh_balance()
		if token != _token:
			return false
		if credits < _baseline_credits:
			_credits_consumed_notified = true
			NotificationsManager.show_system_toast(
				_TOAST_ON_THE_WAY_TITLE, _TOAST_ON_THE_WAY_BODY, _TOAST_TYPE
			)
	return false


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
