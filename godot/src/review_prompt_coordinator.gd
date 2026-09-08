class_name ReviewPromptCoordinator
extends Node

## Decides when to ask Google Play to show its native in-app review card (issue #2739).
##
## The problem it solves: the store rating is self-selecting — people who hit a bug go and rate,
## engaged players never do. So we ask engaged players, implicitly, right after something good
## happened.
##
## Two rules shape the whole design and neither is ours to bend:
##
##   1. No app-owned UI may precede the card — no modal, no button, no "enjoying the app?".
##      Both Apple and Google prohibit a qualifying pre-prompt or a call-to-action, and a CTA
##      dead-ends anyway for any user over Play's quota. The trigger stays implicit.
##   2. The outcome is not observable. Play's callback fires whether the user rated, dismissed,
##      or never saw the card at all (over quota it renders nothing and reports no error). So we
##      instrument the CALL, not the result, and no metric here may imply an outcome.
##
## Cadence — three shots per install, lifetime, then inert forever:
##
##   Shot 1 — review_session_count >= 5 AND a trigger fires
##   Shot 2 — 7 days after shot 1  AND a trigger fires
##   Shot 3 — 14 days after shot 2 AND a trigger fires
##
## The cooldown is a FLOOR, not a schedule: nothing is evaluated on a timer, and the window
## opening fires nothing on its own. No trigger means no shot, indefinitely. That is why there is
## no _process / Timer here — on_trigger() is the only path that can fire the card.
##
## Our caps sit on top of Play's own (undocumented, roughly monthly) quota; they never replace
## it. Over quota, Play renders nothing — and that still consumes a shot on our side, which is
## intended: we asked, that is what we count.
##
## State lives in the persistent config (config_data.gd) so it survives restarts and updates.
## It is device-local, matching Play's own per-device/account quota.

# The four trigger ids, reported verbatim in the `review_prompted` metric. They are emitted by
# the hook, never inferred here, so the analytics split tells us which trigger carries the
# feature (a trigger at ~0% is a broken hook, not a rare action).
const TRIGGER_FRIEND_ADDED := "friend_added"
const TRIGGER_PLACE_FAVORITED := "place_favorited"
const TRIGGER_WEARABLE_CLAIMED := "wearable_claimed"
const TRIGGER_PLACE_UPVOTED := "place_upvoted"

const ANDROID_PLUGIN_NAME := "dcl-godot-android"

const MAX_SHOTS := 3
const DAY_SECONDS := 86400
# PRODUCTION (active): 5 sessions to open the gate, then a 7-day and a 14-day floor.
const SESSION_GATE := 5
const SHOT_2_FLOOR_SECONDS := 7 * DAY_SECONDS
const SHOT_3_FLOOR_SECONDS := 14 * DAY_SECONDS
# TESTING: to exercise all three shots in one pass on the internal test track (where Play's
# quota does not apply), swap the three consts above for these:
#   const SESSION_GATE := 1
#   const SHOT_2_FLOOR_SECONDS := 60
#   const SHOT_3_FLOOR_SECONDS := 120

# True between loading_started and loading_finished. Never prompt over a loading screen.
var _is_loading: bool = false
# True while an IAP purchase is in flight — the purchase overlay is blocking and modal, and the
# review card is topmost, so the two must never overlap.
var _iap_in_flight: bool = false
# First-wins guard: if two triggers land in the same frame the second is discarded, not queued.
var _firing: bool = false
# The plugin is prewarmed at most once per session, the first time the gate is actually open.
var _prewarmed: bool = false


func _ready() -> void:
	# One foreground session per launch — the same definition analytics uses (Global.session_id
	# is minted once per app run). Deliberately not a second session concept.
	var config: ConfigData = Global.get_config()
	config.review_session_count += 1
	config.save_to_settings_file()
	print(
		"[ReviewPrompt] session ", config.review_session_count, " shots ", config.review_shots_fired
	)

	Global.loading_started.connect(_on_loading_started)
	Global.loading_finished.connect(_on_loading_finished)

	# Trigger 1 — friends. Sent requests are relayed locally by Global (the service doesn't
	# stream our own actions); accepts come off the service signal, which covers both our accept
	# and the server-pushed acceptance of a request we sent.
	Global.friendship_request_sent.connect(_on_friendship_request_sent)
	if Global.social_service != null:
		Global.social_service.friendship_request_accepted.connect(_on_friendship_request_accepted)

	# Trigger 3 — the wearable CLAIM NOTIFICATION, not the marketplace: a claim reaches far more
	# users than a purchase, and hooking the notification needs no new marketplace event.
	NotificationsManager.notification_queued.connect(_on_notification_queued)

	# IAP flow tracking (guard rail, not a trigger).
	Iap.purchase_pending.connect(_on_iap_started)
	Iap.purchase_completed.connect(_on_iap_settled)
	Iap.purchase_failed.connect(_on_iap_settled)
	Iap.purchase_cancelled.connect(_on_iap_settled)

	_maybe_prewarm()


