extends Control

signal accepted

@onready var v_box_container_terms: VBoxContainer = %VBoxContainer_Terms
@onready var control_separator: Control = %Control_Separator
@onready var button_accept: Button = %Button_Accept
@onready var timer: Timer = $Timer
@onready var spinner: TextureProgressBar = %Spinner


func _on_check_box_terms_and_privacy_toggled(toggled_on: bool) -> void:
	%Button_Accept.disabled = !toggled_on


func _on_rich_text_label_meta_clicked(meta: Variant) -> void:
	Global.open_webview_url(meta)


func _on_button_accept_pressed() -> void:
	Services.metrics.track_screen_viewed("ACCEPT_EULA", "")
	Services.metrics.track_click_button("accept", "ACCEPT_EULA", "")
	Services.metrics.flush()
	spinner.show()
	control_separator.hide()
	button_accept.hide()
	timer.start()


func _on_button_reject_pressed() -> void:
	v_box_container_terms.hide()
	get_tree().quit()


func _on_control_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if event.pressed:
			%CheckBox_TermsAndPrivacy.button_pressed = !%CheckBox_TermsAndPrivacy.button_pressed


func _on_timer_timeout() -> void:
	Services.config.terms_and_conditions_version = Global.TERMS_AND_CONDITIONS_VERSION
	Services.config.save_to_settings_file()
	accepted.emit()
	if !Global.is_xr():
		get_tree().change_scene_to_file("res://src/ui/pages/auth/lobby.tscn")
