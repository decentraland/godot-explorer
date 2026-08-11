class_name SceneStatsPanel
extends PanelContainer

## Preview-only overlay: one progress bar per resource metric for the scene
## being previewed, colored against the limits in SceneLimits (issue #2493,
## visual spec in the "Scene Performance Plugin" Figma file):
##   green  = under the soft (recommended) limit
##   orange = between soft and hard
##   red    = over the hard limit — track dims and a warning icon appears
## A circular "monitor" button (same size as the camera button, placed to its
## left) shows/hides this panel: white icon when closed, ruby icon + ring when
## open. The Performance row shows a "?" badge; tapping anywhere on that row
## toggles a tooltip explaining the metric. Instantiated ONLY in preview by
## explorer.gd; never created in production.

enum Status { GREEN, YELLOW, RED }

const REFRESH_INTERVAL: float = 1.5
const TOGGLE_SIZE: float = 65.0
const TOGGLE_GAP: float = 12.0
const MONITOR_ICON_PATH: String = "res://assets/ui/scene_stats_monitor.svg"
const WARNING_ICON_PATH: String = "res://assets/ui/scene_stats_warning.svg"
const TOOLTIP_ARROW_PATH: String = "res://assets/ui/scene_stats_tooltip_arrow.svg"

const ROW_HEIGHT: float = 38.0
const BAR_HEIGHT: float = 30.0
const BAR_RADIUS: int = 8
const NAME_COL_WIDTH: float = 171.0
const HEADER_HEIGHT: float = 26.0
const SCROLLBAR_WIDTH: float = 4.0
const WARNING_ICON_SIZE: float = 22.0

const COLOR_GREEN: Color = Color("00b84f")
const COLOR_YELLOW: Color = Color("ff9100")
const COLOR_RED: Color = Color("ff2d55")
const COLOR_TEXT: Color = Color("fcfcfc")
const COLOR_SHADOW_BG: Color = Color("161518")
const COLOR_SCROLLBAR: Color = Color("966ac5")
const COLOR_MONITOR: Color = Color("ff2d55")

const TOOLTIP_TEXT: String = "Percentage of the FPS target the client runs at for this scene"
const TOOLTIP_MIN_SIZE: Vector2 = Vector2(260.0, 52.0)
const TOOLTIP_ARROW_SIZE: Vector2 = Vector2(10.0, 22.0)
const TOOLTIP_HIDE_DELAY: float = 5.0

const FONT_MEDIUM: Font = preload("res://assets/themes/fonts/inter/Inter-Medium.ttf")
const FONT_BOLD: Font = preload("res://assets/themes/fonts/inter/Inter-Bold.ttf")

var _scene_id: int = -1
var _parcels: int = 1
var _collector: SceneStatsCollector = SceneStatsCollector.new()
var _rows: Dictionary = {}
var _fill_styles: Dictionary = {}
var _track_style: StyleBoxFlat = null
var _track_style_red: StyleBoxFlat = null
var _toggle_btn: Button = null
var _toggle_icon: TextureRect = null
var _fps_row: HBoxContainer = null
var _tooltip: Control = null
var _tooltip_bubble: PanelContainer = null
var _tooltip_arrow: TextureRect = null
var _tooltip_timer: Timer = null
var _tooltip_opened_frame: int = -1
var _tooltip_hidden_frame: int = -1
var _badge: PanelContainer = null
var _badge_label: Label = null

@onready var _scroll: ScrollContainer = %Scroll
@onready var _rows_box: VBoxContainer = %RowsContainer
@onready var _timer: Timer = %Timer


func _ready() -> void:
	_build_styleboxes()
	_build_rows()
	_style_scrollbar()
	_create_toggle_button()
	_timer.wait_time = REFRESH_INTERVAL
	_timer.timeout.connect(_on_timer_timeout)
	_timer.start()
	_refresh()


