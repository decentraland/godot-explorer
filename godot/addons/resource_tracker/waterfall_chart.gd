@tool
class_name WaterfallChart
extends Control

## Waterfall chart visualization for resource loading timeline
## Similar to Chrome DevTools Network panel

signal resource_selected(hash_id: String)
signal resource_hovered(hash_id: String, position: Vector2)

enum TimelineMode { FIT, EXPLORE, LIVE }

# Visual configuration
const ROW_HEIGHT: int = 20
const ROW_SPACING: int = 2
const TIMELINE_HEADER_HEIGHT: int = 28
const LEFT_PANEL_WIDTH: int = 180
const MIN_SCALE: float = 0.001  # 1px = 1000ms
const MAX_SCALE: float = 1.0  # 1px = 1ms
const SCROLLBAR_WIDTH: int = 8
const SCROLLBAR_MIN_SIZE: int = 20

# Timeline configuration
var timeline_start_tick: int = 0
var timeline_end_tick: int = 1000
var timeline_scale: float = 0.5  # pixels per millisecond
var auto_scale: bool = true
var timeline_mode: TimelineMode = TimelineMode.FIT
var timeline_period_ms: int = 30000  # 30 seconds default

# Data
var resources: Dictionary = {}
var sorted_resource_ids: Array = []
var filter_state: int = -1  # -1 means no filter

# State colors
var state_colors: Dictionary = {
	ResourceTrackerDebugger.ResourceTrackerState.STARTED: Color(0.6, 0.6, 0.6, 0.8),
	ResourceTrackerDebugger.ResourceTrackerState.DOWNLOADING: Color(0.2, 0.6, 0.9, 1.0),
	ResourceTrackerDebugger.ResourceTrackerState.DOWNLOADED: Color(0.3, 0.7, 0.3, 1.0),
	ResourceTrackerDebugger.ResourceTrackerState.LOADING: Color(0.9, 0.7, 0.2, 1.0),
	ResourceTrackerDebugger.ResourceTrackerState.FAILED: Color(0.9, 0.2, 0.2, 1.0),
	ResourceTrackerDebugger.ResourceTrackerState.FINISHED: Color(0.2, 0.8, 0.2, 1.0),
	ResourceTrackerDebugger.ResourceTrackerState.DELETED: Color(0.5, 0.5, 0.5, 0.5),
	ResourceTrackerDebugger.ResourceTrackerState.TIMEOUT: Color(1.0, 0.4, 0.0, 1.0),
}

# Private state
var _global_start_tick: int = 0  # earliest resource start time
var _hovered_resource: String = ""
var _scroll_offset: Vector2 = Vector2.ZERO
var _tooltip_panel: PanelContainer = null
var _last_resource_count: int = 0


func _ready():
	mouse_filter = Control.MOUSE_FILTER_STOP
	clip_contents = true
	_create_tooltip()
	set_process(true)


func _process(_delta: float):
	# In LIVE mode, continuously update the timeline
	if timeline_mode == TimelineMode.LIVE and visible:
		_update_timeline_bounds()
		queue_redraw()


func _create_tooltip():
	_tooltip_panel = PanelContainer.new()
	_tooltip_panel.visible = false
	_tooltip_panel.z_index = 100

	# Add a dark background style
	var style = StyleBoxFlat.new()
	style.bg_color = Color(0.15, 0.15, 0.18, 0.95)
	style.border_color = Color(0.4, 0.4, 0.45, 1.0)
	style.set_border_width_all(1)
	style.set_corner_radius_all(4)
	style.set_content_margin_all(8)
	_tooltip_panel.add_theme_stylebox_override("panel", style)

	var vbox = VBoxContainer.new()
	vbox.name = "VBox"

	var label_type = Label.new()
	label_type.name = "LabelType"
	label_type.add_theme_font_size_override("font_size", 24)
	vbox.add_child(label_type)

	var label_hash = Label.new()
	label_hash.name = "LabelHash"
	label_hash.add_theme_font_size_override("font_size", 22)
	vbox.add_child(label_hash)

	var label_state = Label.new()
	label_state.name = "LabelState"
	label_state.add_theme_font_size_override("font_size", 22)
	vbox.add_child(label_state)

	var label_size = Label.new()
	label_size.name = "LabelSize"
	label_size.add_theme_font_size_override("font_size", 22)
	vbox.add_child(label_size)

	var label_duration = Label.new()
	label_duration.name = "LabelDuration"
	label_duration.add_theme_font_size_override("font_size", 22)
	vbox.add_child(label_duration)

	var separator = HSeparator.new()
	vbox.add_child(separator)

	var label_breakdown = Label.new()
	label_breakdown.name = "LabelBreakdown"
	label_breakdown.add_theme_font_size_override("font_size", 20)
	vbox.add_child(label_breakdown)

	_tooltip_panel.add_child(vbox)
	add_child(_tooltip_panel)


