class_name SafeMarginDebugOverlay
extends CanvasLayer

## Transparent debug overlay that tints the unsafe margin strips and shows
## a center HUD with the current resolution / scaled resolution / base
## resolution / safe-margin values.
##
## Toggled show/hide via a 3-finger touch (default visible). Instantiated at
## runtime by Global.set_safe_margin_debug_enable() when the deep link param
## `safemargindebug=true` is set.

const UNSAFE_TINT := Color(1.0, 0.25, 0.1, 0.35)

var _unsafe_top: ColorRect
var _unsafe_bottom: ColorRect
var _unsafe_left: ColorRect
var _unsafe_right: ColorRect
var _hud_panel: PanelContainer
var _hud_label: Label

var _active_touches: Dictionary = {}
var _gesture_consumed: bool = false


func _ready() -> void:
	layer = 1000

	_unsafe_top = _make_strip()
	_unsafe_bottom = _make_strip()
	_unsafe_left = _make_strip()
	_unsafe_right = _make_strip()
	add_child(_unsafe_top)
	add_child(_unsafe_bottom)
	add_child(_unsafe_left)
	add_child(_unsafe_right)

	_build_hud()

	get_window().size_changed.connect(_refresh)
	Global.orientation_changed.connect(_on_orientation_changed)
	_refresh()


func _make_strip() -> ColorRect:
	var rect := ColorRect.new()
	rect.color = UNSAFE_TINT
	rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
	return rect


func _build_hud() -> void:
	_hud_panel = PanelContainer.new()
	_hud_panel.mouse_filter = Control.MOUSE_FILTER_IGNORE

	var bg := StyleBoxFlat.new()
	bg.bg_color = Color(0, 0, 0, 0.6)
	bg.content_margin_left = 12
	bg.content_margin_right = 12
	bg.content_margin_top = 8
	bg.content_margin_bottom = 8
	_hud_panel.add_theme_stylebox_override("panel", bg)

	_hud_label = Label.new()
	_hud_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_hud_label.add_theme_font_size_override("font_size", 14)
	_hud_label.add_theme_color_override("font_color", Color.WHITE)
	_hud_label.add_theme_color_override("font_shadow_color", Color.BLACK)
	_hud_label.add_theme_constant_override("shadow_offset_x", 1)
	_hud_label.add_theme_constant_override("shadow_offset_y", 1)
	_hud_panel.add_child(_hud_label)

	add_child(_hud_panel)


func _on_orientation_changed(_is_portrait: bool) -> void:
	_refresh.call_deferred()


func _refresh() -> void:
	if not is_inside_tree() or get_viewport() == null:
		return

	var window_size: Vector2i = DisplayServer.window_get_size()
	var viewport_size: Vector2 = get_viewport().get_visible_rect().size
	if window_size.x <= 0 or window_size.y <= 0:
		return

	var safe_area: Rect2i = Global.get_safe_area()
	var base_resolution: Vector2 = GraphicSettings.get_base_resolution(Vector2(window_size))
	var content_scale: float = get_window().content_scale_factor

	# Safe-area margins in window pixels (what notch/cutout costs us).
	var top_px: int = max(0, safe_area.position.y)
	var left_px: int = max(0, safe_area.position.x)
	var bottom_px: int = max(0, window_size.y - safe_area.end.y)
	var right_px: int = max(0, window_size.x - safe_area.end.x)

	# Convert to viewport-space coords for Control positioning (matches the
	# factor used by SafeMarginContainer.gd).
	var x_factor: float = viewport_size.x / float(window_size.x)
	var y_factor: float = viewport_size.y / float(window_size.y)
	var top_vp: float = top_px * y_factor
	var left_vp: float = left_px * x_factor
	var bottom_vp: float = bottom_px * y_factor
	var right_vp: float = right_px * x_factor

	# Top/bottom strips span full width; left/right span only the safe vertical
	# band so corners aren't double-tinted.
	_unsafe_top.position = Vector2.ZERO
	_unsafe_top.size = Vector2(viewport_size.x, top_vp)

	_unsafe_bottom.position = Vector2(0, viewport_size.y - bottom_vp)
	_unsafe_bottom.size = Vector2(viewport_size.x, bottom_vp)

	_unsafe_left.position = Vector2(0, top_vp)
	_unsafe_left.size = Vector2(left_vp, viewport_size.y - top_vp - bottom_vp)

	_unsafe_right.position = Vector2(viewport_size.x - right_vp, top_vp)
	_unsafe_right.size = Vector2(right_vp, viewport_size.y - top_vp - bottom_vp)

	# Scaled (logical) resolution is what UI layouts actually see.
	var scaled := Vector2(window_size) / max(content_scale, 0.0001)

	_hud_label.text = (
		"=== SAFE MARGIN DEBUG ===\n"
		+ "Window:  %d x %d px\n" % [window_size.x, window_size.y]
		+ "Scaled:  %d x %d  (scale %.3f)\n" % [int(scaled.x), int(scaled.y), content_scale]
		+ "Base:    %d x %d\n" % [int(base_resolution.x), int(base_resolution.y)]
		+ "Margins: T=%d  L=%d  B=%d  R=%d  (window px)\n" % [top_px, left_px, bottom_px, right_px]
		+ "3-finger touch to toggle"
	)

	# Center the HUD using its measured minimum size.
	_hud_panel.reset_size()
	var hud_size: Vector2 = _hud_panel.size
	_hud_panel.position = (viewport_size - hud_size) * 0.5


func _input(event: InputEvent) -> void:
	if not (event is InputEventScreenTouch):
		return

	var touch := event as InputEventScreenTouch
	if touch.pressed:
		_active_touches[touch.index] = true
		if _active_touches.size() >= 3 and not _gesture_consumed:
			_gesture_consumed = true
			visible = not visible
	else:
		_active_touches.erase(touch.index)
		if _active_touches.size() < 3:
			_gesture_consumed = false
