extends AnimatedButton

var _pending_requests_count: int = 0


func _get_unread_count() -> int:
	return _pending_requests_count


func _connect_update_signals() -> void:
	Global.friends_request_size_changed.connect(_set_pending_request_count)
	var explorer = Global.get_explorer()
	if explorer and explorer.friends_panel:
		if explorer.friends_panel.request_list:
			_set_pending_request_count(explorer.friends_panel.request_list.size())


func _set_pending_request_count(count: int) -> void:
	_pending_requests_count = count
	_update_badge()


func _on_button_clicked() -> void:
	Global.close_menu.emit()
	Global.open_friends_panel.emit()


func _get_button_metric_name() -> String:
	return "friends_button"