func _draw():
	# Background
	draw_rect(Rect2(Vector2.ZERO, size), Color(0.12, 0.12, 0.14, 1.0))

	# Draw timeline header
	_draw_timeline_header()

	# Draw separator line
	draw_line(
		Vector2(LEFT_PANEL_WIDTH, 0), Vector2(LEFT_PANEL_WIDTH, size.y), Color(0.3, 0.3, 0.3), 1.0
	)

	# Draw resources
	_draw_resources()

	# Draw scrollbars
	_draw_scrollbars()


func _draw_timeline_header():
	# Header background
	draw_rect(
		Rect2(LEFT_PANEL_WIDTH, 0, size.x - LEFT_PANEL_WIDTH, TIMELINE_HEADER_HEIGHT),
		Color(0.18, 0.18, 0.2, 1.0)
	)

	# Left panel header
	draw_rect(Rect2(0, 0, LEFT_PANEL_WIDTH, TIMELINE_HEADER_HEIGHT), Color(0.15, 0.15, 0.17, 1.0))

	var font = ThemeDB.fallback_font
	var font_size = 11

	# Draw "Resource" label
	draw_string(
		font,
		Vector2(8, TIMELINE_HEADER_HEIGHT - 8),
		"Resource",
		HORIZONTAL_ALIGNMENT_LEFT,
		LEFT_PANEL_WIDTH - 16,
		font_size,
		Color(0.8, 0.8, 0.8)
	)

	# Calculate time markers
	var duration = timeline_end_tick - timeline_start_tick
	if duration <= 0:
		return

	var tick_interval = _calculate_tick_interval(duration)
	var first_tick = int(timeline_start_tick / tick_interval) * tick_interval

	var tick_ms = first_tick
	while tick_ms <= timeline_end_tick + tick_interval:
		var x = (
			LEFT_PANEL_WIDTH + (tick_ms - timeline_start_tick) * timeline_scale - _scroll_offset.x
		)
		if x >= LEFT_PANEL_WIDTH and x <= size.x:
			# Draw tick line
			draw_line(
				Vector2(x, TIMELINE_HEADER_HEIGHT - 6),
				Vector2(x, TIMELINE_HEADER_HEIGHT),
				Color(0.5, 0.5, 0.5),
				1.0
			)

			# Draw time label
			var label = _format_time(tick_ms - timeline_start_tick)
			draw_string(
				font,
				Vector2(x + 3, TIMELINE_HEADER_HEIGHT - 10),
				label,
				HORIZONTAL_ALIGNMENT_LEFT,
				-1,
				font_size,
				Color(0.7, 0.7, 0.7)
			)

			# Draw vertical grid line (light)
			draw_line(
				Vector2(x, TIMELINE_HEADER_HEIGHT),
				Vector2(x, size.y),
				Color(0.2, 0.2, 0.22, 0.5),
				1.0
			)

		tick_ms += tick_interval


func _calculate_tick_interval(duration_ms: int) -> int:
	# Choose appropriate tick interval based on visible duration
	var intervals = [10, 50, 100, 250, 500, 1000, 2500, 5000, 10000, 30000, 60000]
	var visible_width = size.x - LEFT_PANEL_WIDTH
	var target_ticks = 8  # Aim for about 8 tick marks

	for interval in intervals:
		var num_ticks = duration_ms / interval
		if num_ticks <= target_ticks * 2:
			return interval

	return 60000  # Default to 1 minute


