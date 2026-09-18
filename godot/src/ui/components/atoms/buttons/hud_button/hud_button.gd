@tool
class_name HudButton
extends Button

## Reusable HUD action button with three baked background textures swapped by state:
## normal / pressed (finger held) / selected (toggle latched on). Distinct from the joypad's
## OrbSkin buttons — for standalone HUD actions (chat, emote wheel, chat-flip, discover).
## The icon swaps too when an icon_pressed is provided (else icon_normal stays). `selected`
## only ever shows while toggle_mode is on.
##
## SIZE: pick a preset. The px in SIZES are the VISIBLE DISC (and the glyph) — NOT the texture,
## which is larger (glow + free margin). The background overflows the button footprint by
## DISC_TO_CANVAS so the disc renders at the requested size while the footprint stays = disc.

enum Size { SMALL, MEDIUM, LARGE }

# Per preset: x = disc (visible orb) diameter = button footprint, y = icon glyph size, in px.
# Disc sizes are the design spec; icon sizes match the source SVGs (chat/discover are 46).
const SIZES: Dictionary = {
	Size.SMALL: Vector2i(66, 46),
	Size.MEDIUM: Vector2i(80, 48),
	Size.LARGE: Vector2i(96, 67),
}

# The normalized backgrounds are a 512 px disc centered on a 600 px canvas (the glow lives in
# the margin). To render the disc at D px the texture must draw at D * 600/512, so the
# Background is sized that large and centered — overflowing the D-sized footprint by the glow.
const DISC_TO_CANVAS: float = 600.0 / 512.0

# Button theme slots blanked so only the Background texture shows (mirrors OrbSkin). Applied at
# runtime and stripped right before the editor saves, so instances never bake StyleBoxEmpty bloat.
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

@export var normal_texture: Texture2D:
	set(value):
		normal_texture = value
		if is_node_ready():
			_refresh()
@export var pressed_texture: Texture2D:
	set(value):
		pressed_texture = value
		if is_node_ready():
			_refresh()
@export var selected_texture: Texture2D:
	set(value):
		selected_texture = value
		if is_node_ready():
			_refresh()
## Icon shown at rest. Swapped for icon_pressed while pressed / selected (if that one is set).
@export var icon_normal: Texture2D:
	set(value):
		icon_normal = value
		if is_node_ready():
			_refresh()
## Optional: icon shown while the finger is down or the toggle is latched. Null = keep normal.
@export var icon_pressed: Texture2D:
	set(value):
		icon_pressed = value
		if is_node_ready():
			_refresh()
@export var size_preset: Size = Size.SMALL:
	set(value):
		size_preset = value
		if is_node_ready():
			_apply_size()

var _pressing: bool = false

@onready var _bg: TextureRect = $Background


func _ready() -> void:
	_apply_style_overrides()
	expand_icon = true
	icon_alignment = HORIZONTAL_ALIGNMENT_CENTER
	if not Engine.is_editor_hint():
		button_down.connect(_on_button_down)
		button_up.connect(_on_button_up)
		toggled.connect(_on_toggled)
	_apply_size()
	_refresh()


# The blanking overrides are applied live (also in-editor via @tool) but must NOT be serialized
# into the host scene. Mirror OrbSkin: strip them right before the editor writes the file, then
# re-apply so the open scene keeps rendering.
func _notification(what: int) -> void:
	if not Engine.is_editor_hint():
		return
	if what == NOTIFICATION_EDITOR_PRE_SAVE:
		_clear_style_overrides()
	elif what == NOTIFICATION_EDITOR_POST_SAVE:
		_apply_style_overrides()


func _on_button_down() -> void:
	_pressing = true
	_refresh()


func _on_button_up() -> void:
	# button_up fires before toggle_mode flips button_pressed, so settle on the next idle.
	_pressing = false
	_refresh.call_deferred()


func _on_toggled(_toggled_on: bool) -> void:
	_refresh()


func _apply_style_overrides() -> void:
	for slot: StringName in _STYLE_SLOTS:
		add_theme_stylebox_override(slot, StyleBoxEmpty.new())


func _clear_style_overrides() -> void:
	for slot: StringName in _STYLE_SLOTS:
		remove_theme_stylebox_override(slot)


## Footprint = disc; icon capped to the glyph size; Background sized to the full texture
## (disc * DISC_TO_CANVAS) and centered so the disc lands at exactly `disc` px, glow overflowing.
func _apply_size() -> void:
	var spec: Vector2i = SIZES.get(size_preset, SIZES[Size.SMALL])
	var disc: float = float(spec.x)
	var glyph: float = float(spec.y)
	custom_minimum_size = Vector2(disc, disc)
	add_theme_constant_override("icon_max_width", int(glyph))
	if _bg != null:
		var half: float = disc * DISC_TO_CANVAS * 0.5
		_bg.offset_left = -half
		_bg.offset_top = -half
		_bg.offset_right = half
		_bg.offset_bottom = half


## Swap background + icon by state: pressed while held, else selected while toggled on, else
## normal. The icon uses icon_pressed while active (held or latched) when one is provided.
func _refresh(_unused: Variant = null) -> void:
	if _bg != null:
		var tex: Texture2D = normal_texture
		if _pressing and pressed_texture != null:
			tex = pressed_texture
		elif button_pressed and selected_texture != null:
			tex = selected_texture
		_bg.texture = tex
	var glyph: Texture2D = icon_normal
	if (_pressing or button_pressed) and icon_pressed != null:
		glyph = icon_pressed
	icon = glyph
