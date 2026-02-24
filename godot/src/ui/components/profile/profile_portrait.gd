extends Control

const AVATAR_PREVIEW_SCENE: PackedScene = preload(
	"res://src/ui/components/backpack/avatar_preview.tscn"
)

@onready var draggable_bottom_sheet: DraggableBottomSheet = %DraggableBottomSheet
@onready var avatar_container: MarginContainer = %SafeTopMarginContainer

var _avatar_preview: AvatarPreview


func _ready() -> void:
	var content = draggable_bottom_sheet.get_content_instance()
	if content:
		content.link_clicked.connect(_on_link_clicked)


func _on_visibility_changed() -> void:
	if visible:
		_show_avatar()
		_refresh_content()
	else:
		_free_avatar()


func _show_avatar() -> void:
	if _avatar_preview != null:
		return
	_avatar_preview = AVATAR_PREVIEW_SCENE.instantiate()
	_avatar_preview.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_avatar_preview.size_flags_vertical = Control.SIZE_EXPAND_FILL
	_avatar_preview.stretch = true
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


func _on_link_clicked(url: String) -> void:
	OS.shell_open(url)