func _format_time(ms: int) -> String:
	if ms < 1000:
		return str(ms) + "ms"
	if ms < 60000:
		return str(snappedf(ms / 1000.0, 0.1)) + "s"
	return str(snappedf(ms / 60000.0, 0.1)) + "m"


func _draw_resources():
	var font = ThemeDB.fallback_font
	var font_size = 11

	var y = TIMELINE_HEADER_HEIGHT - _scroll_offset.y
	var visible_count = 0

	for hash_id in sorted_resource_ids:
		var resource = resources[hash_id]

		# Apply filter
		if filter_state >= 0 and resource["state"] != filter_state:
			continue

		# Skip if above visible area
		if y + ROW_HEIGHT < TIMELINE_HEADER_HEIGHT:
			y += ROW_HEIGHT + ROW_SPACING
			continue

		# Stop if below visible area
		if y > size.y:
			break

		_draw_resource_row(hash_id, resource, y, font, font_size)
		y += ROW_HEIGHT + ROW_SPACING
		visible_count += 1


func _draw_resource_row(
	hash_id: String, resource: Dictionary, y: float, font: Font, font_size: int
):
	var is_hovered = hash_id == _hovered_resource

	# Background highlight for hover
	if is_hovered:
		draw_rect(Rect2(0, y, size.x, ROW_HEIGHT), Color(0.25, 0.25, 0.3, 0.5))

	# Alternating row background
	var row_index = sorted_resource_ids.find(hash_id)
	if row_index % 2 == 0:
		draw_rect(Rect2(0, y, LEFT_PANEL_WIDTH, ROW_HEIGHT), Color(0.14, 0.14, 0.16, 1.0))

	# Draw resource name (type + truncated hash)
	var resource_type = resource.get("resource_type", "")
	var display_name = hash_id
	if hash_id.length() > 16:
		display_name = hash_id.substr(0, 6) + "..." + hash_id.substr(-6)
	if not resource_type.is_empty():
		display_name = resource_type + ": " + display_name

	var name_color = Color(0.75, 0.75, 0.75)
	if resource["state"] == ResourceTrackerDebugger.ResourceTrackerState.FAILED:
		name_color = Color(1.0, 0.4, 0.4)
	elif resource["state"] == ResourceTrackerDebugger.ResourceTrackerState.TIMEOUT:
		name_color = Color(1.0, 0.6, 0.2)

	draw_string(
		font,
		Vector2(6, y + ROW_HEIGHT - 5),
		display_name,
		HORIZONTAL_ALIGNMENT_LEFT,
		LEFT_PANEL_WIDTH - 12,
		font_size,
		name_color
	)

	# Draw state segments
	var history: Array = resource.get("state_history", [])
	var current_tick = Time.get_ticks_msec()

	for entry in history:
		var state: int = entry["state"]
		var seg_start: int = entry["start_tick"]
		var seg_end: int = entry["end_tick"]
		if seg_end == -1:
			seg_end = current_tick

		var x_start = (
			LEFT_PANEL_WIDTH + (seg_start - timeline_start_tick) * timeline_scale - _scroll_offset.x
		)
		var x_end = (
			LEFT_PANEL_WIDTH + (seg_end - timeline_start_tick) * timeline_scale - _scroll_offset.x
		)

		# Clamp to visible area
		x_start = max(x_start, LEFT_PANEL_WIDTH)
		x_end = min(x_end, size.x - SCROLLBAR_WIDTH)

		if x_end > x_start:
			var bar_rect = Rect2(x_start, y + 3, x_end - x_start, ROW_HEIGHT - 6)
			var bar_color = state_colors.get(state, Color(0.5, 0.5, 0.5))

			# Draw bar
			draw_rect(bar_rect, bar_color)

			# Highlight border for failed/timeout
			if (
				state == ResourceTrackerDebugger.ResourceTrackerState.FAILED
				or state == ResourceTrackerDebugger.ResourceTrackerState.TIMEOUT
			):
				draw_rect(bar_rect, Color(1.0, 0.3, 0.1), false, 2.0)


