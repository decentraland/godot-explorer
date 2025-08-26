extends ColorRect

signal update_name_on_profile(nickname: String)

var new_nickname: String

@onready var button_cancel: Button = %Button_Cancel
@onready var button_save: Button = %Button_Save
@onready var button_claim_name: Button = %Button_ClaimName
@onready var dcl_text_edit_new_nick: VBoxContainer = %DclTextEdit_NewNick
@onready var label_tag: Label = %Label_Tag


func _on_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if event.pressed:
			close()


func close() -> void:
	dcl_text_edit_new_nick.text_edit.text = ""
	hide()


func open() -> void:
	var profile = Global.player_identity.get_profile_or_null()
	var address = profile.get_ethereum_address()
	new_nickname = profile.get_name()
	label_tag.text = "#" + address.substr(address.length() - 4, 4)
	dcl_text_edit_new_nick.set_text(new_nickname)
	_check_error()
	show()


func _check_error() -> void:
	if dcl_text_edit_new_nick.error or dcl_text_edit_new_nick.text_edit.text.length() <= 0:
		button_save.disabled = true
	else:
		button_save.disabled = false


func _on_button_new_link_cancel_pressed() -> void:
	close()


func _on_dcl_text_edit_new_nick_dcl_text_edit_changed() -> void:
	new_nickname = dcl_text_edit_new_nick.text_edit.text
	_check_error()


func _on_button_cancel_pressed() -> void:
	close()


func _on_button_save_pressed() -> void:
	ProfileHelper.get_mutable_profile().set_name(new_nickname)
	ProfileHelper.async_save_profile(false)
	emit_signal("update_name_on_profile", new_nickname)
	close()
