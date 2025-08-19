class_name ProfileSettings
extends Control

# Depends on backpack for the mutable profile
@export var backpack: Backpack = null

@onready var line_edit_name: LineEdit = %LineEdit_Name
@onready var text_edit_about: TextEdit = %TextEdit_About
@onready var use_claimed_name: CheckButton = %CheckButton_UseClaimedName
@onready var radio_claimed_names: RadioSelector = %RadioSelector_ClaimedNames
@onready var passport: Control = $Profile


# gdlint:ignore = async-function-name
func _ready():
	var profile := Global.player_identity.get_profile_or_null()
	if profile != null:
		await _async_on_profile_changed(profile)
	Global.player_identity.profile_changed.connect(self._async_on_profile_changed)


func _async_on_profile_changed(profile: DclUserProfile):
	await passport.async_show_profile(profile)
	
func _on_button_logout_pressed():
	Global.scene_runner.set_pause(true)
	Global.comms.disconnect(true)
