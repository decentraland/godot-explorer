class_name EngineBackpack
extends Node
## Owns the single avatar preview the React Native screens draw through a hole.

signal preload_finished

const PREVIEW_SCENE := preload("res://src/ui/pages/backpack/avatar_preview.tscn")

# In explorer mode only the preview is drawn by the main viewport (nameplates and other 2D layers
# are culled); disable_3d stops the world render.
const OVERLAY_CANVAS_LAYER := 1 << 19

var _layer: CanvasLayer = null
var _canvas_cull_mask := 0
var _preview: AvatarPreview = null
var _preload_pending := false
var _preloaded_for := ""


## Builds the avatar in a hidden preview so the screen that shows it only has to place it.
func async_preload() -> void:
	if _preload_pending or _preview != null:
		return
	_preload_pending = true
	_ensure_preview()
	_preview.visible = false
	_preview.subviewport.render_target_update_mode = SubViewport.UPDATE_DISABLED
	var address: String = Global.player_identity.get_address_str()
	if await _async_refresh_avatar():
		_preloaded_for = address
	_preload_pending = false
	preload_finished.emit()


func async_open(rect: Rect2) -> void:
	_ensure_preview()
	_show_preview(true)
	place(rect)
	if _preload_pending:
		await preload_finished
	if _preloaded_for == Global.player_identity.get_address_str():
		_preloaded_for = ""
	else:
		await _async_refresh_avatar()
	place(rect)
	_emit_avatar()


func _ensure_preview() -> void:
	if _preview != null:
		return
	_layer = CanvasLayer.new()
	_layer.layer = 1
	add_child(_layer)
	_preview = PREVIEW_SCENE.instantiate()
	_preview.fit_avatar = true
	_preview.stretch = true
	_layer.add_child(_preview)
	_preview.visibility_layer = OVERLAY_CANVAS_LAYER


func is_shown() -> bool:
	return _preview != null and _preview.visible


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


## Renders the avatar the core computed. The core already holds it, so the engine reports back only
## when it ended up with something else (it forces defaults for missing required categories).
func async_apply_avatar(args: Dictionary) -> void:
	var avatar = Global.player_identity.get_mutable_avatar()
	if avatar == null:
		return
	avatar.set_body_shape(args.body_shape)
	avatar.set_wearables(PackedStringArray(args.wearables))
	avatar.set_emotes(PackedStringArray(args.emotes))
	avatar.set_skin_color(Color.html(args.skin))
	avatar.set_hair_color(Color.html(args.hair))
	avatar.set_eyes_color(Color.html(args.eyes))
	await PromiseUtils.async_all(
		Global.content_provider.fetch_wearables(
			avatar.get_wearables(), Global.realm.get_profile_content_url()
		)
	)
	await _async_refresh_avatar()
	if not _same_wearables(avatar.get_wearables(), args.wearables):
		_emit_avatar()


func _same_wearables(a: PackedStringArray, b: PackedStringArray) -> bool:
	var left := Array(a)
	var right := Array(b)
	left.sort()
	right.sort()
	return left == right


func focus_camera(target: String) -> void:
	if _preview == null:
		return
	var category := ""
	match target:
		"head":
			category = "hair"
		"torso":
			category = "upper_body"
		"legs":
			category = "lower_body"
		"feet":
			category = "feet"
		"hands":
			category = "hands"
	_preview.focus_camera_on(category)


func play_emote(urn: String) -> void:
	if _preview == null:
		return
	var id := Emotes.get_base_emote_id_from_urn(urn) if Emotes.is_base_emote_urn(urn) else urn
	_preview.avatar.async_play_emote(id, -1, -1)


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


# While the preview shows the avatar draws over RN content: the host puts the Godot surface above
# the RN view (the core's avatar_preview_mode), and in explorer mode the root is transparent too.
func _set_overlay(on: bool, avatar_only: bool) -> void:
	if not avatar_only:
		get_viewport().transparent_bg = on


func _async_refresh_avatar() -> bool:
	var profile = Global.player_identity.get_mutable_profile()
	var avatar = Global.player_identity.get_mutable_avatar()
	if profile == null or avatar == null or _preview == null:
		return false
	profile.set_avatar(avatar)
	await PromiseUtils.async_awaiter(_preview.avatar.async_update_avatar_from_profile(profile))
	return true


func _emit_avatar() -> void:
	var avatar = Global.player_identity.get_mutable_avatar()
	if avatar == null:
		return
	Global.engine.report_avatar(
		avatar.get_body_shape(),
		avatar.get_wearables(),
		avatar.get_emotes(),
		"#" + avatar.get_skin_color().to_html(false),
		"#" + avatar.get_hair_color().to_html(false),
		"#" + avatar.get_eyes_color().to_html(false)
	)
