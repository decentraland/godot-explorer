class_name ProfileSettings
extends Control

# Depends on backpack for the mutable profile
@export var backpack: Backpack = null

@onready var passport: Control = $Profile


# gdlint:ignore = async-function-name
func _ready():
	var profile := Global.player_identity.get_profile_or_null()
	Global.player_identity.profile_changed.connect(self._async_on_profile_changed)
	if profile != null:
		await _async_on_profile_changed(profile)


func _async_on_profile_changed(profile: DclUserProfile):
		# ADR-290: Generate local snapshots if not available from server
	await Global.snapshot.async_generate_for_avatar(profile.get_avatar(), profile)
	await passport.async_show_profile(profile)


func _on_button_logout_pressed():
	Global.scene_runner.set_pause(true)
	Global.comms.disconnect(true)
