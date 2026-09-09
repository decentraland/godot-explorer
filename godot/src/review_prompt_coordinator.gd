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

## Emitted on every decision this coordinator makes — a shot fired, or the reason one was not.
## Consumed by ReviewDebugPanel (dev builds only); nothing in the shipping path listens.
signal debug_event(kind: String, text: String)

# The four trigger ids, reported verbatim in the `review_prompted` metric. They are emitted by
# the hook, never inferred here, so the analytics split tells us which trigger carries the
# feature (a trigger at ~0% is a broken hook, not a rare action).
const TRIGGER_FRIEND_ADDED := "friend_added"
const TRIGGER_PLACE_FAVORITED := "place_favorited"
const TRIGGER_WEARABLE_CLAIMED := "wearable_claimed"
const TRIGGER_PLACE_UPVOTED := "place_upvoted"

# Plain-English names for the QA panel and its messages. The wire ids above are what analytics
# sees and must never change; these are only ever shown to a tester.
const TRIGGER_LABELS := {
	TRIGGER_FRIEND_ADDED: "Added a friend",
	TRIGGER_PLACE_FAVORITED: "Favorited a place",
	TRIGGER_WEARABLE_CLAIMED: "Claimed a wearable",
	TRIGGER_PLACE_UPVOTED: "Liked a place",
}

# Why a prompt was held back, in words a tester can act on.
const RAIL_REASONS := {
	"loading": "the app is loading",
	"iap": "a purchase is in progress",
	"connection": "the connection is poor",
	"modal": "a dialog is open",
}

const ANDROID_PLUGIN_NAME := "dcl-godot-android"

# Beat between the player's action and the card, so the prompt reads as a considered moment
# rather than a reflex to the tap. Long enough for the action's own feedback (the friend added,
# the star filling in) to land first; short enough that the card is still clearly about it.
# Nothing in Play's policy constrains WHEN launchReviewFlow is called — only that we never put
# our own UI or a call-to-action in front of it, which this does not.
const PROMPT_DELAY_SECONDS := 1.2

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
# CanvasLayer holding the QA panel, parented to this node (which lives under root), so the panel
# survives explorer scene reloads and is readable in the lobby too. Null unless the harness is on.
var _debug_layer: CanvasLayer = null


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

	# QA harness (#2739). The boot deeplink is read here rather than in global.gd: that file sits
	# exactly on its 1900-line lint ceiling, and this is the natural owner anyway.
	capture_deeplink(Global.deep_link_obj)
	set_debug_panel_enabled(config.review_debug_enabled)

	_maybe_prewarm()


## The single ReviewPromptTrigger entry point. All four triggers are equal — no ranking, no
## priority — and each passes its own trigger_id, which is what the metric reports.
func on_trigger(trigger_id: String) -> void:
	if _firing:
		# Two triggers in the same frame: the first one evaluated wins. The second is dropped,
		# not queued, so a single moment can never fire two shots.
		return
	var config: ConfigData = Global.get_config()
	var dry_run := is_debug_enabled()
	# Every rejection below is logged. A silent skip is indistinguishable from a hook that never
	# fired at all, which makes the cadence impossible to verify on a device.
	if config.review_shots_fired >= MAX_SHOTS:
		_reject(
			trigger_id,
			"capped at %d shots" % MAX_SHOTS,
			"all %d asks used, it will never ask again" % MAX_SHOTS
		)
		return
	if not _is_due(config):
		_reject(
			trigger_id,
			"shot #%d not due yet" % (config.review_shots_fired + 1),
			"too soon, the next ask is still locked"
		)
		return
	var rail := _blocking_rail()
	if not rail.is_empty():
		# Deferred to the next qualifying trigger, never queued: an app that is loading, in an
		# error state or mid-purchase is exactly when a rating prompt reads worst.
		_reject(
			trigger_id,
			"blocked by guard rail: %s" % rail,
			"held back because %s" % RAIL_REASONS.get(rail, rail)
		)
		return
	# Dry run skips the platform check too, so the harness works on desktop where there is no
	# Play at all.
	if not dry_run and not _is_platform_available():
		_reject(
			trigger_id, "no in-app review on this platform", "this platform has no in-app review"
		)
		return

	# Held across the delay inside _async_launch, so a second trigger landing during it is
	# dropped rather than queueing a duplicate ask.
	_firing = true
	_async_launch(trigger_id, dry_run)