func _draw_scrollbars():
	var content_height = _calculate_content_height()
	var content_width = _calculate_content_width()
	var view_height = size.y - TIMELINE_HEADER_HEIGHT
	var view_width = size.x - LEFT_PANEL_WIDTH - SCROLLBAR_WIDTH

	var scrollbar_color = Color(0.4, 0.4, 0.45, 0.6)
	var scrollbar_bg = Color(0.2, 0.2, 0.22, 0.3)

	# Vertical scrollbar (right side)
	if content_height > view_height:
		var scrollbar_area_height = size.y - TIMELINE_HEADER_HEIGHT - SCROLLBAR_WIDTH
		var thumb_height = max(
			SCROLLBAR_MIN_SIZE, (view_height / content_height) * scrollbar_area_height
		)
		var max_scroll_y = content_height - view_height
		var scroll_ratio = _scroll_offset.y / max_scroll_y if max_scroll_y > 0 else 0
		var thumb_y = TIMELINE_HEADER_HEIGHT + scroll_ratio * (scrollbar_area_height - thumb_height)

		# Background track
		draw_rect(
			Rect2(
				size.x - SCROLLBAR_WIDTH,
				TIMELINE_HEADER_HEIGHT,
				SCROLLBAR_WIDTH,
				scrollbar_area_height
			),
			scrollbar_bg
		)
		# Thumb
		draw_rect(
			Rect2(size.x - SCROLLBAR_WIDTH + 1, thumb_y, SCROLLBAR_WIDTH - 2, thumb_height),
			scrollbar_color,
			true,
			-1,
			true  # antialiased
		)

	# Horizontal scrollbar (bottom) - only in EXPLORE mode
	if timeline_mode == TimelineMode.EXPLORE and content_width > view_width:
		var scrollbar_area_width = size.x - LEFT_PANEL_WIDTH - SCROLLBAR_WIDTH
		var thumb_width = max(
			SCROLLBAR_MIN_SIZE, (view_width / content_width) * scrollbar_area_width
		)
		var max_scroll_x = content_width - view_width
		var scroll_ratio = _scroll_offset.x / max_scroll_x if max_scroll_x > 0 else 0
		var thumb_x = LEFT_PANEL_WIDTH + scroll_ratio * (scrollbar_area_width - thumb_width)

		# Background track
		draw_rect(
			Rect2(
				LEFT_PANEL_WIDTH, size.y - SCROLLBAR_WIDTH, scrollbar_area_width, SCROLLBAR_WIDTH
			),
			scrollbar_bg
		)
		# Thumb
		draw_rect(
			Rect2(thumb_x, size.y - SCROLLBAR_WIDTH + 1, thumb_width, SCROLLBAR_WIDTH - 2),
			scrollbar_color,
			true,
			-1,
			true  # antialiased
		)


func _gui_input(event: InputEvent):
	if event is InputEventMouseMotion:
		var row_index = _get_row_at_position(event.position)
		var filtered_ids = _get_filtered_resource_ids()

		if row_index >= 0 and row_index < filtered_ids.size():
			var new_hovered = filtered_ids[row_index]
			if new_hovered != _hovered_resource:
				_hovered_resource = new_hovered
				_show_tooltip(resources[_hovered_resource], event.global_position)
				resource_hovered.emit(_hovered_resource, event.position)
				queue_redraw()
		else:
			if _hovered_resource != "":
				_hovered_resource = ""
				_hide_tooltip()
				queue_redraw()

	elif event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_LEFT and event.pressed:
			var row_index = _get_row_at_position(event.position)
			var filtered_ids = _get_filtered_resource_ids()
			if row_index >= 0 and row_index < filtered_ids.size():
				var hash_id = filtered_ids[row_index]
				DisplayServer.clipboard_set(hash_id)
				resource_selected.emit(hash_id)

		# Vertical scroll with wheel
		elif event.button_index == MOUSE_BUTTON_WHEEL_UP and event.pressed:
			if event.shift_pressed:
				_scroll_horizontal(-50)
			else:
				_scroll_vertical(-30)
		elif event.button_index == MOUSE_BUTTON_WHEEL_DOWN and event.pressed:
			if event.shift_pressed:
				_scroll_horizontal(50)
			else:
				_scroll_vertical(30)

		# Native horizontal scroll (touchpad, horizontal mouse wheel)
		elif event.button_index == MOUSE_BUTTON_WHEEL_LEFT and event.pressed:
			_scroll_horizontal(-50)
		elif event.button_index == MOUSE_BUTTON_WHEEL_RIGHT and event.pressed:
			_scroll_horizontal(50)

	elif event is InputEventPanGesture:
		# Touchpad pan gesture support
		_scroll_horizontal(event.delta.x * 20)
		_scroll_vertical(event.delta.y * 20)