## While the tooltip is open, ANY click/tap dismisses it (click-away modal
## behavior). Runs before GUI input, so consuming the press also keeps the
## Performance row's gui_input from re-toggling the tooltip open — the row
## handler then only ever fires while the tooltip is closed, i.e. it only opens.
func _input(event: InputEvent) -> void:
	if _tooltip == null or not _tooltip.visible:
		return
	# One physical tap arrives twice (emulate_touch_from_mouse duplicates every
	# press as mouse + touch). The duplicate of the press that OPENED the
	# tooltip lands on this same frame — skip it, or the tooltip would close
	# right after opening and appear to never show.
	if Engine.get_process_frames() == _tooltip_opened_frame:
		return
	var mouse_press: bool = event is InputEventMouseButton and event.pressed
	var touch_press: bool = event is InputEventScreenTouch and event.pressed
	if mouse_press or touch_press:
		_hide_tooltip()
		get_viewport().set_input_as_handled()


## Free the external toggle button (it lives under the camera button, not under
## this panel) when the overlay is torn down.
func _exit_tree() -> void:
	if is_instance_valid(_toggle_btn):
		_toggle_btn.queue_free()
		_toggle_btn = null


## Point the overlay at a scene (called by explorer.gd on preview / scene change).
func set_scene(scene_id: int) -> void:
	if scene_id == _scene_id:
		return
	_scene_id = scene_id
	_parcels = _parcel_count(scene_id)
	_collector.reset()
	_refresh()


func _parcel_count(scene_id: int) -> int:
	if scene_id == -1 or not is_instance_valid(Global.scene_fetcher):
		return 1
	var data = Global.scene_fetcher.get_scene_data_by_scene_id(scene_id)
	if data != null:
		return maxi(data.parcels.size(), 1)
	return 1


func _on_timer_timeout() -> void:
	if not is_visible_in_tree():
		return
	_refresh()


func _on_toggle_pressed() -> void:
	visible = not visible
	_apply_toggle_state()
	if visible:
		_refresh()
	else:
		_hide_tooltip()


func _refresh() -> void:
	if _scene_id == -1:
		for key in _rows:
			_set_row_idle(key)
		return

	var stats: Dictionary = _collector.collect_scene(_scene_id)
	stats["content_size"] = _collector.content_bytes(_scene_id)
	stats["external_size"] = _collector.external_bytes(_scene_id)
	var glob: Dictionary = SceneStatsCollector.global_stats()

	for meta in SceneLimits.metric_order():
		var key: String = meta["key"]
		var value: int = int(stats.get(key, glob.get(key, 0)))
		var lim: Dictionary = SceneLimits.limits_for(key, _parcels)
		var soft: int = int(lim["soft"])
		var hard: int = int(lim["hard"])
		# FPS: the bar's full scale is the CURRENT max fps (thermal/user cap, or
		# the display refresh rate when uncapped). Green at >=90% of the target,
		# orange below it, red at <=85% (issue #2493 thresholds).
		if bool(meta.get("dynamic_max", false)):
			var m: int = _current_max_fps()
			meta["bar_max"] = m
			soft = int(ceil(float(m) * 0.9))
			hard = int(round(float(m) * 0.85))
		_update_row(key, value, soft, hard, meta)


func _update_row(key: String, value: int, soft: int, hard: int, meta: Dictionary) -> void:
	var row = _rows.get(key)
	if row == null:
		return
	var bar: ProgressBar = row["bar"]
	var lbl: Label = row["value"]
	var icon: TextureRect = row["icon"]
	var inverse: bool = bool(meta.get("inverse", false))
	var unit: String = str(meta.get("unit", "count"))
	var status: int = _status(value, soft, hard, inverse)

	if inverse:
		# Show performance as a percentage of the current max (e.g. fps / max fps):
		# 100% = running at the cap, lower = worse. Not the raw fps number.
		var bar_max: int = int(meta.get("bar_max", soft))
		bar.max_value = maxi(bar_max, 1)
		bar.value = clampi(value, 0, int(bar.max_value))
		var pct: int = 0
		if bar_max > 0:
			pct = clampi(int(round(float(value) / float(bar_max) * 100.0)), 0, 100)
		lbl.text = "%d%%" % pct
		icon.visible = false
	else:
		bar.max_value = maxi(hard, 1)
		bar.value = clampi(value, 0, int(bar.max_value))
		var pct: int = 0
		if hard > 0:
			pct = int(round(float(value) / float(hard) * 100.0))
		lbl.text = ("%s / %s (%d%%)" % [_fmt(value, unit), _fmt(hard, unit), pct]).to_upper()
		icon.visible = status == Status.RED

	# Over-limit rows dim the track (40% vs 70% black) per the Figma bar states.
	var track: StyleBoxFlat = _track_style_red if status == Status.RED else _track_style
	bar.add_theme_stylebox_override("background", track)
	bar.add_theme_stylebox_override("fill", _fill_styles[status])


