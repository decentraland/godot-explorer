extends Node

## ConnectionQualityMonitor
##
## Autoload that periodically HEADs the current realm's /about (or peer_base when
## no realm is set) to assess connection quality. Emits signals consumed by the
## toast and modal systems.
##
## Two decoupled layers:
##
##   1. Connection health — a truthful, ping-driven 0..1 score with hysteresis bands.
##      A fail lowers it (a hang/timeout more than a quick refusal); a success raises it
##      (a slow success heals less than a fast one). The UI NEVER alters this score, so it
##      always reflects the real network. Separate enter/exit thresholds mean a single lucky
##      ping cannot move the state out of LOST, so the modal cannot flap.
##
##   2. Feedback policy — decides WHEN to show/re-show the toast & modal. The "Retry" button
##      is cosmetic (it does not reconnect anything — see modal_manager._on_connection_lost_primary),
##      so here Retry only SNOOZES the modal: it hides it and stays quiet for a window while the
##      connection stays objectively LOST underneath. The snooze window backs off (grows) on each
##      successive retry and decays back toward its base during calm; a genuine recovery resets it.
##      Only real health recovery dismisses the modal for good.
##
## Polling:
##   - Slow (10s): happy path, when health is solidly good
##   - Fast (0.5s): investigation mode, while degraded or still recovering
##
## Feedback (consumes health state):
##   - With explorer UI:    POOR → poor connection toast, LOST → connection lost modal
##   - Without explorer UI: POOR → connection lost modal (no toast stage)

enum State { GOOD, POOR, LOST }

const FAST_POLL_SECONDS: float = 0.5
const SLOW_POLL_SECONDS: float = 10.0
const REQUEST_TIMEOUT_SECONDS: float = 3.0

# --- Health-score tunables (layer 1) ---------------------------------------
# Defaults give POOR at ~2 and LOST at ~4 accumulated failures (matching the old
# consecutive-error thresholds) but, unlike a counter, the score also heals on success
# so intermittent loss does not march straight to LOST.
const HEALTH_START: float = 1.0
const HEALTH_W_FAIL: float = 0.23  # quick failure (connection refused, etc.)
const HEALTH_W_TIMEOUT: float = 0.26  # request hung until REQUEST_TIMEOUT (worse than a quick fail)
const HEALTH_W_OK: float = 0.34  # heal per fast success (~3 clean pings fully recover)

const BAND_POOR_ENTER: float = 0.55  # GOOD → POOR
const BAND_POOR_EXIT: float = 0.65  # POOR → GOOD  (exit > enter ⇒ hysteresis)
const BAND_LOST_ENTER: float = 0.25  # → LOST
const BAND_LOST_EXIT: float = 0.45  # leave LOST only once health climbs back above this

# --- Snooze tunables (layer 2): exponential backoff with time decay --------
const SNOOZE_BASE: float = 10.0  # quiet window the first retry buys
const SNOOZE_GROWTH: float = 2.0  # each further retry multiplies the window
const SNOOZE_MAX: float = 120.0  # cap on the quiet window
const SNOOZE_DECAY_HALFLIFE: float = 45.0  # excess-over-base halves every this many seconds of calm

var _state: State = State.GOOD
# Truthful connection health in [0, 1]. Driven only by ping outcomes, never by the UI.
var _health: float = HEALTH_START
var _poll_timer: Timer = null
var _is_checking: bool = false
var _check_generation: int = 0
var _ios_retry_used: bool = false
# True while a connection_lost modal opened *by us* is on screen. Used so that
# _on_connection_restored only dismisses our own modal, not e.g. a teleport modal
# that the user opened in the meantime.
var _showing_our_modal: bool = false
# Feedback snooze: while now < _snooze_until the modal stays hidden even if health is LOST.
var _snooze_until: float = 0.0
# Window the NEXT retry will apply (grows on retry, decays toward SNOOZE_BASE during calm).
var _snooze_next: float = SNOOZE_BASE
var _snooze_last_update: float = 0.0


func _ready() -> void:
	pass


# Setup moved out of _ready so Global.async_boot can sequence it deterministically
# under the splash. See BootInstrumentation.
# gdlint:ignore = async-function-name
func initialize_async() -> void:
	BootInstrumentation.mark("connection_quality_monitor.initialize_async_start")
	_snooze_last_update = _now()

	_poll_timer = Timer.new()
	_poll_timer.wait_time = FAST_POLL_SECONDS
	_poll_timer.timeout.connect(_on_poll_timeout)
	add_child(_poll_timer)
	# Timer is started in _connect_signals after Global is fully initialized.

	_connect_signals.call_deferred()
	BootInstrumentation.mark("connection_quality_monitor.initialize_async_end")


