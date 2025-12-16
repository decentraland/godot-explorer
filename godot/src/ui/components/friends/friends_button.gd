extends AnimatedButton

var _pending_requests_count: int = 0


func _get_unread_count() -> int:
	return _pending_requests_count


func _connect_update_signals() -> void:
	# Connect to social service signals to update badge when requests change
	Global.social_service.friendship_request_received.connect(_on_request_changed)
	Global.social_service.friendship_request_accepted.connect(_on_friendship_changed)
	Global.social_service.friendship_request_rejected.connect(_on_friendship_changed)
	Global.social_service.friendship_request_cancelled.connect(_on_friendship_changed)

	# Initial fetch of pending count
	_async_fetch_pending_count()


func _on_request_changed(_address: String, _message: String = "") -> void:
	_async_fetch_pending_count()


func _on_friendship_changed(_address: String) -> void:
	_async_fetch_pending_count()


func refresh_pending_count() -> void:
	# Public method to refresh the pending count (called after user accepts/rejects requests)
	_async_fetch_pending_count()


func _async_fetch_pending_count() -> void:
	var promise = Global.social_service.get_pending_requests(100, 0)
	await PromiseUtils.async_awaiter(promise)

	if promise.is_rejected():
		return

	_pending_requests_count = promise.get_data().size()
	_update_badge()


func _on_button_clicked() -> void:
	Global.close_menu.emit()
	Global.open_friends_panel.emit()


func _get_button_metric_name() -> String:
	return "friends_button"