func _scroll_vertical(delta: float):
	var max_y = _calculate_content_height() - (size.y - TIMELINE_HEADER_HEIGHT)
	_scroll_offset.y = clampf(_scroll_offset.y + delta, 0, max(max_y, 0))
	queue_redraw()


func _scroll_horizontal(delta: float):
	# Disable horizontal scroll in FIT and LIVE modes
	if timeline_mode == TimelineMode.FIT or timeline_mode == TimelineMode.LIVE:
		return

	var max_x = _calculate_content_width() - (size.x - LEFT_PANEL_WIDTH - SCROLLBAR_WIDTH)
	_scroll_offset.x = clampf(_scroll_offset.x + delta, 0, max(max_x, 0))
	queue_redraw()


func _calculate_content_width() -> float:
	# For EXPLORE mode, content width is the full timeline duration
	if timeline_mode == TimelineMode.EXPLORE and resources.size() > 0:
		var current_tick = Time.get_ticks_msec()
		var global_end_tick = 0
		for hash_id in resources:
			var resource = resources[hash_id]
			var history: Array = resource.get("state_history", [])
			for entry in history:
				var end_tick = entry["end_tick"]
				if end_tick == -1:
					end_tick = current_tick
				global_end_tick = max(global_end_tick, end_tick)
		var total_duration = global_end_tick - _global_start_tick + 500  # Small padding
		return total_duration * timeline_scale

	var duration = timeline_end_tick - timeline_start_tick
	return duration * timeline_scale


func _get_row_at_position(pos: Vector2) -> int:
	if pos.y < TIMELINE_HEADER_HEIGHT:
		return -1
	return int((pos.y - TIMELINE_HEADER_HEIGHT + _scroll_offset.y) / (ROW_HEIGHT + ROW_SPACING))


func _get_filtered_resource_ids() -> Array:
	if filter_state < 0:
		return sorted_resource_ids

	var filtered = []
	for hash_id in sorted_resource_ids:
		if resources[hash_id]["state"] == filter_state:
			filtered.append(hash_id)
	return filtered


func _calculate_content_height() -> float:
	var count = _get_filtered_resource_ids().size()
	return count * (ROW_HEIGHT + ROW_SPACING)


func _show_tooltip(resource: Dictionary, global_pos: Vector2):
	if not _tooltip_panel:
		return

	var vbox = _tooltip_panel.get_node("VBox")
	var resource_type = resource.get("resource_type", "")
	vbox.get_node("LabelType").text = (
		"Type: " + (resource_type if not resource_type.is_empty() else "unknown")
	)
	vbox.get_node("LabelHash").text = "Hash: " + resource["hash_id"]
	vbox.get_node("LabelState").text = (
		"State: " + ResourceTrackerDebugger.get_resource_state_string(resource["state"])
	)
	vbox.get_node("LabelSize").text = "Size: " + resource["size"]
	vbox.get_node("LabelDuration").text = "Total: " + str(resource["elapsed"]) + "s"

	# Build state breakdown
	var breakdown = "Timeline:\n"
	var history: Array = resource.get("state_history", [])
	var current_tick = Time.get_ticks_msec()

	for entry in history:
		var state_name = ResourceTrackerDebugger.get_resource_state_string(entry["state"])
		var duration_ms = entry["end_tick"] - entry["start_tick"]
		if entry["end_tick"] == -1:
			duration_ms = current_tick - entry["start_tick"]
		breakdown += "  %s: %s\n" % [state_name, _format_time(duration_ms)]

	vbox.get_node("LabelBreakdown").text = breakdown

	# Position tooltip
	var local_pos = global_pos - global_position
	_tooltip_panel.position = local_pos + Vector2(15, 15)

	# Keep tooltip in bounds
	var tooltip_size = _tooltip_panel.size
	if _tooltip_panel.position.x + tooltip_size.x > size.x:
		_tooltip_panel.position.x = local_pos.x - tooltip_size.x - 15
	if _tooltip_panel.position.y + tooltip_size.y > size.y:
		_tooltip_panel.position.y = local_pos.y - tooltip_size.y - 15

	_tooltip_panel.visible = true