func _connect_signals() -> void:
	if Services.modal_manager == null:
		# modal_manager not ready yet; retry next frame
		_connect_signals.call_deferred()
		return
	Services.modal_manager.connection_lost_retry.connect(_on_retry)
	Services.modal_manager.connection_lost_exit.connect(_on_exit)

	# Pause polling while a realm change is in flight so 404s / slow /about calls
	# on the new realm don't get counted as connection failures.
	Services.realm.realm_changing.connect(_on_realm_changing)
	Services.realm.realm_changed.connect(_on_realm_changed)
	Services.realm.realm_change_failed.connect(_on_realm_change_failed)

	# Before a realm is set (lobby / backpack / discover), _get_health_url() falls
	# back to peer_base() so we still detect real connection loss on those screens.
	_poll_timer.start()


func _now() -> float:
	return Time.get_ticks_msec() / 1000.0


func _on_poll_timeout() -> void:
	if _is_checking:
		return
	_async_check_connection()


func _async_check_connection() -> void:
	_is_checking = true
	var generation := _check_generation

	var url := _get_health_url()
	if url.is_empty():
		_is_checking = false
		return

	# HEAD instead of GET: we only care about reachability + status code, not the
	# /about payload (which is several KB of realm metadata per poll).
	# Pass an explicit HTTP timeout slightly above the async_race below so the underlying
	# request aborts at the HTTP layer (~4s) instead of lingering on the default 60s. Without
	# this, abandoned requests during an outage hold slots in the shared request queue (limit
	# 10) and starve recovery checks, so health stays stuck even after connectivity returns.
	var promise: Promise = Services.http_requester.request_json_with_timeout(
		url, HTTPClient.METHOD_HEAD, "", {}, REQUEST_TIMEOUT_SECONDS + 1.0
	)
	var timeout_promise := _create_timeout_promise(REQUEST_TIMEOUT_SECONDS)

	var start_ms := Time.get_ticks_msec()
	var result = await PromiseUtils.async_race([promise, timeout_promise])
	var elapsed_ms := Time.get_ticks_msec() - start_ms

	# Discard result if a retry happened or the realm changed while this request was in-flight
	if generation != _check_generation or url != _get_health_url():
		_is_checking = false
		return

	var timed_out := not promise.is_resolved()
	var ok := promise.is_resolved() and not (result is PromiseError)
	var latency_seconds := elapsed_ms / 1000.0

	if not ok:
		if timed_out:
			printerr(
				(
					"[ConnectionQualityMonitor] Request timed out after %d ms (health %.2f)"
					% [elapsed_ms, _health]
				)
			)
		else:
			printerr(
				(
					"[ConnectionQualityMonitor] Request failed (health %.2f): %s"
					% [_health, result.get_error()]
				)
			)

	# Layer 1: update truthful health, then derive the new state.
	var prev_state := _state
	_observe_health(ok, latency_seconds, timed_out)

	# Investigate fast while degraded or still recovering; slow only when solidly good.
	if not ok or _health < BAND_POOR_EXIT:
		_set_poll_interval(FAST_POLL_SECONDS)
	else:
		_set_poll_interval(SLOW_POLL_SECONDS)

	# Layer 2: feedback (toast / modal) reacting to the state transition.
	await _async_update_feedback(prev_state)
	_is_checking = false


# --- Layer 1: connection health --------------------------------------------
func _observe_health(ok: bool, latency_seconds: float, timed_out: bool) -> void:
	if not ok:
		_health -= HEALTH_W_TIMEOUT if timed_out else HEALTH_W_FAIL
	else:
		var latency_factor := clampf(1.0 - latency_seconds / REQUEST_TIMEOUT_SECONDS, 0.0, 1.0)
		_health += HEALTH_W_OK * latency_factor
	_health = clampf(_health, 0.0, 1.0)
	_state = _classify_state()


func _classify_state() -> State:
	var h := _health
	if _state == State.LOST:
		# Hysteresis: leave LOST only once health climbs well past the entry threshold.
		if h >= BAND_LOST_EXIT:
			return State.GOOD if h >= BAND_POOR_EXIT else State.POOR
		return State.LOST
	if _state == State.POOR:
		if h < BAND_LOST_ENTER:
			return State.LOST
		if h >= BAND_POOR_EXIT:
			return State.GOOD
		return State.POOR
	# GOOD
	if h < BAND_LOST_ENTER:
		return State.LOST
	if h < BAND_POOR_ENTER:
		return State.POOR
	return State.GOOD


