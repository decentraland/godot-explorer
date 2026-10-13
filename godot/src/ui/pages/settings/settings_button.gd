extends StaticButton


func _get_unread_count() -> int:
	if Global.player_identity == null:
		return 0
	if not Global.player_identity.is_thirdweb_guest():
		return 0
	return 0 if Global.player_identity.is_thirdweb_guest_upgraded() else 1


func _connect_update_signals() -> void:
	Global.guest_upgrade_state_refreshed.connect(_on_guest_upgrade_state_refreshed)


func _on_guest_upgrade_state_refreshed(_is_upgraded: bool) -> void:
	_update_badge()


func _on_button_clicked() -> void:
	# Settings is a fullscreen menu screen, handled like Backpack: the menu shows the screen and
	# (in the explorer) collapses the navbar. Works the same pre-explorer and in-game.
	Global.open_settings.emit()
	Global.send_haptic_feedback()


func _get_button_metric_name() -> String:
	return "settings"
