class_name AsyncImage
extends Control

## Reusable image component with loading skeleton and async fetch.
## Single panel that switches style between loading (skeleton) and loaded (border).

signal image_loaded

## ORIGINAL fetches at full resolution (bypasses global texture quality).
## DEFAULT uses the global texture quality setting (typically Medium = 512px max).
enum Quality { ORIGINAL, DEFAULT }

@export var quality: Quality = Quality.ORIGINAL
@export var border_radius: int = 12
@export var border_color: Color = Color("E8B9FF")
@export var background_color: Color = Color(0.20784314, 0.03137255, 0.32941177, 0.5)

var _image_ready: bool = false
var _is_loading: bool = true
var _skeleton_material: Material = null

@onready var panel: PanelContainer = %Panel
@onready var panel_border: PanelContainer = %Panel_Border
@onready var texture_image: TextureRect = %TextureRect_Image
@onready var texture_no_image: TextureRect = %TextureRect_NoImage


func _ready() -> void:
	_skeleton_material = panel.material
	_apply_loading_style()


func is_image_ready() -> bool:
	return _image_ready


func set_texture(texture: Texture2D) -> void:
	texture_image.texture = texture
	texture_image.show()
	texture_no_image.hide()
	_image_ready = true
	_is_loading = false
	_apply_loaded_style(Color.WHITE)
	image_loaded.emit()


func load_from_url(url: String) -> void:
	_is_loading = true
	_image_ready = false
	_apply_loading_style()
	if url.is_empty():
		_finish_with_error()
		return
	_async_download(url)


func show_error() -> void:
	_finish_with_error()


func _finish_with_error() -> void:
	texture_image.hide()
	texture_no_image.show()
	_image_ready = true
	_is_loading = false
	_apply_loaded_style(background_color)
	image_loaded.emit()


func _apply_loading_style() -> void:
	if not is_inside_tree():
		return
	if is_instance_valid(panel):
		panel.material = _skeleton_material
		var style := StyleBoxFlat.new()
		style.bg_color = Color(1, 1, 1, 0.08)
		_set_radius(style)
		panel.add_theme_stylebox_override("panel", style)
	if is_instance_valid(panel_border):
		panel_border.hide()


func _apply_loaded_style(bg: Color) -> void:
	if not is_inside_tree():
		return
	if is_instance_valid(panel):
		panel.material = null
		var style := StyleBoxFlat.new()
		style.bg_color = bg
		_set_radius(style)
		panel.add_theme_stylebox_override("panel", style)
	if is_instance_valid(panel_border):
		var border_style := StyleBoxFlat.new()
		border_style.draw_center = false
		border_style.border_width_left = 1
		border_style.border_width_top = 1
		border_style.border_width_right = 1
		border_style.border_width_bottom = 1
		border_style.border_color = border_color
		_set_radius(border_style)
		panel_border.add_theme_stylebox_override("panel", border_style)
		panel_border.show()


func _set_radius(style: StyleBoxFlat) -> void:
	style.corner_radius_top_left = border_radius
	style.corner_radius_top_right = border_radius
	style.corner_radius_bottom_left = border_radius
	style.corner_radius_bottom_right = border_radius


static func _get_hash_from_url(url: String) -> String:
	if url.contains("/content/contents/"):
		var parts = url.split("/")
		return parts[parts.size() - 1]
	var context := HashingContext.new()
	if context.start(HashingContext.HASH_SHA256) == OK:
		context.update(url.to_utf8_buffer())
		return context.finish().hex_encode()
	return "temp-file"


func _async_download(url: String) -> void:
	var url_hash := _get_hash_from_url(url)
	var content_mapping
	if quality == Quality.ORIGINAL:
		content_mapping = Global.content_provider.fetch_texture_by_url_original(url_hash, url)
	else:
		content_mapping = Global.content_provider.fetch_texture_by_url(url_hash, url)
	var result = await PromiseUtils.async_awaiter(content_mapping)
	if result is PromiseError:
		printerr("AsyncImage: download error: ", result.get_error())
		_finish_with_error()
		return
	if not is_instance_valid(self):
		return
	set_texture(result.texture)
