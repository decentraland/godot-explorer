extends Node

# Tracks a round-trip to the web marketplace (opened in an in-app browser) and,
# on return, polls the credits balance and the owned-wearables catalog to surface
# the result of a purchase made on the web:
#   - credits went DOWN  → the purchase went through → "your wearable is on the way"
#   - a NEW wearable urn  → it has been delivered to the inventory → "it has arrived"
#
# Driven by open_and_track(), which arms the tracker right before the browser
# opens so we capture the pre-purchase baseline. Return is detected via the app focus
# notifications: presenting the in-app SFSafariViewController makes the host app
# resign active (FOCUS_OUT), and dismissing it makes it active again (FOCUS_IN) —
# the same lifecycle the lobby already uses for the Apple Sign In browser.

enum State { IDLE, ARMED, AWAY, POLLING }

# In-app toast copy. Kept as constants so the wording (and language) is easy to
# tweak in one place.
const _TOAST_ON_THE_WAY_TITLE := "Your wearable is on the way"
const _TOAST_ON_THE_WAY_BODY := "We're processing your purchase."
const _TOAST_ARRIVED_TITLE := "Your wearable has arrived"
const _TOAST_ARRIVED_BODY := "It's now available in your backpack."
const _TOAST_TYPE := "marketplace_iap"

# Polling schedule, measured from when polling starts (1s after returning):
#   - every 2s for the first 15s
#   - then every 5s for the next 60s
#   - then every 10s until a new wearable arrives (or the safety cap below)
const _INITIAL_DELAY_SEC := 1.0
const _FAST_INTERVAL_SEC := 2.0
const _FAST_PHASE_SEC := 15.0
const _MEDIUM_INTERVAL_SEC := 5.0
const _MEDIUM_PHASE_SEC := 60.0
const _SLOW_INTERVAL_SEC := 10.0
# Safety cap: the open-ended slow phase would otherwise poll forever if the user
# browsed without buying (no credit drop, no new wearable). Stop after this.
const _MAX_TOTAL_POLL_SEC := 300.0
# The wearable baseline must be a reliable snapshot — comparisons are meaningless
# without it — so retry the initial fetch a few times before giving up.
const _BASELINE_FETCH_ATTEMPTS := 5
const _BASELINE_RETRY_SEC := 2.0

var _state: State = State.IDLE
# Bumped on every arm()/stop() so a stale baseline capture or polling loop, which
# can't be cancelled mid-await, becomes a no-op when it resumes.
var _token: int = 0
var _baseline_credits: int = 0
var _baseline_urns: Dictionary = {}
var _baseline_ready: bool = false
var _credits_consumed_notified: bool = false


# Single entry point: arm the tracker (snapshot the pre-purchase baseline) and
# open the web marketplace in the in-app browser. `raw_url` is a plain marketplace
# URL — the mobile-IAP view flag is appended here so every entry point matches.
func open_and_track(raw_url: String) -> void:
	_arm()
	Global.open_webview_url(MarketplaceUrl.with_mobile_iap(raw_url))


# Capture the pre-purchase baseline (credits + owned wearables). No-op without a
# wallet, since both the credits balance and the wearable catalog are per-wallet.
func _arm() -> void:
	if Global.player_identity == null or Global.player_identity.get_address_str().is_empty():
		return
	_token += 1
	_state = State.ARMED
	_baseline_ready = false
	_credits_consumed_notified = false
	_async_capture_baseline(_token)


func stop() -> void:
	_token += 1
	_state = State.IDLE


func _notification(what: int) -> void:
	match what:
		NOTIFICATION_APPLICATION_FOCUS_OUT:
			# The browser opened (host app resigned active). Now wait for the return.
			if _state == State.ARMED:
				_state = State.AWAY
		NOTIFICATION_APPLICATION_FOCUS_IN:
			# Back from the browser → kick off the polling schedule. Accept ARMED too
			# (not just AWAY) in case the FOCUS_OUT was missed; a false start just
			# polls against an unchanged baseline and stops at the safety cap.
			if _state == State.ARMED or _state == State.AWAY:
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
	printerr("[MarketplaceTracker] could not capture wearable baseline; tracking disabled")


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
		if elapsed >= _MAX_TOTAL_POLL_SEC:
			print("[MarketplaceTracker] no delivery within %.0fs; stopping" % _MAX_TOTAL_POLL_SEC)
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


# Returns true once a wearable has arrived (terminal). Wearable arrival is checked
# first so it takes precedence over the credit-drop signal when both land on the
# same tick. Fires the "on the way" toast the first time credits drop.
#
# "Arrived" = a genuinely-owned wearable that wasn't owned at baseline (the one
# bought on the web, incl. a deeplink urn= equip once it mints on-chain).
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
				NotificationsManager.show_system_toast(
					_TOAST_ARRIVED_TITLE, _TOAST_ARRIVED_BODY, _TOAST_TYPE
				)
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


# Returns the set (Dictionary keyed by urn) of owned wearable urns, or null on a
# fetch failure — callers must treat null as "unknown", never as "empty".
# gdlint:ignore = async-function-name
func _async_fetch_owned_urns():
	var resp = await WearableRequest.async_request_all_wearables()
	if resp == null:
		return null
	var owned := {}
	for item in resp.elements:
		owned[item.urn] = true
	return owned
