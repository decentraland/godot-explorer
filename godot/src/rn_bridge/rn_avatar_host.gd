extends Node
## VIBE-CODED SPIKE: scene of the React Native host's avatar-only mode. No world is loaded; the
## engine only renders the preview that RnBackpack places over the RN screens.


func _ready() -> void:
	get_viewport().transparent_bg = true
	_async_boot()


func _async_boot() -> void:
	Global.player_identity.create_disposable_account()
	# The guest wallet's own default-profile pass is deferred; the random profile must land after it.
	await get_tree().process_frame
	await get_tree().process_frame
	Global.player_identity.set_random_profile()
	await Global.player_identity.profile_changed
	SplashOverlay.fade_out()
	RnBridge.on_avatar_host_ready()
