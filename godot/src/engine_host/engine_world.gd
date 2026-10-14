class_name EngineWorld
extends RefCounted
## Moves the RN host between the avatar-only scene and the explorer.

const EXPLORER_SCENE := "res://src/ui/explorer.tscn"
const AVATAR_HOST_SCENE := "res://src/engine_host/avatar_host.tscn"

# RN flips the device orientation right around enter/exit; building or freeing a world while the
# Android surface is being recreated crashed the Mali driver, so wait for the window to settle.
const SURFACE_WAIT_MS := 700
const SURFACE_SETTLE_MS := 300


## Returns false when `is_cancelled` turned true while waiting, leaving the host scene untouched.
## The explorer reads the destination from the deeplink object on boot.
static func async_enter(parcel: Vector2i, realm: String, is_cancelled: Callable) -> bool:
	await _async_settle_surface()
	if is_cancelled.call():
		return false
	Global.deep_link_obj.realm = realm
	Global.deep_link_obj.location = parcel
	Global.get_viewport().transparent_bg = false
	Global.get_tree().change_scene_to_file.call_deferred(EXPLORER_SCENE)
	return true


## Same world teardown as Global.return_to_discover, but the identity stays and the next scene is
## the avatar host; orientation is left to RN.
static func async_exit() -> void:
	await _async_settle_surface()
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


static func _async_settle_surface() -> void:
	var tree := Global.get_tree()
	var window := Global.get_window()
	var size := window.size
	var start := Time.get_ticks_msec()
	var last_change := start
	var changed := false
	while true:
		await tree.process_frame
		var now := Time.get_ticks_msec()
		if window.size != size:
			size = window.size
			last_change = now
			changed = true
		if now - last_change >= SURFACE_SETTLE_MS and (changed or now - start >= SURFACE_WAIT_MS):
			return