func _set_row_idle(key: String) -> void:
	var row = _rows.get(key)
	if row == null:
		return
	var bar: ProgressBar = row["bar"]
	bar.value = 0
	bar.add_theme_stylebox_override("background", _track_style)
	bar.add_theme_stylebox_override("fill", _fill_styles[Status.GREEN])
	(row["value"] as Label).text = "—"
	(row["icon"] as TextureRect).visible = false


## Current max fps: the active cap (thermal or user setting, via Engine.max_fps),
## or the display refresh rate when uncapped, or 60 as a last resort.
func _current_max_fps() -> int:
	var cap: int = Engine.max_fps
	if cap > 0:
		return cap
	var hz: float = DisplayServer.screen_get_refresh_rate()
	if hz > 0.0:
		return int(round(hz))
	return 60


static func _status(value: int, soft: int, hard: int, inverse: bool) -> int:
	if inverse:
		if value >= soft:
			return Status.GREEN
		if value > hard:
			return Status.YELLOW
		return Status.RED
	if value <= soft:
		return Status.GREEN
	if value <= hard:
		return Status.YELLOW
	return Status.RED


static func _fmt(value: int, unit: String) -> String:
	if unit == "bytes":
		return _fmt_bytes(value)
	return _fmt_int(value)


static func _fmt_bytes(n: int) -> String:
	if n >= 1024 * 1024 * 1024:
		return _trim_zero("%.1f" % (float(n) / 1073741824.0)) + " GB"
	if n >= 1024 * 1024:
		return _trim_zero("%.1f" % (float(n) / 1048576.0)) + " MB"
	if n >= 1024:
		return _trim_zero("%.1f" % (float(n) / 1024.0)) + " KB"
	return "%d B" % n


## Compact notation with at most 1 decimal: 1234567 -> "1.2M", 6000 -> "6K".
static func _fmt_int(n: int) -> String:
	var a: int = absi(n)
	var out: String
	if a >= 1000000:
		out = _trim_zero("%.1f" % (float(a) / 1000000.0)) + "M"
	elif a >= 1000:
		out = _trim_zero("%.1f" % (float(a) / 1000.0)) + "K"
	else:
		out = str(a)
	return ("-" + out) if n < 0 else out


static func _trim_zero(s: String) -> String:
	return s.trim_suffix(".0")


func _build_styleboxes() -> void:
	_fill_styles[Status.GREEN] = _make_fill(COLOR_GREEN)
	_fill_styles[Status.YELLOW] = _make_fill(COLOR_YELLOW)
	_fill_styles[Status.RED] = _make_fill(COLOR_RED)
	_track_style = StyleBoxFlat.new()
	_track_style.bg_color = Color(0, 0, 0, 0.7)
	_track_style.set_corner_radius_all(BAR_RADIUS)
	_track_style_red = StyleBoxFlat.new()
	_track_style_red.bg_color = Color(0, 0, 0, 0.4)
	_track_style_red.set_corner_radius_all(BAR_RADIUS)


func _make_fill(c: Color) -> StyleBoxFlat:
	var sb: StyleBoxFlat = StyleBoxFlat.new()
	sb.bg_color = c
	sb.set_corner_radius_all(BAR_RADIUS)
	return sb


