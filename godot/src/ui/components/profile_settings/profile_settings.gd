class_name ProfileSettings
extends Control

# Depends on backpack for the mutable profile
@export var backpack: Backpack = null

@onready var line_edit_name: LineEdit = %LineEdit_Name
@onready var text_edit_about: TextEdit = %TextEdit_About
@onready var claimed_names: OptionButton = %ItemList_ClaimedNames
@onready var use_claimed_name: CheckButton = %CheckButton_UseClaimedName


# Called when the node enters the scene tree for the first time.
func _ready():
	Global.player_identity.profile_changed.connect(self._async_on_profile_changed)


func _async_on_profile_changed(new_profile: DclUserProfile):
	line_edit_name.text = new_profile.get_name()
	text_edit_about.text = new_profile.get_description()

	claimed_names.clear()
	var response = await NamesRequest.async_request_all_names()
	var elements = response.elements
	for claimed_name in elements:
		claimed_names.add_item(claimed_name.name)

	if claimed_names.get_item_count() > 0:
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


func _on_item_list_claimed_names_item_selected(index):
	var new_name = claimed_names.get_item_text(index)
	if (
		backpack.mutable_profile.get_name() != new_name
		or !backpack.mutable_profile.has_claimed_name()
	):
		backpack.mutable_profile.set_name(new_name)
		backpack.mutable_profile.set_has_claimed_nlame(true)


func _on_check_button_toggled(toggled_on):
	line_edit_name.visible = not toggled_on
	claimed_names.visible = toggled_on

	if toggled_on:
		_on_item_list_claimed_names_item_selected(claimed_names.get_selected_id())
	else:
		_on_line_edit_name_text_changed(line_edit_name.text)