func _hide_tooltip():
	if _tooltip_panel:
		_tooltip_panel.visible = false


# Public API


func set_resources(new_resources: Dictionary):
	resources = new_resources
	_update_sorted_ids()
	_update_timeline_bounds()

	# In LIVE mode, auto-scroll to bottom when new items arrive
	var new_count = resources.size()
	if timeline_mode == TimelineMode.LIVE and new_count > _last_resource_count:
		_scroll_to_bottom()
	_last_resource_count = new_count

	queue_redraw()


func _scroll_to_bottom():
	var content_height = _calculate_content_height()
	var view_height = size.y - TIMELINE_HEADER_HEIGHT
	_scroll_offset.y = max(0, content_height - view_height)


func clear_resources():
	resources.clear()
	sorted_resource_ids.clear()
	timeline_start_tick = 0
	timeline_end_tick = 1000
	_scroll_offset = Vector2.ZERO
	_last_resource_count = 0
	queue_redraw()


func set_filter_state(state: int):
	filter_state = state
	queue_redraw()


func set_timeline_mode(mode: int, period_ms: int):
	timeline_mode = mode as TimelineMode
	timeline_period_ms = period_ms
	auto_scale = (timeline_mode == TimelineMode.FIT)
	_update_timeline_bounds()
	queue_redraw()


func _update_sorted_ids():
	sorted_resource_ids = resources.keys()
	sorted_resource_ids.sort_custom(
		func(a, b): return resources[a]["start_tick"] < resources[b]["start_tick"]
	)


func _update_timeline_bounds():
	if resources.is_empty():
		_global_start_tick = 0
		timeline_start_tick = 0
		timeline_end_tick = 1000
		return

	var current_tick = Time.get_ticks_msec()

	# Find global bounds first
	_global_start_tick = 9999999999
	var global_end_tick = 0

	for hash_id in resources:
		var resource = resources[hash_id]
		_global_start_tick = min(_global_start_tick, resource["start_tick"])

		var history: Array = resource.get("state_history", [])
		for entry in history:
			var end_tick = entry["end_tick"]
			if end_tick == -1:
				end_tick = current_tick
			global_end_tick = max(global_end_tick, end_tick)

	# Apply timeline mode
	match timeline_mode:
		TimelineMode.FIT:
			# Show everything
			timeline_start_tick = _global_start_tick
			timeline_end_tick = global_end_tick
			# Add some padding
			var duration = timeline_end_tick - timeline_start_tick
			timeline_end_tick += max(duration * 0.05, 100)
			_fit_timeline_to_view()

		TimelineMode.EXPLORE:
			# Show a fixed time window that can be scrolled across full timeline
			# Timeline covers full duration, but scale is based on period
			timeline_start_tick = _global_start_tick
			timeline_end_tick = global_end_tick + 500  # Small padding
			_fit_period_to_view()

		TimelineMode.LIVE:
			# Show the last period_ms, auto-scrolling to follow new data
			timeline_end_tick = current_tick + 500  # Small buffer ahead
			timeline_start_tick = timeline_end_tick - timeline_period_ms
			# Auto-scroll to the end
			_scroll_offset.x = max(0, _calculate_content_width() - (size.x - LEFT_PANEL_WIDTH))
			_fit_period_to_view()


func _fit_timeline_to_view():
	var available_width = size.x - LEFT_PANEL_WIDTH
	var duration = timeline_end_tick - timeline_start_tick
	if duration > 0 and available_width > 0:
		timeline_scale = available_width / float(duration)
		timeline_scale = clampf(timeline_scale, MIN_SCALE, MAX_SCALE)
	_scroll_offset.x = 0  # Reset horizontal scroll in FIT mode


func _fit_period_to_view():
	# Scale so the period fits the view width
	var available_width = size.x - LEFT_PANEL_WIDTH - SCROLLBAR_WIDTH
	if available_width > 0 and timeline_period_ms > 0:
		timeline_scale = available_width / float(timeline_period_ms)
		timeline_scale = clampf(timeline_scale, MIN_SCALE, MAX_SCALE)