func _make_circle(c: Color, ring: bool) -> StyleBoxFlat:
	var sb: StyleBoxFlat = StyleBoxFlat.new()
	sb.bg_color = c
	sb.set_corner_radius_all(int(TOGGLE_SIZE / 2.0))
	sb.content_margin_left = 12.0
	sb.content_margin_right = 12.0
	sb.content_margin_top = 12.0
	sb.content_margin_bottom = 12.0
	if ring:
		sb.set_border_width_all(2)
		sb.border_color = COLOR_MONITOR
	return sb


## 4px scrollbar: dark track, purple grabber (Figma "Group 3107").
func _style_scrollbar() -> void:
	var vsb: VScrollBar = _scroll.get_v_scroll_bar()
	vsb.custom_minimum_size = Vector2(SCROLLBAR_WIDTH, 0)
	var track: StyleBoxFlat = StyleBoxFlat.new()
	track.bg_color = COLOR_SHADOW_BG
	vsb.add_theme_stylebox_override("scroll", track)
	var grabber: StyleBoxFlat = StyleBoxFlat.new()
	grabber.bg_color = COLOR_SCROLLBAR
	for style_name in ["grabber", "grabber_highlight", "grabber_pressed"]:
		vsb.add_theme_stylebox_override(style_name, grabber)


## Build the monitor toggle button and place it to the LEFT of the camera
## (1st/3rd person) button at the same size; fall back to the panel's parent.
func _create_toggle_button() -> void:
	_toggle_btn = Button.new()
	_toggle_btn.custom_minimum_size = Vector2(TOGGLE_SIZE, TOGGLE_SIZE)
	_toggle_btn.focus_mode = Control.FOCUS_NONE
	_toggle_btn.pressed.connect(_on_toggle_pressed)

	# Icon as a centered child TextureRect — guarantees centering and sizing
	# regardless of Button icon-layout quirks.
	_toggle_icon = TextureRect.new()
	_toggle_icon.texture = load(MONITOR_ICON_PATH)
	_toggle_icon.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	_toggle_icon.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	_toggle_icon.set_anchors_preset(Control.PRESET_FULL_RECT)
	_toggle_icon.offset_left = 15.0
	_toggle_icon.offset_top = 15.0
	_toggle_icon.offset_right = -15.0
	_toggle_icon.offset_bottom = -15.0
	_toggle_icon.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_toggle_btn.add_child(_toggle_icon)
	_apply_toggle_state()

	var cam: Control = _find_camera_button()
	if cam != null:
		cam.get_parent().add_child(_toggle_btn)
		_toggle_btn.anchor_left = cam.anchor_left
		_toggle_btn.anchor_top = cam.anchor_top
		_toggle_btn.anchor_right = cam.anchor_right
		_toggle_btn.anchor_bottom = cam.anchor_bottom
		_toggle_btn.grow_horizontal = cam.grow_horizontal
		_toggle_btn.offset_top = cam.offset_top
		_toggle_btn.offset_bottom = cam.offset_bottom
		_toggle_btn.offset_right = cam.offset_left - TOGGLE_GAP
		_toggle_btn.offset_left = cam.offset_left - TOGGLE_GAP - TOGGLE_SIZE
	else:
		get_parent().add_child(_toggle_btn)
		_toggle_btn.set_anchors_preset(Control.PRESET_TOP_RIGHT)
		_toggle_btn.grow_horizontal = Control.GROW_DIRECTION_BEGIN
		_toggle_btn.offset_right = -12.0
		_toggle_btn.offset_left = -12.0 - TOGGLE_SIZE
		_toggle_btn.offset_top = 40.0
		_toggle_btn.offset_bottom = 40.0 + TOGGLE_SIZE


