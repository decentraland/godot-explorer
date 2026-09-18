@tool
class_name ScreenshotSlot
extends Control

## One tile in an attachment strip: either a picked image with a delete badge,
## or an empty "add" tile.
##
## Both states share the same footprint so a row of slots keeps its rhythm as
## images are added and removed. The caller sets `variant` (or just calls
## `set_image()` / `clear()`, which set it implicitly) and listens for the two
## signals; it never has to build or restyle nodes.

signal delete_pressed
signal add_pressed
## Emitted when a FILLED tile is tapped — the caller re-runs the picker and
## swaps this slot's image. The delete badge sits above and takes its own taps.
signal replace_pressed

enum Variant { FILLED, EMPTY }

@export var variant: Variant = Variant.EMPTY:
	set(value):
		variant = value
		_refresh()

## Disables both the add and delete affordances without changing the visuals'
## footprint — used while a submit is in flight.
@export var locked: bool = false:
	set(value):
		locked = value
		if locked:
			_pressed = false
		_refresh()

var _image: Image = null
var _pressed: bool = false
var _border_normal: StyleBox = null
var _border_pressed: StyleBox = null
var _empty_normal: StyleBox = null
var _empty_pressed: StyleBox = null

@onready var texture_rect_image: TextureRect = %TextureRect_Image
@onready var texture_rect_plus: TextureRect = %TextureRect_Plus
@onready var button_delete: Button = %Button_Delete
@onready var button_add: Button = %Button_Add
@onready var panel_filled: PanelContainer = %Panel_Filled
@onready var panel_empty: PanelContainer = %Panel_Empty
@onready var panel_border: PanelContainer = %Panel_Border

# Pressed uses a 2px Legendary Light outline on both variants (Figma 26:428,
# "Property 1=Pressed"). Godot can't drive a sibling panel's stylebox from a
# Button's own state, so the tile's button reports press/release and the border
# panel is restyled here.


func _ready() -> void:
	# Captured before any override so the Default styleboxes authored in the
	# scene stay the source of truth.
	_border_normal = panel_border.get_theme_stylebox("panel")
	_empty_normal = panel_empty.get_theme_stylebox("panel")
	_border_pressed = _make_pressed(_border_normal, Color(0, 0, 0, 0))
	_empty_pressed = _make_pressed(_empty_normal, Color(0, 0, 0, 0.4))
	button_add.button_down.connect(_on_press_changed.bind(true))
	button_add.button_up.connect(_on_press_changed.bind(false))
	_refresh()


# Mirrors the Default box but with the design's 2px #DF9CFF outline.
func _make_pressed(source: StyleBox, bg: Color) -> StyleBoxFlat:
	var box: StyleBoxFlat = source.duplicate()
	box.bg_color = bg
	box.set_border_width_all(2)
	box.border_color = Color(0.8745098, 0.6117647, 1, 1)
	return box


func _on_press_changed(pressed: bool) -> void:
	if locked:
		return
	_pressed = pressed
	_refresh()


## Shows `image` and switches to the filled state. A null image clears instead.
func set_image(image: Image) -> void:
	if image == null or image.is_empty():
		clear()
		return
	_image = image
	if is_node_ready():
		texture_rect_image.texture = ImageTexture.create_from_image(image)
	variant = Variant.FILLED


func clear() -> void:
	_image = null
	if is_node_ready():
		texture_rect_image.texture = null
	variant = Variant.EMPTY


func get_image() -> Image:
	return _image


func _refresh() -> void:
	if not is_node_ready():
		return
	var is_filled: bool = variant == Variant.FILLED
	panel_filled.visible = is_filled
	panel_border.visible = is_filled
	if _border_pressed != null:
		panel_border.add_theme_stylebox_override(
			"panel", _border_pressed if _pressed else _border_normal
		)
		panel_empty.add_theme_stylebox_override(
			"panel", _empty_pressed if _pressed else _empty_normal
		)
	panel_empty.visible = not is_filled
	button_delete.visible = is_filled
	button_delete.disabled = locked
	button_add.disabled = locked
	texture_rect_plus.visible = not is_filled
	if is_filled and _image != null and texture_rect_image.texture == null:
		texture_rect_image.texture = ImageTexture.create_from_image(_image)


func _on_button_delete_pressed() -> void:
	delete_pressed.emit()


func _on_button_add_pressed() -> void:
	if variant == Variant.FILLED:
		replace_pressed.emit()
	else:
		add_pressed.emit()
