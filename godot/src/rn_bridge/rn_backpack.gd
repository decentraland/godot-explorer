class_name RnBackpack
extends Node
## VIBE-CODED SPIKE: owns the single avatar preview the React Native screens draw through a hole.

signal avatar_changed(state: Dictionary)

const PREVIEW_SCENE := preload("res://src/ui/pages/backpack/avatar_preview.tscn")
const EMOTE_SLOTS := 10

# In explorer mode only the preview is drawn by the main viewport (nameplates and other 2D layers
# are culled); disable_3d stops the world render.
const OVERLAY_CANVAS_LAYER := 1 << 19

var _layer: CanvasLayer = null
var _canvas_cull_mask := 0
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
		_preview.visibility_layer = OVERLAY_CANVAS_LAYER
	_show_preview(true)
	place(rect)
	await _async_refresh_avatar()
	place(rect)
	_emit_avatar()


func place(rect: Rect2) -> void:
	if _preview == null:
		return
	_preview.set_anchors_preset(Control.PRESET_TOP_LEFT)
	# RN reports device pixels; Godot positions UI in its scaled viewport units.
	var scale := get_viewport().get_visible_rect().size.x / DisplayServer.window_get_size().x
	_preview.position = rect.position * scale
	_preview.size = rect.size * scale


func close() -> void:
	if _preview != null:
		_show_preview(false)


func async_equip(urn: String) -> void:
	var avatar = Global.player_identity.get_mutable_avatar()
	if avatar == null:
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
	await _async_commit()


# Categories the engine forces back to defaults are kept: removing them would be undone anyway.
func async_unequip(urn: String) -> void:
	var avatar = Global.player_identity.get_mutable_avatar()
	if avatar == null:
		return
	var item = Global.content_provider.get_wearable(urn)
	if item != null and item.get_category() in Wearables.Categories.REQUIRED_CATEGORIES:
		return
	var kept := PackedStringArray()
	for existing in avatar.get_wearables():
		if existing != urn:
			kept.append(existing)
	avatar.set_wearables(kept)
	await _async_commit()


func async_set_body_shape(urn: String) -> void:
	var avatar = Global.player_identity.get_mutable_avatar()
	if avatar == null:
		return
	avatar.set_body_shape(urn)
	await _async_commit()


func async_set_color(target: String, html: String) -> void:
	var avatar = Global.player_identity.get_mutable_avatar()
	if avatar == null or not Color.html_is_valid(html):
		return
	var color := Color.html(html)
	match target:
		"skin":
			avatar.set_skin_color(color)
		"hair":
			avatar.set_hair_color(color)
		"eyes":
			avatar.set_eyes_color(color)
		_:
			return
	await _async_commit()


func play_emote(urn: String) -> void:
	if _preview == null:
		return
	var id := Emotes.get_base_emote_id_from_urn(urn) if Emotes.is_base_emote_urn(urn) else urn
	_preview.avatar.emote_controller.async_play_emote(id)


func set_emote_slot(slot: int, urn: String) -> void:
	var avatar = Global.player_identity.get_mutable_avatar()
	if avatar == null or slot < 0 or slot >= EMOTE_SLOTS:
		return
	var emotes: PackedStringArray = avatar.get_emotes()
	emotes[slot] = urn
	avatar.set_emotes(emotes)
	_emit_avatar()


func async_randomize() -> void:
	Global.player_identity.set_random_profile()
	await Global.player_identity.profile_changed
	await _async_commit()


func async_set_profile_name(new_name: String) -> void:
	var profile = Global.player_identity.get_mutable_profile()
	if profile == null:
		return
	profile.set_name(new_name)
	if _preview != null:
		await _async_refresh_avatar()


func _async_commit() -> void:
	await _async_refresh_avatar()
	_emit_avatar()


func _show_preview(on: bool) -> void:
	_preview.visible = on
	_preview.subviewport.render_target_update_mode = (
		SubViewport.UPDATE_WHEN_VISIBLE if on else SubViewport.UPDATE_DISABLED
	)
	var explorer := Global.get_explorer()
	_set_overlay(on, explorer == null)
	if explorer == null:
		return
	get_viewport().disable_3d = on
	var culling := get_viewport().canvas_cull_mask == OVERLAY_CANVAS_LAYER
	if on and not culling:
		_canvas_cull_mask = get_viewport().canvas_cull_mask
		get_viewport().canvas_cull_mask = OVERLAY_CANVAS_LAYER
	elif not on and culling:
		get_viewport().canvas_cull_mask = _canvas_cull_mask
	explorer.ui_root.visible = not on


# The host's Godot surface goes above the RN view while the preview shows, so the avatar draws
# over RN content; in explorer mode the root is also made transparent for that.
func _set_overlay(on: bool, avatar_only: bool) -> void:
	if not avatar_only:
		get_viewport().transparent_bg = on
	if Engine.has_singleton("RnHostPlugin"):
		Engine.get_singleton("RnHostPlugin").call("set_surface_on_top", on)


func _async_refresh_avatar() -> void:
	var profile = Global.player_identity.get_mutable_profile()
	var avatar = Global.player_identity.get_mutable_avatar()
	if profile == null or avatar == null or _preview == null:
		return
	profile.set_avatar(avatar)
	await _preview.avatar.async_update_avatar_from_profile(profile)


func _emit_avatar() -> void:
	var avatar = Global.player_identity.get_mutable_avatar()
	if avatar == null:
		return
	(
		avatar_changed
		. emit(
			{
				"body_shape": avatar.get_body_shape(),
				"wearables": Array(avatar.get_wearables()),
				"emotes": Array(avatar.get_emotes()),
				"skin": "#" + avatar.get_skin_color().to_html(false),
				"hair": "#" + avatar.get_hair_color().to_html(false),
				"eyes": "#" + avatar.get_eyes_color().to_html(false),
			}
		)
	)