## Closed: white icon on a plain dark circle. Open: ruby icon + ruby ring
## (Figma "Performance-Close" / "Performance-Open" frames).
func _apply_toggle_state() -> void:
	if _toggle_btn == null:
		return
	var open: bool = visible
	_toggle_btn.add_theme_stylebox_override("normal", _make_circle(Color(0, 0, 0, 0.62), open))
	var active: StyleBoxFlat = _make_circle(Color(0.22, 0.22, 0.28, 0.85), open)
	_toggle_btn.add_theme_stylebox_override("hover", active)
	_toggle_btn.add_theme_stylebox_override("pressed", active)
	_toggle_icon.modulate = COLOR_MONITOR if open else Color.WHITE


func _find_camera_button() -> Control:
	if not is_inside_tree():
		return null
	var node := get_tree().get_root().find_child("Button_Camera", true, false)
	return node as Control


func _build_rows() -> void:
	_rows_box.add_theme_constant_override("separation", 0)
	var last_group: String = ""
	var first_header: bool = true
	var options_box: VBoxContainer = null
	for meta in SceneLimits.metric_order():
		var key: String = meta["key"]
		var group: String = str(meta.get("group", "scene"))
		if group != last_group:
			last_group = group
			_rows_box.add_child(_make_header(group, first_header))
			first_header = false
			options_box = _make_options_box()
			_rows_box.add_child(options_box.get_parent())

		var row: HBoxContainer = HBoxContainer.new()
		row.custom_minimum_size = Vector2(0, ROW_HEIGHT)
		row.mouse_filter = Control.MOUSE_FILTER_IGNORE
		row.add_theme_constant_override("separation", 0)

		var name_lbl: Label = Label.new()
		name_lbl.text = str(meta["label"])
		name_lbl.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
		name_lbl.mouse_filter = Control.MOUSE_FILTER_IGNORE
		name_lbl.add_theme_font_override("font", FONT_MEDIUM)
		name_lbl.add_theme_font_size_override("font_size", 18)
		name_lbl.add_theme_color_override("font_color", COLOR_TEXT)

		if key == "fps":
			# Name column holds the label + the "?" tooltip badge; the whole row
			# is tappable to toggle the tooltip (issue #2493).
			var name_box: HBoxContainer = HBoxContainer.new()
			name_box.custom_minimum_size = Vector2(NAME_COL_WIDTH, 0)
			name_box.mouse_filter = Control.MOUSE_FILTER_IGNORE
			name_box.add_theme_constant_override("separation", 8)
			name_box.add_child(name_lbl)
			name_box.add_child(_make_tooltip_badge())
			row.add_child(name_box)
		else:
			name_lbl.custom_minimum_size = Vector2(NAME_COL_WIDTH, 0)
			row.add_child(name_lbl)

		var bar: ProgressBar = ProgressBar.new()
		bar.show_percentage = false
		bar.custom_minimum_size = Vector2(0, BAR_HEIGHT)
		bar.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		bar.size_flags_vertical = Control.SIZE_SHRINK_CENTER
		bar.mouse_filter = Control.MOUSE_FILTER_IGNORE
		bar.add_theme_stylebox_override("background", _track_style)
		bar.add_theme_stylebox_override("fill", _fill_styles[Status.GREEN])

		# Value text drawn INSIDE the bar — white bold with a subtle dark glow so
		# it stays readable over any fill color (green/orange/red) and the track.
		var val_lbl: Label = Label.new()
		val_lbl.set_anchors_preset(Control.PRESET_FULL_RECT)
		val_lbl.offset_left = 8
		val_lbl.offset_right = -8
		val_lbl.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
		val_lbl.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
		val_lbl.mouse_filter = Control.MOUSE_FILTER_IGNORE
		val_lbl.add_theme_font_override("font", FONT_BOLD)
		val_lbl.add_theme_font_size_override("font_size", 16)
		val_lbl.add_theme_color_override("font_color", Color.WHITE)
		val_lbl.add_theme_color_override("font_outline_color", Color(0, 0, 0, 0.8))
		val_lbl.add_theme_constant_override("outline_size", 3)
		bar.add_child(val_lbl)

		# Warning icon at the left inside the bar, shown only when over the limit.
		var warn: TextureRect = TextureRect.new()
		warn.texture = load(WARNING_ICON_PATH)
		warn.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
		warn.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
		warn.set_anchors_preset(Control.PRESET_CENTER_LEFT)
		warn.offset_left = 7
		warn.offset_right = 7 + WARNING_ICON_SIZE
		warn.offset_top = -WARNING_ICON_SIZE / 2.0
		warn.offset_bottom = WARNING_ICON_SIZE / 2.0
		warn.mouse_filter = Control.MOUSE_FILTER_IGNORE
		warn.visible = false
		bar.add_child(warn)

		row.add_child(bar)
		options_box.add_child(row)
		_rows[key] = {"bar": bar, "value": val_lbl, "icon": warn}

		if key == "fps":
			_fps_row = row
			row.mouse_filter = Control.MOUSE_FILTER_STOP
			row.gui_input.connect(_on_fps_row_input)