## The single ReviewPromptTrigger entry point. All four triggers are equal — no ranking, no
## priority — and each passes its own trigger_id, which is what the metric reports.
func on_trigger(trigger_id: String) -> void:
	if _firing:
		# Two triggers in the same frame: the first one evaluated wins. The second is dropped,
		# not queued, so a single moment can never fire two shots.
		return
	var config: ConfigData = Global.get_config()
	# Every rejection below is logged. A silent skip is indistinguishable from a hook that never
	# fired at all, which makes the cadence impossible to verify on a device.
	if config.review_shots_fired >= MAX_SHOTS:
		print("[ReviewPrompt] skip (%s): capped at %d shots" % [trigger_id, MAX_SHOTS])
		return
	if not _is_due(config):
		print(
			(
				"[ReviewPrompt] skip (%s): shot #%d not due yet"
				% [trigger_id, config.review_shots_fired + 1]
			)
		)
		return
	if not _guard_rails_clear():
		# Deferred to the next qualifying trigger, never queued: an app that is loading, in an
		# error state or mid-purchase is exactly when a rating prompt reads worst.
		print("[ReviewPrompt] skip (%s): guard rails not clear" % trigger_id)
		return
	if not _is_platform_available():
		print("[ReviewPrompt] skip (%s): no in-app review on this platform" % trigger_id)
		return

	_firing = true
	var shot_id: int = config.review_shots_fired + 1
	# The counters advance on INVOCATION, not on any user outcome — there is no observable
	# outcome. Persist before launching so a crash inside the flow can't hand out a free shot.
	config.review_shots_fired = shot_id
	config.review_last_shot_unix = int(Time.get_unix_time_from_system())
	config.save_to_settings_file()

	print("[ReviewPrompt] launching shot %d (trigger=%s)" % [shot_id, trigger_id])
	_get_android_plugin().launchInAppReview()

	if Global.metrics != null:
		Global.metrics.track_review_prompted(shot_id, trigger_id)
	_firing = false


# Whether the window for the NEXT shot is open. Shot 1 is gated on session count; shots 2 and 3
# on time elapsed since the previous shot actually fired.
func _is_due(config: ConfigData) -> bool:
	var now := int(Time.get_unix_time_from_system())
	match config.review_shots_fired:
		0:
			return config.review_session_count >= SESSION_GATE
		1:
			return now - config.review_last_shot_unix >= SHOT_2_FLOOR_SECONDS
		2:
			return now - config.review_last_shot_unix >= SHOT_3_FLOOR_SECONDS
		_:
			return false


func _guard_rails_clear() -> bool:
	if _is_loading:
		return false
	if _iap_in_flight:
		return false
	# A degraded connection is the app's error state as far as the player is concerned.
	if not ConnectionQualityMonitor.is_connection_healthy():
		return false
	# Any modal of ours on screen. This covers the error states that have their own UI — low
	# memory, scene crash, connection lost, session ended — and it is deliberately broader than
	# just those: the Play card is rendered topmost and cannot be repositioned, so it would land
	# over whatever we were showing. It also keeps us on the right side of the "no app-owned UI
	# in front of the prompt" rule, which a modal underneath the card would otherwise breach.
	if Global.modal_manager != null and Global.modal_manager.current_modal != null:
		return false
	return true


# Android-only for now. The cadence above is deliberately platform-agnostic, so iOS parity is
# just a second branch here calling SKStoreReviewController through the iOS plugin.
func _is_platform_available() -> bool:
	return _get_android_plugin() != null


func _get_android_plugin() -> Object:
	if OS.get_name() != "Android":
		return null
	# Always has_singleton first: a bare get_singleton on a missing name logs an ERROR that
	# Sentry ingests as a real fault.
	if not Engine.has_singleton(ANDROID_PLUGIN_NAME):
		return null
	return Engine.get_singleton(ANDROID_PLUGIN_NAME)


# requestReviewFlow has real latency and its ReviewInfo expires, so we warm it once the gate is
# actually open rather than at boot. The plugin re-requests a stale one instead of firing it.
func _maybe_prewarm() -> void:
	if _prewarmed:
		return
	var config: ConfigData = Global.get_config()
	if config.review_shots_fired >= MAX_SHOTS:
		return
	if not _is_due(config):
		return
	var plugin := _get_android_plugin()
	if plugin == null:
		return
	_prewarmed = true
	plugin.prewarmInAppReview()


func _on_loading_started() -> void:
	_is_loading = true


func _on_loading_finished() -> void:
	_is_loading = false
	# Entering the world is a good moment to warm the request: the gate may have opened this
	# session, and the next trigger could land seconds later.
	_maybe_prewarm()


func _on_iap_started(_product_id: String) -> void:
	_iap_in_flight = true


func _on_iap_settled(_product_id: String, _extra = null) -> void:
	_iap_in_flight = false


func _on_friendship_request_sent(_address: String) -> void:
	on_trigger(TRIGGER_FRIEND_ADDED)


func _on_friendship_request_accepted(_address: String) -> void:
	on_trigger(TRIGGER_FRIEND_ADDED)


func _on_notification_queued(notification: Dictionary) -> void:
	if notification.get("type", "") == "reward_assignment":
		on_trigger(TRIGGER_WEARABLE_CLAIMED)
