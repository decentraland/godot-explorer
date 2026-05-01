class_name DebouncedAction
extends Node

const COOLDOWN_MSEC: int = 1000

var _pending_state: Variant
var _last_sent_state: Variant
var _sending: bool = false
var _last_send_msec: int = 0
var _debounce_timer: Timer = null
var _callback: Callable


func _init(callback: Callable, initial_state: Variant = null) -> void:
	_callback = callback
	_pending_state = initial_state
	_last_sent_state = initial_state


func schedule(state: Variant) -> void:
	_pending_state = state
	var elapsed := Time.get_ticks_msec() - _last_send_msec
	if not _sending and elapsed >= COOLDOWN_MSEC:
		_async_flush_pending()
		return
	var remaining := maxi(COOLDOWN_MSEC - elapsed, 0)
	if _debounce_timer == null:
		_debounce_timer = Timer.new()
		_debounce_timer.one_shot = true
		_debounce_timer.timeout.connect(_on_debounce_timeout)
		add_child(_debounce_timer)
	_debounce_timer.start(remaining / 1000.0)


func set_state_no_send(state: Variant) -> void:
	_pending_state = state
	_last_sent_state = state


func _on_debounce_timeout() -> void:
	_async_flush_pending()


func _async_flush_pending() -> void:
	if _sending:
		return
	if _pending_state == _last_sent_state:
		return

	_sending = true
	var state = _pending_state
	_last_sent_state = state
	_last_send_msec = Time.get_ticks_msec()
	await _callback.call(state)
	_sending = false

	if _pending_state != _last_sent_state:
		var elapsed := Time.get_ticks_msec() - _last_send_msec
		if elapsed >= COOLDOWN_MSEC:
			_async_flush_pending()
		else:
			_debounce_timer.start((COOLDOWN_MSEC - elapsed) / 1000.0)
