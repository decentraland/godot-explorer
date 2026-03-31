extends Node

## ConnectionQualityMonitor
##
## Autoload that periodically pings a lightweight endpoint to assess
## connection quality. Emits signals consumed by the toast and modal systems.
##
## States:
##   GOOD  -> no issues
##   POOR  -> high latency or isolated errors  -> toast
##   LOST  -> consecutive errors threshold hit  -> modal

signal poor_connection_detected
signal connection_lost_detected
signal connection_restored

enum State { GOOD, POOR, LOST }

const POLL_INTERVAL_SECONDS: float = 10.0
const RETRY_POLL_INTERVAL_SECONDS: float = 3.0
const SLOW_RESPONSE_MS: float = 5000.0

# With explorer: toast at 2, modal at 4 (toast warns first)
const CONSECUTIVE_ERRORS_FOR_POOR: int = 2
const CONSECUTIVE_ERRORS_FOR_LOST: int = 4

# Without explorer: no toast available, go straight to modal faster
const CONSECUTIVE_ERRORS_FOR_LOST_NO_EXPLORER: int = 2

# After retry: skip toast, go straight to modal
const CONSECUTIVE_ERRORS_FOR_LOST_AFTER_RETRY: int = 2

var _state: State = State.GOOD
var _consecutive_errors: int = 0
var _poll_timer: Timer = null
var _is_checking: bool = false
var _retrying: bool = false
var _check_generation: int = 0


func _ready() -> void:
	_poll_timer = Timer.new()
	_poll_timer.wait_time = POLL_INTERVAL_SECONDS
	_poll_timer.timeout.connect(_on_poll_timeout)
	add_child(_poll_timer)
	_poll_timer.start()

	poor_connection_detected.connect(_on_poor_connection)
	connection_lost_detected.connect(_async_on_connection_lost)
	connection_restored.connect(_on_connection_restored)
	Global.modal_manager.connection_lost_retry.connect(_on_retry)
	Global.modal_manager.connection_lost_exit.connect(_on_exit)


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

	var start_ms := Time.get_ticks_msec()
	var promise: Promise = Global.http_requester.request_json(url, HTTPClient.METHOD_GET, "", {})
	var result = await PromiseUtils.async_awaiter(promise)
	var elapsed_ms := Time.get_ticks_msec() - start_ms

	# Discard result if a retry happened or the realm changed while this request was in-flight
	if generation != _check_generation or url != _get_health_url():
		_is_checking = false
		return

	if result is PromiseError:
		_consecutive_errors += 1
		print(
			(
				"[ConnectionQualityMonitor] Request failed (%d consecutive errors): %s"
				% [_consecutive_errors, result.get_error()]
			)
		)
	elif elapsed_ms > SLOW_RESPONSE_MS:
		_consecutive_errors += 1
		print(
			(
				"[ConnectionQualityMonitor] Slow response (%d ms, %d consecutive errors)"
				% [elapsed_ms, _consecutive_errors]
			)
		)
	else:
		if _consecutive_errors > 0 or _retrying:
			print(
				(
					"[ConnectionQualityMonitor] Connection recovered (was %d errors, retrying=%s)"
					% [_consecutive_errors, _retrying]
				)
			)
		_consecutive_errors = 0
		var was_degraded := _state != State.GOOD or _retrying
		_retrying = false
		if _state != State.GOOD:
			_state = State.GOOD
			connection_restored.emit()
		if was_degraded:
			_poll_timer.wait_time = POLL_INTERVAL_SECONDS
			_poll_timer.start()
		_is_checking = false
		return

	_update_state()
	_is_checking = false


func _update_state() -> void:
	var has_explorer := Global.get_explorer() != null
	var lost_threshold: int

	if _retrying:
		lost_threshold = CONSECUTIVE_ERRORS_FOR_LOST_AFTER_RETRY
	elif has_explorer:
		lost_threshold = CONSECUTIVE_ERRORS_FOR_LOST
	else:
		lost_threshold = CONSECUTIVE_ERRORS_FOR_LOST_NO_EXPLORER

	print(
		(
			"[ConnectionQualityMonitor] _update_state: errors=%d threshold=%d state=%d retrying=%s explorer=%s"
			% [_consecutive_errors, lost_threshold, _state, _retrying, has_explorer]
		)
	)

	if _consecutive_errors >= lost_threshold:
		if _state != State.LOST:
			_state = State.LOST
			_retrying = false
			_poll_timer.stop()
			connection_lost_detected.emit()
	elif has_explorer and not _retrying and _consecutive_errors >= CONSECUTIVE_ERRORS_FOR_POOR:
		if _state != State.POOR and _state != State.LOST:
			_state = State.POOR
			poor_connection_detected.emit()


func _get_health_url() -> String:
	var realm_url: String = Global.realm.realm_url
	if realm_url.is_empty():
		return DclUrls.peer_base() + "/about"
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
	await Global.modal_manager.async_show_connection_lost_modal()
	# Replace the default secondary (exit) handler so the modal stays visible while quitting
	if Global.modal_manager.current_modal and Global.modal_manager.current_modal.button_secondary:
		var btn = Global.modal_manager.current_modal.button_secondary
		for connection in btn.pressed.get_connections():
			btn.pressed.disconnect(connection.callable)
		btn.pressed.connect(_on_exit)


func _on_connection_restored() -> void:
	_retrying = false
	_poll_timer.wait_time = POLL_INTERVAL_SECONDS
	_poll_timer.start()
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
	_poll_timer.wait_time = RETRY_POLL_INTERVAL_SECONDS
	_poll_timer.start()
