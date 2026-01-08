extends ColorRect

signal update_name_on_profile(nickname: String)

var new_nickname: String

@onready var button_cancel: Button = %Button_Cancel
@onready var button_save: Button = %Button_Save
@onready var button_claim_name: Button = %Button_ClaimName
@onready var label_tag: Label = %Label_Tag
@onready var line_edit_new_name: LineEdit = %LineEdit_NewName
@onready var label_advise: Label = %Label_Advise
@onready var label_error: RichTextLabel = %Label_Error
@onready var label_length: Label = %Label_Length
@onready var panel_container_error_border: PanelContainer = %PanelContainer_ErrorBorder


func _on_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if event.pressed:
			close()


func close() -> void:
	line_edit_new_name.text = ""
	hide()


func open() -> void:
	var profile = Global.player_identity.get_profile_or_null()
	var address = profile.get_ethereum_address()
	new_nickname = profile.get_name()
	label_tag.text = "#" + address.substr(address.length() - 4, 4)
	line_edit_new_name.set_text(new_nickname)
	_check_error()
	show()


func _check_error() -> void:
	var color: Color = Color.WHITE
	label_length.text = (
		str(line_edit_new_name.text.length()) + "/" + str(line_edit_new_name.character_limit)
	)
	if line_edit_new_name.text.length() > line_edit_new_name.character_limit:
		color = Color.RED
	else:
		color = Color.WHITE
	label_length.label_settings.font_color = color

	if line_edit_new_name.error:
		label_error.show()
		label_advise.hide()
		label_error.text = line_edit_new_name.error_message
		button_save.disabled = true
		panel_container_error_border.self_modulate = Color.RED
	else:
		label_error.hide()
		label_advise.show()
		button_save.disabled = line_edit_new_name.text.is_empty()
		panel_container_error_border.self_modulate = Color.TRANSPARENT


func _on_button_new_link_cancel_pressed() -> void:
	close()


func _on_button_cancel_pressed() -> void:
	close()


func _on_button_save_pressed() -> void:
	ProfileHelper.get_mutable_profile().set_name(new_nickname)
	ProfileHelper.async_save_profile()
	emit_signal("update_name_on_profile", new_nickname)
	close()


func _on_line_edit_new_name_dcl_line_edit_changed() -> void:
	new_nickname = line_edit_new_name.text
	_check_error()
