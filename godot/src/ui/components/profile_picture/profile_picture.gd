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

@onready var texture_rect_profile: TextureRect = %TextureRect_Profile
@onready var panel_border: PanelContainer = %Panel_Border


func _ready() -> void:
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


func async_update_profile_picture(avatar_ifo: DclAvatar):
	avatar = avatar_ifo
	var avatar_name = avatar_ifo.get_avatar_name()
	var nickname_color = avatar_ifo.get_nickname_color(avatar_name)

	var background_color = nickname_color
	apply_style(background_color)

	# Skip image loading in editor mode
	if Engine.is_editor_hint():
		return

	var avatar_data = avatar_ifo.get_avatar_data()
	if avatar_data == null:
		printerr("Profile picture: avatar_data is null")
		return

	var face256_value = avatar_data.to_godot_dictionary()["snapshots"]["face256"]
	var hash = ""
	var url = ""
	if face256_value.begins_with("http"):
		var parts = face256_value.split("/")
		hash = parts[4]
		url = face256_value
	else:
		hash = face256_value
		url = "https://profile-images.decentraland.org/entities/%s/face.png" % hash

	if hash.is_empty() or url.is_empty():
		printerr("Profile picture: missing face256 data")
		return

	var promise = Global.content_provider.fetch_texture_by_url(hash, url)
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
	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_LEFT and event.pressed:
			if avatar != null and is_instance_valid(avatar):
				var explorer = Global.get_explorer()
				if avatar.avatar_id == Global.player_identity.get_address_str():
					explorer.control_menu.show_own_profile()
				else:
					Global.emit_signal("open_profile",avatar)
