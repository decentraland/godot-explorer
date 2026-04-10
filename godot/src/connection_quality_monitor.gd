extends Node

## ConnectionQualityMonitor
##
## Autoload that periodically HEADs the current realm's /about (or peer_base when
## no realm is set) to assess connection quality. Emits signals consumed by the
## toast and modal systems.
##
## Polling:
##   - Slow (10s): happy path, when the last ping succeeded
##   - Fast (0.5s): investigation mode, triggered as soon as any ping fails so
##     consecutive errors get counted quickly without waiting a full slow tick
##
## Thresholds (consecutive failures):
##   - With explorer UI:    2 → poor connection toast, 4 → connection lost modal
##   - Without explorer UI: 2 → connection lost modal (no toast stage)

enum State { GOOD, POOR, LOST }

const FAST_POLL_SECONDS: float = 0.5
const SLOW_POLL_SECONDS: float = 10.0
const REQUEST_TIMEOUT_SECONDS: float = 3.0
const CONSECUTIVE_ERRORS_FOR_DEGRADED: int = 2
const CONSECUTIVE_ERRORS_FOR_LOST: int = 4

var _state: State = State.GOOD
var _consecutive_errors: int = 0
var _poll_timer: Timer = null
var _is_checking: bool = false
var _check_generation: int = 0
var _retrying: bool = false
var _ios_retry_used: bool = false


func _ready() -> void:
	_poll_timer = Timer.new()
	_poll_timer.wait_time = FAST_POLL_SECONDS
	_poll_timer.timeout.connect(_on_poll_timeout)
	add_child(_poll_timer)
	_poll_timer.start()
	# Before a realm is set (lobby / backpack / discover), _get_health_url() falls
	# back to peer_base() so we still detect real connection loss on those screens.

	Global.modal_manager.connection_lost_retry.connect(_on_retry)
	Global.modal_manager.connection_lost_exit.connect(_on_exit)

	# Pause polling while a realm change is in flight so 404s / slow /about calls
	# on the new realm don't get counted as connection failures.
	Global.realm.realm_changing.connect(_on_realm_changing)
	Global.realm.realm_changed.connect(_on_realm_changed)
	Global.realm.realm_change_failed.connect(_on_realm_change_failed)


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
	var promise: Promise = Global.http_requester.request_json(url, HTTPClient.METHOD_HEAD, "", {})
	var timeout_promise := _create_timeout_promise(REQUEST_TIMEOUT_SECONDS)

	var start_ms := Time.get_ticks_msec()
	var result = await PromiseUtils.async_race([promise, timeout_promise])
	var elapsed_ms := Time.get_ticks_msec() - start_ms

	# Discard result if a retry happened or the realm changed while this request was in-flight
	if generation != _check_generation or url != _get_health_url():
		_is_checking = false
		return

	# Timeout or error
	if not promise.is_resolved() or result is PromiseError:
		_consecutive_errors += 1
		_set_poll_interval(FAST_POLL_SECONDS)
		if not promise.is_resolved():
			printerr(
				(
					"[ConnectionQualityMonitor] Request timed out after %d ms (%d consecutive errors)"
					% [elapsed_ms, _consecutive_errors]
				)
			)
		else:
			printerr(
				(
					"[ConnectionQualityMonitor] Request failed (%d consecutive errors): %s"
					% [_consecutive_errors, result.get_error()]
				)
			)
	else:
		if _consecutive_errors > 0:
			prints(
				"[ConnectionQualityMonitor] Connection recovered (was",
				_consecutive_errors,
				"errors)"
			)
		_consecutive_errors = 0
		_set_poll_interval(SLOW_POLL_SECONDS)
		if _state != State.GOOD:
			_state = State.GOOD
			_on_connection_restored()
		_is_checking = false
		return

	_update_state()
	_is_checking = false


func _update_state() -> void:
	if _state == State.LOST:
		return

	# After retry: 1 failure goes straight to modal
	if _retrying and _consecutive_errors >= 1:
		_state = State.LOST
		_retrying = false
		_async_on_connection_lost()
		return

	var has_explorer := Global.get_explorer() != null

	# With explorer: toast at 2, modal at 4
	# Without explorer: modal at 2 (no toast)
	if (
		has_explorer
		and _state == State.GOOD
		and _consecutive_errors >= CONSECUTIVE_ERRORS_FOR_DEGRADED
	):
		_state = State.POOR
		_on_poor_connection()
	elif (
		_consecutive_errors
		>= (CONSECUTIVE_ERRORS_FOR_LOST if has_explorer else CONSECUTIVE_ERRORS_FOR_DEGRADED)
	):
		_state = State.LOST
		_async_on_connection_lost()


func _create_timeout_promise(timeout_seconds: float) -> Promise:
	var p := Promise.new()
	get_tree().create_timer(timeout_seconds).timeout.connect(func(): p.reject("timeout"))
	return p


func _set_poll_interval(interval: float) -> void:
	if _poll_timer.wait_time != interval:
		_poll_timer.wait_time = interval
		_poll_timer.start()


func _get_health_url() -> String:
	var realm_url: String = Global.realm.realm_url
	if realm_url.is_empty():
		return DclUrls.peer_base() + "/about"
	if not realm_url.ends_with("/"):
		realm_url += "/"
	return realm_url + "about"


func _on_poor_connection() -> void:
	if not Global.get_explorer():
		return
	NotificationsManager.show_system_toast(
		"Poor connection",
		"Your connection is unstable. Some features may not work properly.",
		"poor_connection",
		"alert"
	)


func _async_on_connection_lost() -> void:
	# On iOS: first time show retry, second time show modal without buttons
	var hide_buttons := OS.get_name() == "iOS" and _ios_retry_used
	await Global.modal_manager.async_show_connection_lost_modal(hide_buttons)
	# Replace the default secondary (exit) handler so the modal stays visible while quitting
	# On iOS the exit button is hidden, so no rewiring needed
	if OS.get_name() != "iOS":
		if (
			Global.modal_manager.current_modal
			and Global.modal_manager.current_modal.button_secondary
		):
			var btn = Global.modal_manager.current_modal.button_secondary
			if btn.pressed.is_connected(Global.modal_manager._on_connection_lost_secondary):
				btn.pressed.disconnect(Global.modal_manager._on_connection_lost_secondary)
			btn.pressed.connect(_on_exit)


func _on_connection_restored() -> void:
	_retrying = false
	_ios_retry_used = false
	Global.modal_manager.close_current_modal()


func _on_exit() -> void:
	_poll_timer.stop()
	get_tree().quit()


func _on_retry() -> void:
	_check_generation += 1
	_consecutive_errors = 0
	_state = State.GOOD
	_is_checking = false
	_retrying = true
	if OS.get_name() == "iOS":
		_ios_retry_used = true


func _on_realm_changing() -> void:
	# Discard any in-flight check and stop pinging until the realm change settles.
	_poll_timer.stop()
	_check_generation += 1
	_is_checking = false
	_consecutive_errors = 0


func _on_realm_changed() -> void:
	# New realm is validated: (re)start polling from a clean slate.
	_resume_polling()


func _on_realm_change_failed(_new_realm_string: String, _reason: String) -> void:
	# Realm change was rejected. Resume polling either against the previous realm
	# (if any) or against the peer_base() fallback used in the pre-realm screens.
	_resume_polling()


func _resume_polling() -> void:
	_state = State.GOOD
	_consecutive_errors = 0
	_retrying = false
	_set_poll_interval(FAST_POLL_SECONDS)
	_poll_timer.start()