# The launch half of on_trigger, split out only because it waits: it is a coroutine, and
# on_trigger is called from signal handlers that should not have to await it.
func _async_launch(trigger_id: String, dry_run: bool) -> void:
	await get_tree().create_timer(PROMPT_DELAY_SECONDS).timeout

	# The app can change while we wait — a scene starts loading, a modal opens, a purchase
	# begins. The rails were clear when the trigger arrived; what matters is that they are clear
	# when the card actually appears, so re-check rather than trust the earlier answer.
	var rail := _blocking_rail()
	if not rail.is_empty():
		_reject(
			trigger_id,
			"blocked by guard rail after delay: %s" % rail,
			"held back because %s" % RAIL_REASONS.get(rail, rail)
		)
		_firing = false
		return

	var config: ConfigData = Global.get_config()
	var shot_id: int = config.review_shots_fired + 1
	# The counters advance on INVOCATION, not on any user outcome — there is no observable
	# outcome. Advanced here, after the delay, rather than before it: a shot the rails cancelled
	# was never shown to anyone, and burning one of the three for it would be a real loss.
	config.review_shots_fired = shot_id
	config.review_last_shot_unix = int(Time.get_unix_time_from_system())
	config.save_to_settings_file()

	if dry_run:
		# Report what WOULD have happened and stop. Deliberately no track_review_prompted here:
		# a fake shot in Segment would corrupt the very metric the feature is judged on. The
		# real metric path is exercised whenever the harness is off.
		print(
			"[ReviewPrompt] DRY RUN shot %d (trigger=%s) — Play not called" % [shot_id, trigger_id]
		)
		debug_event.emit(
			"fired",
			(
				"%s — would ask now (test mode, no real prompt) · ask %d of %d"
				% [trigger_label(trigger_id), shot_id, MAX_SHOTS]
			)
		)
		_firing = false
		return

	print("[ReviewPrompt] launching shot %d (trigger=%s)" % [shot_id, trigger_id])
	debug_event.emit(
		"fired", "%s — asking now · ask %d of %d" % [trigger_label(trigger_id), shot_id, MAX_SHOTS]
	)
	_get_android_plugin().launchInAppReview()

	if Global.metrics != null:
		Global.metrics.track_review_prompted(shot_id, trigger_id)
	_firing = false


# One place for "a trigger arrived and nothing happened", so the log line and the debug panel
# can never drift apart.
func _reject(trigger_id: String, reason: String, human: String) -> void:
	# The log line stays terse and technical — it is grepped and lands in Sentry breadcrumbs.
	# The signal carries the sentence the QA panel shows.
	print("[ReviewPrompt] skip (%s): %s" % [trigger_id, reason])
	debug_event.emit("skip", "%s — %s" % [trigger_label(trigger_id), human])


## Plain-English name for a trigger id, for the QA panel. Falls back to the raw id.
func trigger_label(trigger_id: String) -> String:
	return TRIGGER_LABELS.get(trigger_id, trigger_id)


## Apply `?review-debug=true|false` from a deeplink (issue #2739) and persist it. Called at boot
## with the fake/generated deeplink, and again by DeepLinkRouter for a live one.
##
## Accepts `false` as well as `true` so a tester can switch the harness back off without clearing
## app data — a one-way flag would strand the panel on screen for the life of the install.
func capture_deeplink(obj) -> void:
	if obj == null or Global.is_production():
		return
	if not obj.params.has("review-debug"):
		return
	var enabled := String(obj.params.get("review-debug", "")).to_lower() == "true"
	var config: ConfigData = Global.get_config()
	if config.review_debug_enabled != enabled:
		config.review_debug_enabled = enabled
		config.save_to_settings_file()
	set_debug_panel_enabled(enabled)
	print("[ReviewPrompt] review-debug=", enabled)


## Mount or unmount the on-screen QA panel.
func set_debug_panel_enabled(enable: bool) -> void:
	# The harness must never be reachable in a shipping build, whatever settings.cfg says.
	if enable and Global.is_production():
		return
	if is_instance_valid(_debug_layer) == enable:
		return

	if enable:
		_debug_layer = CanvasLayer.new()
		_debug_layer.name = "ReviewDebugLayer"
		_debug_layer.layer = 1000
		var scene := load(
			"res://src/ui/components/organisms/review_debug_panel/review_debug_panel.tscn"
		)
		_debug_layer.add_child(scene.instantiate())
		add_child(_debug_layer)
		print("[ReviewPrompt] QA panel mounted")
	else:
		_debug_layer.queue_free()
		_debug_layer = null


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


## Returns the name of the guard rail currently blocking a prompt, or "" when all are clear.
## A name rather than a bool so the log and the debug panel can say WHICH rail blocked.
func _blocking_rail() -> String:
	if _is_loading:
		return "loading"
	if _iap_in_flight:
		return "iap"
	# A degraded connection is the app's error state as far as the player is concerned.
	if not ConnectionQualityMonitor.is_connection_healthy():
		return "connection"
	# Any modal of ours on screen. This covers the error states that have their own UI — low
	# memory, scene crash, connection lost, session ended — and it is deliberately broader than
	# just those: the Play card is rendered topmost and cannot be repositioned, so it would land
	# over whatever we were showing. It also keeps us on the right side of the "no app-owned UI
	# in front of the prompt" rule, which a modal underneath the card would otherwise breach.
	if Global.modal_manager != null and Global.modal_manager.current_modal != null:
		return "modal"
	return ""


## True when the QA harness is on. Hard-gated to non-production: is_production() is the gate this
## codebase uses, NOT OS.is_debug_build() — CI exports with --export-release, so the debug-build
## check would hide this from the very builds QA installs.
func is_debug_enabled() -> bool:
	return Global.get_config().review_debug_enabled and not Global.is_production()


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
