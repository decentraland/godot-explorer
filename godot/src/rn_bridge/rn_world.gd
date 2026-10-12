class_name RnWorld
extends RefCounted
## VIBE-CODED SPIKE: moves the RN host between the avatar-only scene and the explorer.

const EXPLORER_SCENE := "res://src/ui/explorer.tscn"
const AVATAR_HOST_SCENE := "res://src/rn_bridge/rn_avatar_host.tscn"


## The explorer reads the destination from the deeplink object on boot.
static func enter(parcel: Vector2i, realm: String) -> void:
	Global.deep_link_obj.realm = realm
	Global.deep_link_obj.location = parcel
	Global.get_viewport().transparent_bg = false
	Global.get_tree().change_scene_to_file.call_deferred(EXPLORER_SCENE)


## Same world teardown as Global.return_to_discover, but the identity stays and the next scene is
## the avatar host; orientation is left to RN.
static func exit() -> void:
	var explorer := Global.get_explorer()
	if explorer != null:
		explorer.prepare_for_logout()
	Global.modal_manager.close_current_modal()
	NotificationsManager.stop_polling()
	if Global.analytics_controller != null:
		Global.analytics_controller.cancel_first_move_poll()
	Global.scene_runner.kill_all_scenes()
	Global.realm.async_clear_realm()
	Global.scene_fetcher.reset_for_logout()
	Global.comms.disconnect(false)
	Global.scene_runner.detach_base_ui()
	Global.get_tree().change_scene_to_file.call_deferred(AVATAR_HOST_SCENE)
