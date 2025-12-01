@tool
class_name ProfilePicture
extends Control

enum Size { EXTRA_LARGE, LARGE, MEDIUM, SMALL }
const DECENTRALAND_LOGO = preload("res://decentraland_logo.png")

@export var picture_size: Size = Size.MEDIUM:
	set(value):
		picture_size = value
		_update_size()
		if Engine.is_editor_hint():
			notify_property_list_changed()

var border_width: int
var avatar: DclAvatar
var connection_status_online = load("res://assets/ui/connection_status_online.svg")
var connection_status_offline = load("res://assets/ui/connection_status_offline.svg")

@onready var texture_rect_profile: TextureRect = %TextureRect_Profile
@onready var panel_border: PanelContainer = %Panel_Border
@onready var texture_rect_status: TextureRect = %TextureRect_Status
@onready var texture_rect_friendship: TextureRect = %TextureRect_Friendship


func _ready() -> void:
	hide_status()
	_update_size()
	if panel_border:
		_update_border_style()


func _get_configuration_warnings():
	# This forces the editor to refresh when properties change
	return []


func _update_size() -> void:
	var size_px: int
	var border_px: int

	match picture_size:
		Size.EXTRA_LARGE:
			size_px = 60
			border_px = 3
		Size.LARGE:
			size_px = 40
			border_px = 2
		Size.MEDIUM:
			size_px = 32
			border_px = 2
		Size.SMALL:
			size_px = 28
			border_px = 2

	# Update the border width property
	border_width = border_px

	# Set the custom minimum size
	custom_minimum_size = Vector2(size_px, size_px)

	# Force size update in editor and runtime
	size = Vector2(size_px, size_px)

	if Engine.is_editor_hint():
		# Force immediate update in editor
		queue_redraw()
		# Notify editor of changes
		set_notify_transform(true)

	# Update border style if nodes are ready
	if is_node_ready() and has_node("%Panel_Border"):
		_update_border_style()


func _update_border_style() -> void:
	if not panel_border:
		return

	var stylebox_border_panel := panel_border.get_theme_stylebox("panel")
	if not stylebox_border_panel:
		return

	stylebox_border_panel = stylebox_border_panel.duplicate()
	if stylebox_border_panel is StyleBoxFlat:
		stylebox_border_panel.border_width_bottom = border_width
		stylebox_border_panel.border_width_left = border_width
		stylebox_border_panel.border_width_top = border_width
		stylebox_border_panel.border_width_right = border_width
	panel_border.add_theme_stylebox_override("panel", stylebox_border_panel)


func async_update_profile_picture(data: SocialItemData):
	var nickname_color = DclAvatar.get_nickname_color(data.name)

	var background_color = nickname_color
	apply_style(background_color)

	# Skip image loading in editor mode
	if Engine.is_editor_hint():
		return

	if data.profile_picture_url.is_empty():
		return

	# Use address-based hash for caching, or fallback to avatar_name
	var texture_hash = (
		data.address + "_face" if not data.address.is_empty() else data.avatar_name + "_face"
	)
	var promise = Global.content_provider.fetch_texture_by_url(
		texture_hash, data.profile_picture_url
	)
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		printerr("profile_picture::_async_download_image promise error: ", result.get_error())
		return
	texture_rect_profile.texture = result.texture


func set_dcl_logo() -> void:
	texture_rect_profile.texture = DECENTRALAND_LOGO
	apply_style(Color.GREEN)


func apply_style(color: Color) -> void:
	# Apply background color to the main panel container
	var stylebox_background := get_theme_stylebox("panel")
	stylebox_background = stylebox_background.duplicate()
	if stylebox_background is StyleBoxFlat:
		stylebox_background.bg_color = color
	add_theme_stylebox_override("panel", stylebox_background)

	# Apply border color to the border panel
	var factor = 0.3
	var border_color = color.lerp(Color.WHITE, factor)

	var stylebox_border := panel_border.get_theme_stylebox("panel")
	stylebox_border = stylebox_border.duplicate()
	if stylebox_border is StyleBoxFlat:
		stylebox_border.border_color = border_color
		# Ensure border width is correctly applied
		stylebox_border.border_width_bottom = border_width
		stylebox_border.border_width_left = border_width
		stylebox_border.border_width_top = border_width
		stylebox_border.border_width_right = border_width
	panel_border.add_theme_stylebox_override("panel", stylebox_border)


func _on_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if event.pressed:
			if avatar != null and is_instance_valid(avatar):
				var explorer = Global.get_explorer()
				if avatar.avatar_id == Global.player_identity.get_address_str():
					explorer.control_menu.show_own_profile()
				else:
					Global.open_profile_by_avatar.emit(avatar)


func set_online() -> void:
	texture_rect_status.show()
	texture_rect_status.texture = connection_status_online


func set_offline() -> void:
	texture_rect_status.show()
	texture_rect_status.texture = connection_status_offline


func set_friend() -> void:
	texture_rect_friendship.show()

func hide_status() -> void:
	texture_rect_status.hide()
	texture_rect_friendship.hide()
