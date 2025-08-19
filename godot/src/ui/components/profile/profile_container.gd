extends ColorRect
@onready var profile_panel: PanelContainer = %Profile


func _on_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if event.pressed:
			close()


func close() -> void:
	profile_panel.close()
	hide()


func open(profile: DclUserProfile) -> void:
	show()
	profile_panel.async_show_profile(profile)


func _on_button_close_profile_pressed() -> void:
	close()
