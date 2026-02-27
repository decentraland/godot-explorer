extends Control

const AVATAR_PREVIEW_SCENE: PackedScene = preload(
	"res://src/ui/components/backpack/avatar_preview.tscn"
)

var _avatar_preview: AvatarPreview
var _opened_from_landscape: bool = false

@onready var draggable_bottom_sheet: DraggableBottomSheet = %DraggableBottomSheet
@onready var avatar_container: MarginContainer = %SafeTopMarginContainer
@onready var canvas_layer: CanvasLayer = $CanvasLayer
@onready var profile_editor = %ProfileEditor


func _ready() -> void:
	var content = draggable_bottom_sheet.get_content_instance()
	if content:
		content.link_clicked.connect(_on_link_clicked)
		content.emote_pressed.connect(_on_emote_pressed)
		content.stop_emote.connect(_on_stop_emote)
		content.edit_profile_pressed.connect(show_editor)

	profile_editor.close_requested.connect(_on_close_editor)
	profile_editor.save_failed.connect(_on_save_failed)


func _on_visibility_changed() -> void:
	if visible:
		hide_editor()
		_show_avatar()
		_refresh_content()
	else:
		hide_editor()
		_free_avatar()


func show_editor(from_landscape: bool = false) -> void:
	_opened_from_landscape = from_landscape
	var profile := Global.player_identity.get_profile_or_null()
	if profile != null:
		profile_editor.populate(profile)
	canvas_layer.visible = true


func hide_editor() -> void:
	if canvas_layer != null:
		canvas_layer.visible = false


func _on_close_editor(saved: bool = false) -> void:
	hide_editor()
	if saved:
		Global.player_identity.set_profile(Global.player_identity.get_mutable_profile())
		_refresh_content_from_mutable()
	if _opened_from_landscape:
		_opened_from_landscape = false
		Global.set_orientation_landscape()
		Global.close_menu.emit()
		Global.open_own_profile.emit()


func _show_avatar() -> void:
	if _avatar_preview != null:
		return
	_avatar_preview = AVATAR_PREVIEW_SCENE.instantiate()
	_avatar_preview.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_avatar_preview.size_flags_vertical = Control.SIZE_EXPAND_FILL
	_avatar_preview.stretch = true
	_avatar_preview.hide_name = true
	avatar_container.add_child(_avatar_preview)
	var profile: DclUserProfile = Global.player_identity.get_profile_or_null()
	if profile != null:
		_avatar_preview.avatar.async_update_avatar_from_profile(profile)


func _free_avatar() -> void:
	if _avatar_preview == null:
		return
	avatar_container.remove_child(_avatar_preview)
	_avatar_preview.queue_free()
	_avatar_preview = null


func _refresh_content() -> void:
	var profile: DclUserProfile = Global.player_identity.get_profile_or_null()
	if profile == null:
		return
	var content = draggable_bottom_sheet.get_content_instance()
	if content and content.has_method("refresh"):
		content.refresh(profile)


func _refresh_content_from_mutable() -> void:
	var profile: DclUserProfile = Global.player_identity.get_mutable_profile()
	if profile == null:
		return
	var content = draggable_bottom_sheet.get_content_instance()
	if content and content.has_method("refresh"):
		content.refresh(profile)


func _on_save_failed() -> void:
	_refresh_content()


func _on_emote_pressed(urn: String) -> void:
	if _avatar_preview == null:
		return
	_avatar_preview.reset_avatar_rotation()
	_avatar_preview.avatar.emote_controller.stop_emote()
	if not _avatar_preview.avatar.emote_controller.is_playing():
		_avatar_preview.avatar.emote_controller.async_play_emote(urn)


func _on_stop_emote() -> void:
	if _avatar_preview == null:
		return
	_avatar_preview.avatar.emote_controller.stop_emote()


func _on_link_clicked(url: String) -> void:
	OS.shell_open(url)
