class_name AsyncImage
extends Control

## Reusable image component. The image textures (loaded + no-image) carry their own rounded-corner
## shader and use their alpha directly; the Panel is only the loading skeleton and hides once done.

signal image_loaded

## Optional per-instance texture quality override.
## NONE → uses the global texture quality setting (DclConfig).
## LOW/MEDIUM/HIGH/SOURCE values mirror DclConfig.TEXTURE_QUALITY_* — keep in sync with
## TextureQuality in lib/src/godot_classes/dcl_config.rs.
enum ForcedQuality { NONE = -1, LOW = 0, MEDIUM = 1, HIGH = 2, SOURCE = 3 }

# Rounded backdrop kept behind the placeholder glyph in the no-image / failed-load state.
const NO_IMAGE_BACKGROUND: Color = Color(0.20784314, 0.03137255, 0.32941177, 0.5)

@export var forced_quality: ForcedQuality = ForcedQuality.NONE
## Corner radius (px) for the border and the image's rounded-corner shader.
@export var border_radius: int = 12:
	set(value):
		border_radius = value
		if is_node_ready():
			_apply_corner_radius()
@export var border_color: Color = Color("E8B9FF")
## Border thickness in px; 0 hides the outline. Defaults to the historical 1px.
@export var border_width: int = 1

var _image_ready: bool = false
var _is_loading: bool = true
var _skeleton_material: Material = null

@onready var panel: PanelContainer = %Panel
@onready var panel_border: PanelContainer = %Panel_Border
@onready var texture_image: TextureRect = %TextureRect_Image
@onready var texture_no_image: TextureRect = %TextureRect_NoImage


func _ready() -> void:
	_skeleton_material = panel.material
	_apply_corner_radius()
	_apply_loading_style()


## Drives the image textures' rounded-corner shader from border_radius. Materials are duplicated per
## instance so AsyncImages with different radii don't fight over one shared material.
func _apply_corner_radius() -> void:
	for tex in [texture_image, texture_no_image]:
		if is_instance_valid(tex) and tex.material is ShaderMaterial:
			var mat: ShaderMaterial = tex.material.duplicate()
			mat.set_shader_parameter("corner_radius_px", float(border_radius))
			tex.material = mat


func is_image_ready() -> bool:
	return _image_ready


func set_texture(texture: Texture2D) -> void:
	texture_image.texture = texture
	texture_image.show()
	texture_no_image.hide()
	_image_ready = true
	_is_loading = false
	_apply_loaded_style()
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
	_apply_no_image_style()
	image_loaded.emit()


func _apply_loading_style() -> void:
	if not is_inside_tree():
		return
	# Skeleton only: the panel is the shimmer; the image textures stay hidden until they resolve.
	texture_image.hide()
	texture_no_image.hide()
	if is_instance_valid(panel):
		panel.material = _skeleton_material
		var style := StyleBoxFlat.new()
		style.bg_color = Color(1, 1, 1, 0.08)
		_set_radius(style)
		panel.add_theme_stylebox_override("panel", style)
		panel.show()
	if is_instance_valid(panel_border):
		panel_border.hide()


func _apply_loaded_style() -> void:
	if not is_inside_tree():
		return
	# A real image loaded — hide the skeleton panel so the image's own alpha shows what's behind the
	# component instead of a solid backdrop.
	if is_instance_valid(panel):
		panel.hide()
	_apply_border()


func _apply_no_image_style() -> void:
	if not is_inside_tree():
		return
	# No image: keep the panel as a plain rounded backdrop behind the placeholder glyph.
	if is_instance_valid(panel):
		panel.material = null
		var style := StyleBoxFlat.new()
		style.bg_color = NO_IMAGE_BACKGROUND
		_set_radius(style)
		panel.add_theme_stylebox_override("panel", style)
		panel.show()
	_apply_border()


func _apply_border() -> void:
	if not is_instance_valid(panel_border):
		return
	var border_style := StyleBoxFlat.new()
	border_style.draw_center = false
	border_style.border_width_left = border_width
	border_style.border_width_top = border_width
	border_style.border_width_right = border_width
	border_style.border_width_bottom = border_width
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
	if forced_quality == ForcedQuality.NONE:
		content_mapping = Global.content_provider.fetch_texture_by_url(url_hash, url)
	else:
		content_mapping = Global.content_provider.fetch_texture_by_url_with_quality(
			url_hash, url, forced_quality
		)
	var result = await PromiseUtils.async_awaiter(content_mapping)
	if result is PromiseError:
		printerr("AsyncImage: download error: ", result.get_error())
		_finish_with_error()
		return
	if not is_instance_valid(self):
		return
	if result.failed:
		_finish_with_error()
		return
	set_texture(result.texture)