## Rows of a group sit in a VBox inset 16px from the panel edges (Figma
## "options" frames). Returns the VBox; its MarginContainer parent is what
## gets added to the panel.
func _make_options_box() -> VBoxContainer:
	var margin: MarginContainer = MarginContainer.new()
	margin.mouse_filter = Control.MOUSE_FILTER_IGNORE
	margin.add_theme_constant_override("margin_left", 16)
	margin.add_theme_constant_override("margin_right", 16)
	margin.add_theme_constant_override("margin_top", 2)
	margin.add_theme_constant_override("margin_bottom", 8)
	var box: VBoxContainer = VBoxContainer.new()
	box.mouse_filter = Control.MOUSE_FILTER_IGNORE
	box.add_theme_constant_override("separation", 2)
	margin.add_child(box)
	return box


func _make_header(group: String, first: bool) -> PanelContainer:
	var pc: PanelContainer = PanelContainer.new()
	pc.custom_minimum_size = Vector2(0, HEADER_HEIGHT)
	pc.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var sb: StyleBoxFlat = StyleBoxFlat.new()
	sb.bg_color = COLOR_SHADOW_BG
	sb.content_margin_left = 10.0
	sb.content_margin_right = 10.0
	sb.content_margin_top = 4.0
	sb.content_margin_bottom = 4.0
	if first:
		# The first header hugs the panel's rounded top corners.
		sb.corner_radius_top_left = BAR_RADIUS
		sb.corner_radius_top_right = BAR_RADIUS
	pc.add_theme_stylebox_override("panel", sb)
	var hdr: Label = Label.new()
	hdr.text = "Per Scene" if group == "scene" else "Whole App"
	hdr.mouse_filter = Control.MOUSE_FILTER_IGNORE
	hdr.add_theme_font_override("font", FONT_BOLD)
	hdr.add_theme_font_size_override("font_size", 16)
	hdr.add_theme_color_override("font_color", COLOR_TEXT)
	pc.add_child(hdr)
	return pc


func _make_tooltip_badge() -> PanelContainer:
	_badge = PanelContainer.new()
	_badge.custom_minimum_size = Vector2(20, 22)
	_badge.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	_badge.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_badge_label = Label.new()
	_badge_label.text = "?"
	_badge_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_badge_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	_badge_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_badge_label.add_theme_font_override("font", FONT_MEDIUM)
	_badge_label.add_theme_font_size_override("font_size", 16)
	_update_badge_style()
	_badge.add_child(_badge_label)
	return _badge


## "?" badge: dark with light text normally, inverted while the tooltip is open.
func _update_badge_style() -> void:
	if _badge == null:
		return
	var active: bool = _tooltip != null and _tooltip.visible
	var sb: StyleBoxFlat = StyleBoxFlat.new()
	sb.bg_color = COLOR_TEXT if active else COLOR_SHADOW_BG
	sb.set_corner_radius_all(4)
	_badge.add_theme_stylebox_override("panel", sb)
	var text_color: Color = COLOR_SHADOW_BG if active else COLOR_TEXT
	_badge_label.add_theme_color_override("font_color", text_color)


