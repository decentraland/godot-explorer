extends ColorRect

signal update_name_on_profile(nickname: String)

var new_nickname: String

@onready var button_cancel: Button = %Button_Cancel
@onready var button_save: Button = %Button_Save
@onready var button_claim_name: Button = %Button_ClaimName
@onready var label_tag: Label = %Label_Tag
@onready var claim_name_container: MarginContainer = %ClaimNameContainer
@onready var dcl_line_edit: VBoxContainer = %DclLineEdit


func _ready():
	claim_name_container.visible = !Global.is_ios()


func _on_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if event.pressed:
			close()


func close() -> void:
	dcl_line_edit.line_edit.text = ""
	hide()


func open() -> void:
	var profile = Global.player_identity.get_profile_or_null()
	var address = profile.get_ethereum_address()
	new_nickname = profile.get_name()
	label_tag.text = "#" + address.substr(address.length() - 4, 4)
	dcl_line_edit.set_text_value(new_nickname)
	show()


func _on_button_new_link_cancel_pressed() -> void:
	close()


func _on_button_cancel_pressed() -> void:
	close()


func _on_button_save_pressed() -> void:
	#ProfileHelper.get_mutable_profile().set_name(new_nickname)
	#ProfileHelper.async_save_profile()
	emit_signal("update_name_on_profile", new_nickname)
	close()


func _on_button_claim_name_pressed() -> void:
	Global.open_url("https://decentraland.org/marketplace/names/claim")


func _on_dcl_line_edit_dcl_line_edit_changed() -> void:
	new_nickname = dcl_line_edit.line_edit.text
	button_save.disabled = dcl_line_edit.error
