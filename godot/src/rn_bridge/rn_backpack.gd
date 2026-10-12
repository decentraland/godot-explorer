class_name RnBackpack
extends Node
## VIBE-CODED SPIKE: shows the avatar preview in the rectangle the React Native backpack leaves open.

const PREVIEW_SCENE := preload("res://src/ui/pages/backpack/avatar_preview.tscn")

var _layer: CanvasLayer = null
var _preview: AvatarPreview = null


func async_open(rect: Rect2) -> void:
	if _preview == null:
		_layer = CanvasLayer.new()
		_layer.layer = 1
		add_child(_layer)
		_preview = PREVIEW_SCENE.instantiate()
		_preview.fit_avatar = true
		_preview.stretch = true
		_layer.add_child(_preview)
		get_viewport().disable_3d = true
		_set_overlay(true)
		Global.get_explorer().ui_root.hide()
		place(rect)
		await _async_refresh_avatar()
	place(rect)


func place(rect: Rect2) -> void:
	if _preview == null:
		return
	_preview.set_anchors_preset(Control.PRESET_TOP_LEFT)
	# RN reports device pixels; Godot positions UI in its scaled viewport units.
	var scale := get_viewport().get_visible_rect().size.x / DisplayServer.window_get_size().x
	_preview.position = rect.position * scale
	_preview.size = rect.size * scale


func close() -> void:
	if _layer == null:
		return
	get_viewport().disable_3d = false
	_set_overlay(false)
	Global.get_explorer().ui_root.show()
	_layer.queue_free()
	_layer = null
	_preview = null


# Transparent root + the host's Godot surface above the RN view: the avatar draws over RN content.
func _set_overlay(on: bool) -> void:
	get_viewport().transparent_bg = on
	if Engine.has_singleton("RnHostPlugin"):
		Engine.get_singleton("RnHostPlugin").call("set_surface_on_top", on)


func async_equip(urn: String) -> void:
	var avatar = Global.player_identity.get_mutable_avatar()
	if _preview == null or avatar == null:
		return
	var current: PackedStringArray = avatar.get_wearables()
	var wanted := PackedStringArray(current)
	wanted.append(urn)
	await PromiseUtils.async_all(
		Global.content_provider.fetch_wearables(wanted, Global.realm.get_profile_content_url())
	)
	var category: String = Global.content_provider.get_wearable(urn).get_category()
	var kept := PackedStringArray()
	for existing in current:
		var item = Global.content_provider.get_wearable(existing)
		if item == null or item.get_category() != category:
			kept.append(existing)
	kept.append(urn)
	avatar.set_wearables(kept)
	await _async_refresh_avatar()


func _async_refresh_avatar() -> void:
	var profile = Global.player_identity.get_mutable_profile()
	profile.set_avatar(Global.player_identity.get_mutable_avatar())
	await _preview.avatar.async_update_avatar_from_profile(profile)
