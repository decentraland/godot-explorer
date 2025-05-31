class_name ProfileSettings
extends Control

# Depends on backpack for the mutable profile
@export var backpack: Backpack = null

@onready var line_edit_name: LineEdit = %LineEdit_Name
@onready var text_edit_about: TextEdit = %TextEdit_About
@onready var use_claimed_name: CheckButton = %CheckButton_UseClaimedName
@onready var radio_claimed_names: RadioSelector = %RadioSelector_ClaimedNames


# gdlint:ignore = async-function-name
func _ready():
	var profile := Global.player_identity.get_profile_or_null()
	if profile != null:
		await _async_on_profile_changed(profile)
	Global.player_identity.profile_changed.connect(self._async_on_profile_changed)


func _async_on_profile_changed(new_profile: DclUserProfile):
	line_edit_name.text = new_profile.get_name()
	text_edit_about.text = new_profile.get_description()

	radio_claimed_names.clear()
	var response = await NamesRequest.async_request_all_names()
	var elements = response.elements
	for claimed_name in elements:
		radio_claimed_names.add_item(claimed_name.name)

	if radio_claimed_names.items.size() > 0:
		radio_claimed_names.select_by_item(new_profile.get_name())
		use_claimed_name.show()
		if new_profile.has_claimed_name():
			use_claimed_name.set_pressed_no_signal(true)
		else:
			use_claimed_name.set_pressed_no_signal(false)
	else:
		use_claimed_name.set_pressed_no_signal(false)
		use_claimed_name.hide()

	_on_check_button_toggled(use_claimed_name.button_pressed)


func _on_button_logout_pressed():
	Global.scene_runner.set_pause(true)
	Global.comms.disconnect(true)


func _on_control_claim_image_pressed():
	Global.open_url("https://decentraland.org/marketplace/names/claim")


func _on_text_edit_about_text_changed():
	backpack.mutable_profile.set_description(text_edit_about.text)


func _on_line_edit_name_text_changed(new_name):
	if (
		backpack.mutable_profile.get_name() != new_name
		or backpack.mutable_profile.has_claimed_name()
	):
		backpack.mutable_profile.set_name(new_name)
		backpack.mutable_profile.set_has_claimed_nlame(false)


func _on_check_button_toggled(toggled_on):
	line_edit_name.visible = not toggled_on
	radio_claimed_names.visible = toggled_on

	if toggled_on:
		radio_claimed_names.select_by_item(backpack.mutable_profile.get_name())
	else:
		_on_line_edit_name_text_changed(line_edit_name.text)


func _on_radio_selector_claimed_names_select_item(_index: int, item: String) -> void:
	var new_name = item
	if (
		backpack.mutable_profile.get_name() != new_name
		or !backpack.mutable_profile.has_claimed_name()
	):
		backpack.mutable_profile.set_name(new_name)
		backpack.mutable_profile.set_has_claimed_nlame(true)
