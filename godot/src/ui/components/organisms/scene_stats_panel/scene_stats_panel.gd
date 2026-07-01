class_name SceneStatsPanel
extends PanelContainer

## Preview-only overlay: one progress bar per resource metric for the scene
## being previewed, colored against the limits in SceneLimits.
##   green  = under the soft (recommended) limit
##   yellow = between soft and hard
##   red    = over the hard limit, annotated with the overpass % (e.g. 200%)
## A separate red "monitor" icon button (same size as the camera button, placed
## to its left) shows/hides this panel. Instantiated ONLY in preview by
## explorer.gd; never created in production.

enum Status { GREEN, YELLOW, RED }

const REFRESH_INTERVAL: float = 1.5
const TOGGLE_SIZE: float = 65.0
const TOGGLE_GAP: float = 12.0
const MONITOR_ICON_PATH: String = "res://assets/ui/scene_stats_monitor.svg"

const COLOR_GREEN: Color = Color(0.30, 0.85, 0.35)
const COLOR_YELLOW: Color = Color(0.95, 0.80, 0.20)
const COLOR_RED: Color = Color(0.95, 0.30, 0.30)
const COLOR_DIM: Color = Color(0.70, 0.70, 0.72)
const COLOR_TEXT: Color = Color(0.90, 0.90, 0.92)
const COLOR_MONITOR: Color = Color(0.95, 0.18, 0.18)

const UI_FONT: Font = preload("res://assets/themes/fonts/inter/inter_600.ttf")

var _scene_id: int = -1
var _parcels: int = 1
var _collector: SceneStatsCollector = SceneStatsCollector.new()
var _rows: Dictionary = {}
var _fill_styles: Dictionary = {}
var _bg_style: StyleBoxFlat = null
var _toggle_btn: Button = null

@onready var _rows_box: VBoxContainer = %RowsContainer
@onready var _timer: Timer = %Timer


func _ready() -> void:
	_build_styleboxes()
	_build_rows()
	_create_toggle_button()
	_timer.wait_time = REFRESH_INTERVAL
	_timer.timeout.connect(_on_timer_timeout)
	_timer.start()
	_refresh()


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
	if visible:
		_refresh()


func _refresh() -> void:
	if _scene_id == -1:
		for key in _rows:
			_set_row_idle(key)
		return

	var stats: Dictionary = _collector.collect_scene(_scene_id)
	stats["content_size"] = _collector.content_bytes(_scene_id)
	var glob: Dictionary = SceneStatsCollector.global_stats()

	for meta in SceneLimits.metric_order():
		var key: String = meta["key"]
		var value: int = int(stats.get(key, glob.get(key, 0)))
		var lim: Dictionary = SceneLimits.limits_for(key, _parcels)
		var soft: int = int(lim["soft"])
		var hard: int = int(lim["hard"])
		# FPS: the bar's full scale is the CURRENT max fps (thermal/user cap, or
		# the display refresh rate when uncapped). Green within 80% of the cap,
		# red below half of it — so an 18fps-capped scene treats 18 as the max.
		if bool(meta.get("dynamic_max", false)):
			var m: int = _current_max_fps()
			meta["bar_max"] = m
			soft = int(round(float(m) * 0.8))
			hard = int(round(float(m) * 0.5))
		_update_row(key, value, soft, hard, meta)


func _update_row(key: String, value: int, soft: int, hard: int, meta: Dictionary) -> void:
	var row = _rows.get(key)
	if row == null:
		return
	var bar: ProgressBar = row["bar"]
	var lbl: Label = row["value"]
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
	else:
		bar.max_value = maxi(hard, 1)
		bar.value = clampi(value, 0, int(bar.max_value))
		var pct: int = 0
		if hard > 0:
			pct = int(round(float(value) / float(hard) * 100.0))
		lbl.text = "%s / %s (%d%%)" % [_fmt(value, unit), _fmt(hard, unit), pct]
		if status == Status.RED:
			lbl.text += "  ⚠"

	bar.add_theme_stylebox_override("fill", _fill_styles[status])


func _set_row_idle(key: String) -> void:
	var row = _rows.get(key)
	if row == null:
		return
	var bar: ProgressBar = row["bar"]
	bar.value = 0
	bar.add_theme_stylebox_override("fill", _fill_styles[Status.GREEN])
	(row["value"] as Label).text = "—"


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
		if value >= hard:
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
		return "%.2f GB" % (float(n) / 1073741824.0)
	if n >= 1024 * 1024:
		return "%.1f MB" % (float(n) / 1048576.0)
	if n >= 1024:
		return "%.1f KB" % (float(n) / 1024.0)
	return "%d B" % n


static func _fmt_int(n: int) -> String:
	var s: String = str(absi(n))
	var grouped: String = ""
	var count: int = 0
	for i in range(s.length() - 1, -1, -1):
		grouped = s[i] + grouped
		count += 1
		if count % 3 == 0 and i > 0:
			grouped = "," + grouped
	return ("-" if n < 0 else "") + grouped


func _build_styleboxes() -> void:
	_fill_styles[Status.GREEN] = _make_fill(COLOR_GREEN)
	_fill_styles[Status.YELLOW] = _make_fill(COLOR_YELLOW)
	_fill_styles[Status.RED] = _make_fill(COLOR_RED)
	_bg_style = StyleBoxFlat.new()
	_bg_style.bg_color = Color(0.12, 0.12, 0.15, 0.9)
	_bg_style.set_corner_radius_all(3)