# --- Layer 2: feedback policy ----------------------------------------------
func _async_update_feedback(prev_state: State) -> void:
	var has_explorer := Global.get_explorer() != null

	# Warn only while degrading (GOOD → POOR); never on the way back up.
	if _state == State.POOR and prev_state == State.GOOD and has_explorer:
		_on_poor_connection()

	# Genuine recovery is the ONLY thing that dismisses the modal for real.
	if _state == State.GOOD and prev_state != State.GOOD:
		_on_connection_restored()
		_snooze_next = SNOOZE_BASE  # clean slate: a later, unrelated problem starts fresh
		_snooze_last_update = _now()
		return

	# "Lost enough to alarm": LOST, or POOR when there is no toast stage.
	var alarm := _state == State.LOST or (_state == State.POOR and not has_explorer)
	if alarm and not _showing_our_modal and _now() >= _snooze_until:
		await _async_on_connection_lost()


func _create_timeout_promise(timeout_seconds: float) -> Promise:
	var p := Promise.new()
	get_tree().create_timer(timeout_seconds).timeout.connect(func(): p.reject("timeout"))
	return p


func _set_poll_interval(interval: float) -> void:
	if _poll_timer.wait_time != interval:
		_poll_timer.wait_time = interval
		_poll_timer.start()


func _get_health_url() -> String:
	var realm_url: String = Services.realm.realm_url
	if realm_url.is_empty():
		return DclUrls.peer_base() + "/about"
	if not realm_url.ends_with("/"):
		realm_url += "/"
	return realm_url + "about"


func _on_poor_connection() -> void:
	if not Global.get_explorer():
		return
	Services.notifications_manager.show_system_toast(
		"Poor connection",
		"Your connection is unstable. Some features may not work properly.",
		"poor_connection",
		"alert"
	)


func _async_on_connection_lost() -> void:
	# On iOS: first time show retry, second time show modal without buttons
	var hide_buttons := OS.get_name() == "iOS" and _ios_retry_used
	_showing_our_modal = true
	await Services.modal_manager.async_show_connection_lost_modal(hide_buttons)
	# Exit handling is wired via Services.modal_manager.connection_lost_exit → _on_exit
	# in _ready(). modal_manager._on_connection_lost_secondary intentionally does not
	# close the modal so it stays visible while get_tree().quit() runs.


func _on_connection_restored() -> void:
	_ios_retry_used = false
	if _showing_our_modal:
		_showing_our_modal = false
		Services.modal_manager.close_current_modal()


func _on_exit() -> void:
	_poll_timer.stop()
	get_tree().quit()


func _on_retry() -> void:
	# Retry is cosmetic w.r.t. the network: it does NOT touch health. It only snoozes the
	# modal, with the quiet window backing off on each successive retry (and decaying during
	# calm). Health stays objectively LOST, so if the connection is still down the modal
	# returns once the snooze expires (buttonless on iOS).
	var now := _now()
	_decay_snooze(now)
	_snooze_until = now + _snooze_next
	_snooze_next = minf(_snooze_next * SNOOZE_GROWTH, SNOOZE_MAX)
	# modal_manager._on_connection_lost_primary already closed the modal.
	_showing_our_modal = false
	if OS.get_name() == "iOS":
		_ios_retry_used = true


func _decay_snooze(now: float) -> void:
	# Relax the backoff back toward SNOOZE_BASE for the calm time elapsed since last update.
	var dt := now - _snooze_last_update
	_snooze_last_update = now
	var excess := _snooze_next - SNOOZE_BASE
	if dt > 0.0 and excess > 0.0:
		excess *= pow(0.5, dt / SNOOZE_DECAY_HALFLIFE)
		_snooze_next = SNOOZE_BASE + excess


func _on_realm_changing() -> void:
	# Discard any in-flight check and stop pinging until the realm change settles.
	_poll_timer.stop()
	_check_generation += 1
	_is_checking = false
	_reset_health()


func _on_realm_changed() -> void:
	# New realm is validated: (re)start polling from a clean slate.
	_resume_polling()


func _on_realm_change_failed(_new_realm_string: String, _reason: String) -> void:
	# Realm change was rejected. Resume polling either against the previous realm
	# (if any) or against the peer_base() fallback used in the pre-realm screens.
	_resume_polling()


func _reset_health() -> void:
	_health = HEALTH_START
	_state = State.GOOD
	_snooze_until = 0.0
	_snooze_next = SNOOZE_BASE
	_snooze_last_update = _now()
	if _showing_our_modal:
		_showing_our_modal = false
		Services.modal_manager.close_current_modal()


func _resume_polling() -> void:
	_reset_health()
	_set_poll_interval(FAST_POLL_SECONDS)
	_poll_timer.start()
