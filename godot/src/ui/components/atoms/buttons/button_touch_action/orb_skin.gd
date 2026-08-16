@tool
class_name OrbSkin
extends TextureRect

# Texture-based "orb" skin for a circular touch button. Drop this as a child of a Button
# (e.g. the TouchableButton variation used across the joypad): on ready it blanks the theme
# styleboxes so the flat circle stops drawing, then renders a pre-baked orb texture *behind*
# the button's glyph (show_behind_parent) and swaps texture + icon by state.
#
# States: Default (normal) / Pressed (button held) / Hold (external latch, e.g. the glider
# active state, via set_hold()) / Disabled. The textures already bake the fill, outline and
# glow at a consistent scale, so no per-state geometry is needed here.
#
# @tool so the Default look renders live in the editor when the scene is open.

# Theme style slots the TouchableButton variation fills; blanked so only the orb shows.
const _STYLE_SLOTS: Array[StringName] = [
	&"normal",
	&"normal_mirrored",
	&"hover",
	&"hover_mirrored",
	&"pressed",
	&"pressed_mirrored",
	&"hover_pressed",
	&"hover_pressed_mirrored",
	&"disabled",
	&"disabled_mirrored",
	&"focus",
]

# Glyph stays light in every state (the theme flips it dark for the old white pressed bg).
const _ICON_LIGHT := Color(0.9098039, 0.8509804, 1.0, 1.0)
const _ICON_COLOR_SLOTS: Array[StringName] = [
	&"icon_pressed_color",
	&"icon_hover_pressed_color",
	&"font_pressed_color",
	&"font_hover_pressed_color",
]

@export var default_texture: Texture2D:
	set(value):
		default_texture = value
		if is_node_ready():
			_refresh()
@export var pressed_texture: Texture2D
@export var hold_texture: Texture2D
@export var disabled_texture: Texture2D

# Per-state glyphs. When set, the parent button's icon tracks the state alongside the
# texture. hold_icon is optional (only the glider has one).
@export var normal_icon: Texture2D
@export var pressed_icon: Texture2D
@export var hold_icon: Texture2D

var _held: bool = false


func _ready() -> void:
	show_behind_parent = true
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED

	var btn := get_parent() as Button
	if btn != null:
		for slot in _STYLE_SLOTS:
			btn.add_theme_stylebox_override(slot, StyleBoxEmpty.new())
		for slot in _ICON_COLOR_SLOTS:
			btn.add_theme_color_override(slot, _ICON_LIGHT)
		# Fill the parent explicitly: anchors don't resolve reliably under a
		# non-container Button, and the orb must track the button's size.
		_fit()
		btn.resized.connect(_fit)
		if not Engine.is_editor_hint():
			btn.button_down.connect(_refresh)
			# button_up fires before the atom lowers button_pressed, so settle next idle.
			btn.button_up.connect(func() -> void: _refresh.call_deferred())
			btn.toggled.connect(func(_on: bool) -> void: _refresh())
	_refresh()


func _fit() -> void:
	var btn := get_parent() as Control
	if btn != null:
		position = Vector2.ZERO
		size = btn.size


## External latch for the Hold look (kept through press/release until cleared).
func set_hold(on: bool) -> void:
	if _held == on:
		return
	_held = on
	_refresh()


## Assign the per-state glyphs; the button shows the one matching its current state.
func set_icons(normal: Texture2D, pressed: Texture2D, hold: Texture2D) -> void:
	normal_icon = normal
	pressed_icon = pressed
	hold_icon = hold
	if is_node_ready():
		_refresh()


func _refresh(_unused_arg: Variant = null) -> void:
	var btn := get_parent() as Button
	var tex: Texture2D = default_texture
	var ic: Texture2D = normal_icon
	if btn != null and btn.disabled:
		if disabled_texture != null:
			tex = disabled_texture
	elif _held:
		if hold_texture != null:
			tex = hold_texture
		if hold_icon != null:
			ic = hold_icon
	elif btn != null and btn.button_pressed:
		if pressed_texture != null:
			tex = pressed_texture
		if pressed_icon != null:
			ic = pressed_icon
	texture = tex
	if btn != null and (normal_icon != null or pressed_icon != null or hold_icon != null):
		btn.icon = ic