func _make_fill(c: Color) -> StyleBoxFlat:
	var sb: StyleBoxFlat = StyleBoxFlat.new()
	sb.bg_color = c
	sb.set_corner_radius_all(3)
	return sb


func _make_circle(c: Color) -> StyleBoxFlat:
	var sb: StyleBoxFlat = StyleBoxFlat.new()
	sb.bg_color = c
	sb.set_corner_radius_all(int(TOGGLE_SIZE / 2.0))
	sb.content_margin_left = 12.0
	sb.content_margin_right = 12.0
	sb.content_margin_top = 12.0
	sb.content_margin_bottom = 12.0
	return sb


## Build the red monitor toggle button and place it to the LEFT of the camera
## (1st/3rd person) button at the same size; fall back to the panel's parent.
func _create_toggle_button() -> void:
	_toggle_btn = Button.new()
	_toggle_btn.custom_minimum_size = Vector2(TOGGLE_SIZE, TOGGLE_SIZE)
	_toggle_btn.focus_mode = Control.FOCUS_NONE
	_toggle_btn.add_theme_stylebox_override("normal", _make_circle(Color(0, 0, 0, 0.62)))
	_toggle_btn.add_theme_stylebox_override("hover", _make_circle(Color(0.22, 0.22, 0.28, 0.85)))
	_toggle_btn.add_theme_stylebox_override("pressed", _make_circle(Color(0.22, 0.22, 0.28, 0.85)))
	_toggle_btn.pressed.connect(_on_toggle_pressed)

	# Icon as a centered child TextureRect — guarantees centering and sizing
	# regardless of Button icon-layout quirks.
	var icon_rect: TextureRect = TextureRect.new()
	icon_rect.texture = load(MONITOR_ICON_PATH)
	icon_rect.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	icon_rect.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	icon_rect.set_anchors_preset(Control.PRESET_FULL_RECT)
	icon_rect.offset_left = 15.0
	icon_rect.offset_top = 15.0
	icon_rect.offset_right = -15.0
	icon_rect.offset_bottom = -15.0
	icon_rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
	icon_rect.modulate = COLOR_MONITOR
	_toggle_btn.add_child(icon_rect)

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


func _find_camera_button() -> Control:
	if not is_inside_tree():
		return null
	var node := get_tree().get_root().find_child("Button_Camera", true, false)
	return node as Control


func _build_rows() -> void:
	_rows_box.add_theme_constant_override("separation", 2)
	var last_group: String = ""
	for meta in SceneLimits.metric_order():
		var key: String = meta["key"]
		var group: String = str(meta.get("group", "scene"))
		if group != last_group:
			last_group = group
			_rows_box.add_child(_make_header(group))

		var row: HBoxContainer = HBoxContainer.new()
		row.mouse_filter = Control.MOUSE_FILTER_IGNORE
		row.add_theme_constant_override("separation", 6)

		var name_lbl: Label = Label.new()
		name_lbl.text = str(meta["label"])
		name_lbl.custom_minimum_size = Vector2(108, 0)
		name_lbl.mouse_filter = Control.MOUSE_FILTER_IGNORE
		name_lbl.add_theme_font_override("font", UI_FONT)
		name_lbl.add_theme_font_size_override("font_size", 12)
		name_lbl.add_theme_color_override("font_color", COLOR_TEXT)

		var bar: ProgressBar = ProgressBar.new()
		bar.show_percentage = false
		bar.custom_minimum_size = Vector2(180, 21)
		bar.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		bar.size_flags_vertical = Control.SIZE_SHRINK_CENTER
		bar.mouse_filter = Control.MOUSE_FILTER_IGNORE
		bar.add_theme_stylebox_override("background", _bg_style)
		bar.add_theme_stylebox_override("fill", _fill_styles[Status.GREEN])

		# Value text drawn INSIDE the bar — white with a dark outline + shadow so
		# it stays readable over any fill color (green/yellow/red) and the track.
		var val_lbl: Label = Label.new()
		val_lbl.set_anchors_preset(Control.PRESET_FULL_RECT)
		val_lbl.offset_left = 6
		val_lbl.offset_right = -6
		val_lbl.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
		val_lbl.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
		val_lbl.mouse_filter = Control.MOUSE_FILTER_IGNORE
		val_lbl.add_theme_font_override("font", UI_FONT)
		val_lbl.add_theme_font_size_override("font_size", 12)
		val_lbl.add_theme_color_override("font_color", Color.WHITE)
		val_lbl.add_theme_color_override("font_outline_color", Color(0, 0, 0, 1.0))
		val_lbl.add_theme_constant_override("outline_size", 5)
		val_lbl.add_theme_color_override("font_shadow_color", Color(0, 0, 0, 0.6))
		val_lbl.add_theme_constant_override("shadow_offset_x", 1)
		val_lbl.add_theme_constant_override("shadow_offset_y", 1)
		bar.add_child(val_lbl)

		row.add_child(name_lbl)
		row.add_child(bar)
		_rows_box.add_child(row)
		_rows[key] = {"bar": bar, "value": val_lbl}


func _make_header(group: String) -> Label:
	var hdr: Label = Label.new()
	hdr.text = "— Per scene —" if group == "scene" else "— Whole app (not per-scene) —"
	hdr.mouse_filter = Control.MOUSE_FILTER_IGNORE
	hdr.add_theme_font_override("font", UI_FONT)
	hdr.add_theme_font_size_override("font_size", 11)
	hdr.add_theme_color_override("font_color", COLOR_DIM)
	return hdr