func _on_fps_row_input(event: InputEvent) -> void:
	# Mirror of the guard in _input(): if the dismissing press hid the tooltip
	# on this same frame, its emulated duplicate must not re-open it.
	if Engine.get_process_frames() == _tooltip_hidden_frame:
		return
	var mb: InputEventMouseButton = event as InputEventMouseButton
	if mb != null and mb.pressed and mb.button_index == MOUSE_BUTTON_LEFT:
		_toggle_tooltip()


func _toggle_tooltip() -> void:
	if _tooltip == null:
		_build_tooltip()
	if _tooltip.visible:
		_hide_tooltip()
		return
	_tooltip.visible = true
	_tooltip_opened_frame = Engine.get_process_frames()
	_update_badge_style()
	# Just left of the panel, vertically centered on the Performance row, with
	# the arrow pointing at the row. Sized from the bubble's real minimum so a
	# longer text (more wrapped lines) keeps the arrow on the bubble's middle.
	var bubble_size: Vector2 = _tooltip_bubble.get_combined_minimum_size()
	_tooltip_bubble.size = bubble_size
	var arrow_y: float = (bubble_size.y - TOOLTIP_ARROW_SIZE.y) / 2.0
	_tooltip_arrow.position = Vector2(bubble_size.x, arrow_y)
	var row_rect: Rect2 = _fps_row.get_global_rect()
	var x: float = get_global_rect().position.x - bubble_size.x - TOOLTIP_ARROW_SIZE.x
	var y: float = row_rect.position.y + (row_rect.size.y - bubble_size.y) / 2.0
	_tooltip.global_position = Vector2(x, y)
	_tooltip_timer.start()


func _hide_tooltip() -> void:
	if _tooltip == null:
		return
	_tooltip.visible = false
	_tooltip_hidden_frame = Engine.get_process_frames()
	_tooltip_timer.stop()
	_update_badge_style()


## Speech bubble: StyleBoxFlat body + the design's exported arrow SVG attached
## to its right edge (Figma "Tooltip" component).
func _build_tooltip() -> void:
	_tooltip = Control.new()
	_tooltip.top_level = true
	_tooltip.visible = false
	_tooltip.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(_tooltip)

	var bubble: PanelContainer = PanelContainer.new()
	bubble.custom_minimum_size = TOOLTIP_MIN_SIZE
	bubble.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var sb: StyleBoxFlat = StyleBoxFlat.new()
	sb.bg_color = Color(COLOR_SHADOW_BG, 0.8)
	sb.set_corner_radius_all(11)
	sb.content_margin_left = 9.0
	sb.content_margin_right = 9.0
	sb.content_margin_top = 5.0
	sb.content_margin_bottom = 5.0
	bubble.add_theme_stylebox_override("panel", sb)
	var lbl: Label = Label.new()
	lbl.text = TOOLTIP_TEXT
	lbl.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	# Fixed wrap width — without it an autowrap Label reports its minimum size
	# at longest-word width, which inflates the bubble's minimum height.
	lbl.custom_minimum_size = Vector2(TOOLTIP_MIN_SIZE.x - 18.0, 0)
	lbl.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	lbl.mouse_filter = Control.MOUSE_FILTER_IGNORE
	lbl.add_theme_font_override("font", FONT_MEDIUM)
	lbl.add_theme_font_size_override("font_size", 17)
	lbl.add_theme_color_override("font_color", COLOR_TEXT)
	bubble.add_child(lbl)
	_tooltip.add_child(bubble)
	_tooltip_bubble = bubble

	var arrow: TextureRect = TextureRect.new()
	arrow.texture = load(TOOLTIP_ARROW_PATH)
	arrow.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	arrow.stretch_mode = TextureRect.STRETCH_SCALE
	arrow.size = TOOLTIP_ARROW_SIZE
	arrow.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_tooltip.add_child(arrow)
	_tooltip_arrow = arrow

	_tooltip_timer = Timer.new()
	_tooltip_timer.one_shot = true
	_tooltip_timer.wait_time = TOOLTIP_HIDE_DELAY
	_tooltip_timer.timeout.connect(_hide_tooltip)
	_tooltip.add_child(_tooltip_timer)
