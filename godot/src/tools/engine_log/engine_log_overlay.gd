extends CanvasLayer

# Invisible input padding behind and around the log window.
# The panel itself remains interactive, this only catches near-miss touches.
const INPUT_BLOCK_MARGIN := 24.0

var _input_blocker: ColorRect = null
var _focus_block_pointer_active := false
var _focus_block_touch_index := -1

const REFRESH_INTERVAL := 0.20

# Starts smaller by default and is clamped to the visible viewport afterwards.
const START_PANEL_SIZE := Vector2(820, 460)

# Minimum resize size. On small screens, this minimum is reduced automatically.
const MIN_PANEL_SIZE := Vector2(520, 92)
const MAX_PANEL_SIZE := Vector2(2200, 1600)

const VIEWPORT_MARGIN := 24.0
const HEADER_HEIGHT := 52.0
const RESIZE_CORNER_SIZE := 50.0
const RESIZE_EDGE_THICKNESS := 45.0
const AUTO_SCROLL_THRESHOLD := 24.0

var _service: Node = null
var _refresh_accumulator := 0.0
var _last_line_count := -1
var _last_dropped_count := -1
var _paused := false
var _follow_tail := true

var _root: Control = null
var _panel: PanelContainer = null
var _header: PanelContainer = null
var _title_label: Label = null
var _dropped_label: Label = null
var _pause_button: Button = null
var _follow_button: Button = null

var _scroll_container: ScrollContainer = null
var _log_label: RichTextLabel = null

var _drag_handle: Label = null
var _resize_zones: Array[Control] = []
var _resize_edge := Vector2i.ZERO
var _resize_start_position := Vector2.ZERO
var _drag_touch_index := -1
var _resize_touch_index := -1

var _dragging_panel := false
var _drag_offset := Vector2.ZERO

var _dragging_scroll := false
var _last_drag_y := 0.0

var _resizing := false
var _resize_start_mouse := Vector2.ZERO
var _resize_start_size := Vector2.ZERO

var _released_explorer_focus := false

func setup(service: Node) -> void:
	_service = service
	_refresh(true)


func _ready() -> void:
	layer = 4096
	process_mode = Node.PROCESS_MODE_ALWAYS

	_build_ui()

	if not get_viewport().size_changed.is_connected(_on_viewport_size_changed):
		get_viewport().size_changed.connect(_on_viewport_size_changed)

	_center_panel()
	_refresh(true)

func _gui_event_to_viewport_pos(control: Control, event: InputEvent) -> Vector2:
	if event is InputEventMouseButton or event is InputEventMouseMotion:
		return get_viewport().get_mouse_position()

	if event is InputEventScreenTouch:
		return control.get_global_transform_with_canvas() * event.position

	if event is InputEventScreenDrag:
		return control.get_global_transform_with_canvas() * event.position

	return Vector2.ZERO

func _on_viewport_size_changed() -> void:
	if _panel == null:
		return

	var max_size := _get_effective_max_panel_size()
	_panel.size.x = minf(_panel.size.x, max_size.x)
	_panel.size.y = minf(_panel.size.y, max_size.y)

	_clamp_panel_to_viewport()
	_update_interaction_zones()

func _build_ui() -> void:
	_root = Control.new()
	_root.name = "Root"
	_root.set_anchors_preset(Control.PRESET_FULL_RECT)
	_root.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(_root)

	_input_blocker = ColorRect.new()
	_input_blocker.name = "InputBlocker"
	_input_blocker.color = Color(0, 0, 0, 0)
	_input_blocker.mouse_filter = Control.MOUSE_FILTER_STOP
	_input_blocker.gui_input.connect(_on_input_blocker_gui_input)
	_root.add_child(_input_blocker)

	_panel = PanelContainer.new()
	_panel.name = "Panel"
	_panel.custom_minimum_size = Vector2.ZERO
	_panel.size = START_PANEL_SIZE
	_panel.mouse_filter = Control.MOUSE_FILTER_STOP
	_panel.gui_input.connect(_on_panel_gui_input)
	_root.add_child(_panel)

	var panel_style := StyleBoxFlat.new()
	panel_style.bg_color = Color(0.03, 0.03, 0.03, 0.96)
	panel_style.border_width_left = 2
	panel_style.border_width_top = 2
	panel_style.border_width_right = 2
	panel_style.border_width_bottom = 2
	panel_style.border_color = Color(0.30, 0.30, 0.30, 1.0)
	panel_style.corner_radius_top_left = 14
	panel_style.corner_radius_top_right = 14
	panel_style.corner_radius_bottom_left = 14
	panel_style.corner_radius_bottom_right = 14
	_panel.add_theme_stylebox_override("panel", panel_style)

	var outer_margin := MarginContainer.new()
	outer_margin.add_theme_constant_override("margin_left", 10)
	outer_margin.add_theme_constant_override("margin_top", 10)
	outer_margin.add_theme_constant_override("margin_right", 10)
	outer_margin.add_theme_constant_override("margin_bottom", 10)
	_panel.add_child(outer_margin)

	var vbox := VBoxContainer.new()
	vbox.add_theme_constant_override("separation", 8)
	outer_margin.add_child(vbox)

	# Header
	_header = PanelContainer.new()
	_header.custom_minimum_size = Vector2(0, HEADER_HEIGHT)
	_header.mouse_filter = Control.MOUSE_FILTER_STOP
	_header.gui_input.connect(_on_header_gui_input)
	vbox.add_child(_header)

	var header_style := StyleBoxFlat.new()
	header_style.bg_color = Color(0.08, 0.08, 0.08, 1.0)
	header_style.corner_radius_top_left = 10
	header_style.corner_radius_top_right = 10
	header_style.corner_radius_bottom_left = 10
	header_style.corner_radius_bottom_right = 10
	header_style.border_width_left = 1
	header_style.border_width_top = 1
	header_style.border_width_right = 1
	header_style.border_width_bottom = 1
	header_style.border_color = Color(0.20, 0.20, 0.20, 1.0)
	_header.add_theme_stylebox_override("panel", header_style)

	var header_margin := MarginContainer.new()
	header_margin.mouse_filter = Control.MOUSE_FILTER_IGNORE
	header_margin.add_theme_constant_override("margin_left", 12)
	header_margin.add_theme_constant_override("margin_top", 8)
	header_margin.add_theme_constant_override("margin_right", 12)
	header_margin.add_theme_constant_override("margin_bottom", 8)
	_header.add_child(header_margin)

	var header_hbox := HBoxContainer.new()
	header_hbox.mouse_filter = Control.MOUSE_FILTER_IGNORE
	header_hbox.alignment = BoxContainer.ALIGNMENT_CENTER
	header_hbox.add_theme_constant_override("separation", 8)
	header_margin.add_child(header_hbox)

	_drag_handle = Label.new()
	_drag_handle.text = "☰"
	_drag_handle.tooltip_text = "Drag window"
	_drag_handle.custom_minimum_size = Vector2(38, 34)
	_drag_handle.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_drag_handle.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	_drag_handle.mouse_filter = Control.MOUSE_FILTER_STOP
	_drag_handle.mouse_default_cursor_shape = Control.CURSOR_MOVE
	_drag_handle.add_theme_color_override("font_color", Color(0.85, 0.85, 0.85, 1.0))
	_drag_handle.add_theme_font_size_override("font_size", 20)
	_drag_handle.gui_input.connect(_on_drag_handle_gui_input)
	header_hbox.add_child(_drag_handle)

	_title_label = Label.new()
	_title_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_title_label.text = "Engine Logs"
	_title_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_title_label.add_theme_color_override("font_color", Color.WHITE)
	_title_label.add_theme_font_size_override("font_size", 16)
	header_hbox.add_child(_title_label)

	_dropped_label = Label.new()
	_dropped_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_dropped_label.text = ""
	_dropped_label.add_theme_color_override("font_color", Color(0.8, 0.8, 0.8, 1.0))
	_dropped_label.add_theme_font_size_override("font_size", 14)
	header_hbox.add_child(_dropped_label)

	_follow_button = _make_header_button("Tail: On")
	_follow_button.pressed.connect(_on_follow_tail_pressed)
	header_hbox.add_child(_follow_button)

	var clear_button := _make_header_button("Clear")
	clear_button.pressed.connect(_on_clear_pressed)
	header_hbox.add_child(clear_button)

	_pause_button = _make_header_button("Pause")
	_pause_button.pressed.connect(_on_pause_pressed)
	header_hbox.add_child(_pause_button)

	# Log area
	var log_panel := PanelContainer.new()
	log_panel.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	log_panel.size_flags_vertical = Control.SIZE_EXPAND_FILL
	vbox.add_child(log_panel)

	var log_style := StyleBoxFlat.new()
	log_style.bg_color = Color(0.0, 0.0, 0.0, 0.96)
	log_style.border_width_left = 1
	log_style.border_width_top = 1
	log_style.border_width_right = 1
	log_style.border_width_bottom = 1
	log_style.border_color = Color(0.28, 0.28, 0.28, 1.0)
	log_style.corner_radius_top_left = 10
	log_style.corner_radius_top_right = 10
	log_style.corner_radius_bottom_left = 10
	log_style.corner_radius_bottom_right = 10
	log_panel.add_theme_stylebox_override("panel", log_style)

	var log_margin := MarginContainer.new()
	log_margin.add_theme_constant_override("margin_left", 8)
	log_margin.add_theme_constant_override("margin_top", 8)
	log_margin.add_theme_constant_override("margin_right", 8)
	log_margin.add_theme_constant_override("margin_bottom", 8)
	log_panel.add_child(log_margin)

	_scroll_container = ScrollContainer.new()
	_scroll_container.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_scroll_container.size_flags_vertical = Control.SIZE_EXPAND_FILL
	_scroll_container.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	_scroll_container.vertical_scroll_mode = ScrollContainer.SCROLL_MODE_AUTO
	_scroll_container.mouse_filter = Control.MOUSE_FILTER_STOP
	_scroll_container.gui_input.connect(_on_scroll_gui_input)
	log_margin.add_child(_scroll_container)

	_log_label = RichTextLabel.new()
	_log_label.bbcode_enabled = true
	_log_label.fit_content = true
	_log_label.scroll_active = false
	_log_label.selection_enabled = true
	_log_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_log_label.size_flags_vertical = Control.SIZE_EXPAND_FILL
	_log_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_log_label.add_theme_font_size_override("normal_font_size", 14)
	_log_label.add_theme_constant_override("line_separation", 0)
	_scroll_container.add_child(_log_label)
	
	_build_resize_zones()

func _build_resize_zones() -> void:
	# Side edges. The top edge is intentionally omitted because the header is used for dragging.
	_add_resize_zone("ResizeLeft", Vector2i(-1, 0), Control.CURSOR_HSIZE)
	_add_resize_zone("ResizeRight", Vector2i(1, 0), Control.CURSOR_HSIZE)
	_add_resize_zone("ResizeBottom", Vector2i(0, 1), Control.CURSOR_VSIZE)

	# Corners
	_add_resize_zone("ResizeTopLeft", Vector2i(-1, -1), Control.CURSOR_FDIAGSIZE)
	_add_resize_zone("ResizeTopRight", Vector2i(1, -1), Control.CURSOR_BDIAGSIZE)
	_add_resize_zone("ResizeBottomLeft", Vector2i(-1, 1), Control.CURSOR_BDIAGSIZE)
	_add_resize_zone("ResizeBottomRight", Vector2i(1, 1), Control.CURSOR_FDIAGSIZE)

	_update_interaction_zones()


func _add_resize_zone(name: String, edge: Vector2i, cursor_shape: int) -> void:
	var zone := ColorRect.new()
	zone.name = name
	zone.color = Color(1, 1, 1, 0.0)
	zone.mouse_filter = Control.MOUSE_FILTER_STOP
	zone.mouse_default_cursor_shape = cursor_shape
	zone.gui_input.connect(_on_resize_zone_gui_input.bind(edge, zone))
	_root.add_child(zone)
	_resize_zones.append(zone)

func _make_header_button(text: String) -> Button:
	var button := Button.new()
	button.text = text
	button.custom_minimum_size = Vector2(84, 34)
	button.focus_mode = Control.FOCUS_NONE
	button.mouse_filter = Control.MOUSE_FILTER_STOP
	button.add_theme_font_size_override("font_size", 14)
	return button


func _process(delta: float) -> void:
	_refresh_accumulator += delta

	if _refresh_accumulator >= REFRESH_INTERVAL:
		_refresh_accumulator = 0.0
		if not _paused:
			_refresh(false)


func _center_panel() -> void:
	var viewport_rect := get_viewport().get_visible_rect()
	var min_size := _get_effective_min_panel_size()
	var max_size := _get_effective_max_panel_size()

	var target_size := START_PANEL_SIZE
	target_size.x = clampf(target_size.x, min_size.x, max_size.x)
	target_size.y = clampf(target_size.y, min_size.y, max_size.y)

	_panel.size = target_size
	_panel.position = viewport_rect.position + (viewport_rect.size - _panel.size) * 0.5

	_clamp_panel_to_viewport()
	_update_interaction_zones()


func _update_interaction_zones() -> void:
	if _panel == null:
		return

	var p := _panel.position
	var s := _panel.size
	var outer_padding := INPUT_BLOCK_MARGIN
	var corner_size := minf(RESIZE_CORNER_SIZE, minf(s.x, s.y) * 0.5)
	var edge_thickness := minf(RESIZE_EDGE_THICKNESS, minf(s.x, s.y) * 0.5)

	var corner_zone_size := Vector2(
		corner_size + outer_padding,
		corner_size + outer_padding
	)

	for zone in _resize_zones:
		match zone.name:
			"ResizeLeft":
				zone.position = p + Vector2(-outer_padding, corner_size)
				zone.size = Vector2(
					edge_thickness + outer_padding,
					maxf(0.0, s.y - corner_size * 2.0)
				)

			"ResizeRight":
				zone.position = p + Vector2(s.x - edge_thickness, corner_size)
				zone.size = Vector2(
					edge_thickness + outer_padding,
					maxf(0.0, s.y - corner_size * 2.0)
				)

			"ResizeBottom":
				zone.position = p + Vector2(corner_size, s.y - edge_thickness)
				zone.size = Vector2(
					maxf(0.0, s.x - corner_size * 2.0),
					edge_thickness + outer_padding
				)

			"ResizeTopLeft":
				zone.position = p + Vector2(-outer_padding, -outer_padding)
				zone.size = corner_zone_size

			"ResizeTopRight":
				zone.position = p + Vector2(s.x - corner_size, -outer_padding)
				zone.size = corner_zone_size

			"ResizeBottomLeft":
				zone.position = p + Vector2(-outer_padding, s.y - corner_size)
				zone.size = corner_zone_size

			"ResizeBottomRight":
				zone.position = p + Vector2(s.x - corner_size, s.y - corner_size)
				zone.size = corner_zone_size

	_update_input_blocker_rect()

func _refresh(force: bool) -> void:
	if _service == null or not is_instance_valid(_service):
		return

	var lines: Array = []
	var dropped_count := 0

	if _service.has_method("get_lines"):
		lines = _service.call("get_lines")

	if _service.has_method("get_dropped_count"):
		dropped_count = int(_service.call("get_dropped_count"))

	if not force and lines.size() == _last_line_count and dropped_count == _last_dropped_count:
		return

	var should_autoscroll := _follow_tail or _is_near_bottom()

	_last_line_count = lines.size()
	_last_dropped_count = dropped_count

	_title_label.text = "Engine Logs (%d)" % lines.size()
	_dropped_label.text = "Dropped: %d" % dropped_count if dropped_count > 0 else ""

	_log_label.text = _build_bbcode(lines)

	await get_tree().process_frame

	if should_autoscroll and not _paused:
		_scroll_to_bottom()


func _build_bbcode(lines: Array) -> String:
	var out: PackedStringArray = []

	for raw_line in lines:
		var line := str(raw_line)
		out.append(_format_log_entry(line))

	return "\n".join(out)


func _format_log_entry(line: String) -> String:
	var prefix_color := "#FFFFFF"
	var body_color := "#EAEAEA"

	if line.begins_with("[WARNING]"):
		prefix_color = "#FFD166"
		body_color = "#FFF1BF"
	elif line.begins_with("[ERROR]"):
		prefix_color = "#FF5C5C"
		body_color = "#FFD0D0"
	elif line.begins_with("[SCRIPT]"):
		prefix_color = "#B388FF"
		body_color = "#E2D3FF"
	elif line.begins_with("[SHADER]"):
		prefix_color = "#4FC3F7"
		body_color = "#D7F3FF"
	elif line.begins_with("[LOG]"):
		prefix_color = "#7EE787"
		body_color = "#E7FFE9"

	var parts := line.split("\n")
	if parts.is_empty():
		return ""

	var first_line := parts[0]
	var formatted := PackedStringArray()

	var prefix_end := first_line.find("] ")
	if prefix_end != -1:
		var prefix := first_line.substr(0, prefix_end + 1)
		var body := first_line.substr(prefix_end + 2)
		formatted.append(
			"[color=%s][b]%s[/b][/color] [color=%s]%s[/color]"
			% [prefix_color, _escape_bbcode(prefix), body_color, _escape_bbcode(body)]
		)
	else:
		formatted.append("[color=%s]%s[/color]" % [body_color, _escape_bbcode(first_line)])

	for i in range(1, parts.size()):
		var continuation := "    " + parts[i]
		formatted.append("[color=#AAAAAA]%s[/color]" % _escape_bbcode(continuation))

	return "\n".join(formatted)


func _escape_bbcode(text: String) -> String:
	return text.replace("[", "[lb]").replace("]", "[rb]")


func _is_near_bottom() -> bool:
	var vbar := _scroll_container.get_v_scroll_bar()
	if vbar == null:
		return true

	return (vbar.max_value - vbar.value) <= AUTO_SCROLL_THRESHOLD


func _scroll_to_bottom() -> void:
	var vbar := _scroll_container.get_v_scroll_bar()
	if vbar != null:
		_scroll_container.scroll_vertical = int(vbar.max_value)


func _on_clear_pressed() -> void:
	if _service != null and is_instance_valid(_service) and _service.has_method("clear"):
		_service.call("clear")

	_last_line_count = -1
	_last_dropped_count = -1
	_refresh(true)


func _on_pause_pressed() -> void:
	_paused = not _paused
	_pause_button.text = "Play" if _paused else "Pause"

	if not _paused:
		_last_line_count = -1
		_last_dropped_count = -1
		_refresh(true)

func _on_follow_tail_pressed() -> void:
	_set_follow_tail(not _follow_tail)


func _set_follow_tail(enabled: bool) -> void:
	_follow_tail = enabled
	_update_follow_tail_button()

	if _follow_tail:
		_scroll_to_bottom()


func _update_follow_tail_button() -> void:
	if _follow_button == null:
		return

	_follow_button.text = "Tail: On" if _follow_tail else "Tail: Off"

func _block_explorer_input() -> void:
	if _released_explorer_focus:
		return

	var explorer = Global.get_explorer()
	if not is_instance_valid(explorer):
		return

	# Prevent Explorer camera/player input while interacting with the overlay.
	Global.explorer_release_focus()

	if Global.is_mobile():
		explorer.release_mouse()

	_released_explorer_focus = true

func _restore_explorer_input_if_safe() -> void:
	if not _released_explorer_focus:
		return

	var explorer = Global.get_explorer()
	if not is_instance_valid(explorer):
		_released_explorer_focus = false
		return

	# Do not restore camera/player input while another Explorer panel is open.
	if _explorer_has_open_blocking_panel(explorer):
		return

	Global.explorer_grab_focus()
	_released_explorer_focus = false

func _exit_tree() -> void:
	_restore_explorer_input_if_safe()
	
func _explorer_has_open_blocking_panel(explorer) -> bool:
	return (
		(is_instance_valid(explorer.settings_panel) and explorer.settings_panel.visible)
		or (is_instance_valid(explorer.friends_panel) and explorer.friends_panel.visible)
		or (is_instance_valid(explorer.notifications_panel) and explorer.notifications_panel.visible)
		or (is_instance_valid(explorer.profile_container) and explorer.profile_container.visible)
		or (is_instance_valid(explorer.control_menu) and explorer.control_menu.visible)
	)

func _clamp_panel_to_viewport() -> void:
	if _panel == null:
		return

	var rect := get_viewport().get_visible_rect()
	var max_x := maxf(0.0, rect.size.x - _panel.size.x)
	var max_y := maxf(0.0, rect.size.y - _panel.size.y)

	_panel.position.x = clampf(_panel.position.x, 0.0, max_x)
	_panel.position.y = clampf(_panel.position.y, 0.0, max_y)

func _on_scroll_gui_input(event: InputEvent) -> void:
	_handle_overlay_pointer_event(event)
	# Desktop mouse wheel
	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_WHEEL_UP and event.pressed:
			_set_follow_tail(false)
			_scroll_container.scroll_vertical -= 48
			get_viewport().set_input_as_handled()
		elif event.button_index == MOUSE_BUTTON_WHEEL_DOWN and event.pressed:
			_set_follow_tail(false)
			_scroll_container.scroll_vertical += 48
			get_viewport().set_input_as_handled()
		return

	# Touch drag scrolling
	if event is InputEventScreenTouch:
		var pointer_pos := _gui_event_to_viewport_pos(_scroll_container, event)

		if event.pressed:
			_dragging_scroll = true
			_last_drag_y = pointer_pos.y
		else:
			_dragging_scroll = false

		get_viewport().set_input_as_handled()
		return

	if event is InputEventScreenDrag and _dragging_scroll and not _dragging_panel and not _resizing:
		_set_follow_tail(false)

		var pointer_pos := _gui_event_to_viewport_pos(_scroll_container, event)
		var scroll_delta_y: float = pointer_pos.y - _last_drag_y

		_scroll_container.scroll_vertical -= int(scroll_delta_y)
		_last_drag_y = pointer_pos.y

		get_viewport().set_input_as_handled()
		
func _on_drag_handle_gui_input(event: InputEvent) -> void:
	_handle_overlay_pointer_event(event)
	if _resizing:
		return

	if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT:
		var pointer_pos := _gui_event_to_viewport_pos(_drag_handle, event)

		if event.pressed:
			_begin_panel_drag(pointer_pos, -1)
		else:
			_end_panel_drag()

		get_viewport().set_input_as_handled()
		return

	if event is InputEventScreenTouch:
		var pointer_pos := _gui_event_to_viewport_pos(_drag_handle, event)

		if event.pressed:
			_begin_panel_drag(pointer_pos, event.index)
		else:
			if event.index == _drag_touch_index:
				_end_panel_drag()

		get_viewport().set_input_as_handled()
		return


func _on_resize_zone_gui_input(event: InputEvent, edge: Vector2i, zone: Control) -> void:
	_handle_overlay_pointer_event(event)
	if _dragging_panel:
		return

	if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT:
		var pointer_pos := _gui_event_to_viewport_pos(zone, event)

		if event.pressed:
			_begin_edge_resize(edge, pointer_pos, -1)
		else:
			_end_edge_resize()

		get_viewport().set_input_as_handled()
		return

	if event is InputEventScreenTouch:
		var pointer_pos := _gui_event_to_viewport_pos(zone, event)

		if event.pressed:
			_begin_edge_resize(edge, pointer_pos, event.index)
		else:
			if event.index == _resize_touch_index:
				_end_edge_resize()

		get_viewport().set_input_as_handled()
		return


func _input(event: InputEvent) -> void:
	if _focus_block_pointer_active:
		if (
			event is InputEventMouseButton
			and event.button_index == MOUSE_BUTTON_LEFT
			and not event.pressed
			and _focus_block_touch_index == -1
		):
			_end_local_focus_block(-1)

		if event is InputEventScreenTouch and not event.pressed:
			if _focus_block_touch_index == -1 or event.index == _focus_block_touch_index:
				var ended_touch_index := _focus_block_touch_index
				_end_local_focus_block(ended_touch_index)

	if _dragging_panel:
		if event is InputEventMouseMotion and _drag_touch_index == -1:
			_apply_panel_drag(get_viewport().get_mouse_position())
			get_viewport().set_input_as_handled()
			return

		if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT and not event.pressed:
			_end_panel_drag()
			get_viewport().set_input_as_handled()
			return

		if event is InputEventScreenDrag and event.index == _drag_touch_index:
			_apply_panel_drag(event.position)
			get_viewport().set_input_as_handled()
			return

		if event is InputEventScreenTouch and event.index == _drag_touch_index and not event.pressed:
			_end_panel_drag()
			get_viewport().set_input_as_handled()
			return

	if _resizing:
		if event is InputEventMouseMotion and _resize_touch_index == -1:
			_apply_edge_resize(get_viewport().get_mouse_position())
			get_viewport().set_input_as_handled()
			return

		if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT and not event.pressed:
			_end_edge_resize()
			get_viewport().set_input_as_handled()
			return

		if event is InputEventScreenDrag and event.index == _resize_touch_index:
			_apply_edge_resize(event.position)
			get_viewport().set_input_as_handled()
			return

		if event is InputEventScreenTouch and event.index == _resize_touch_index and not event.pressed:
			_end_edge_resize()
			get_viewport().set_input_as_handled()
			return


func _begin_panel_drag(pointer_pos: Vector2, touch_index: int) -> void:
	if _resizing:
		return

	_dragging_panel = true
	_drag_touch_index = touch_index
	_drag_offset = pointer_pos - _panel.position


func _apply_panel_drag(pointer_pos: Vector2) -> void:
	_panel.position = pointer_pos - _drag_offset
	_clamp_panel_to_viewport()
	_update_interaction_zones()


func _end_panel_drag() -> void:
	_dragging_panel = false
	_drag_touch_index = -1

func _begin_edge_resize(edge: Vector2i, pointer_pos: Vector2, touch_index: int) -> void:
	if _dragging_panel:
		_end_panel_drag()

	_resizing = true
	_resize_touch_index = touch_index
	_resize_edge = edge
	_resize_start_mouse = pointer_pos
	_resize_start_size = _panel.size
	_resize_start_position = _panel.position

func _apply_edge_resize(pointer_pos: Vector2) -> void:
	var delta := pointer_pos - _resize_start_mouse
	var min_size := _get_effective_min_panel_size()
	var max_size := _get_effective_max_panel_size()

	var new_position := _resize_start_position
	var new_size := _resize_start_size

	if _resize_edge.x < 0:
		new_size.x = clampf(_resize_start_size.x - delta.x, min_size.x, max_size.x)
		new_position.x = _resize_start_position.x + (_resize_start_size.x - new_size.x)
	elif _resize_edge.x > 0:
		new_size.x = clampf(_resize_start_size.x + delta.x, min_size.x, max_size.x)

	if _resize_edge.y < 0:
		new_size.y = clampf(_resize_start_size.y - delta.y, min_size.y, max_size.y)
		new_position.y = _resize_start_position.y + (_resize_start_size.y - new_size.y)
	elif _resize_edge.y > 0:
		new_size.y = clampf(_resize_start_size.y + delta.y, min_size.y, max_size.y)

	_panel.position = new_position
	_panel.size = new_size

	_clamp_panel_to_viewport()
	_update_interaction_zones()

func _end_edge_resize() -> void:
	_resizing = false
	_resize_touch_index = -1
	_resize_edge = Vector2i.ZERO
	
func _get_viewport_panel_limit() -> Vector2:
	var viewport_size := get_viewport().get_visible_rect().size

	return Vector2(
		maxf(240.0, viewport_size.x - VIEWPORT_MARGIN * 2.0),
		maxf(180.0, viewport_size.y - VIEWPORT_MARGIN * 2.0)
	)

func _get_effective_min_panel_size() -> Vector2:
	var limit := _get_viewport_panel_limit()

	return Vector2(
		minf(MIN_PANEL_SIZE.x, limit.x),
		minf(MIN_PANEL_SIZE.y, limit.y)
	)

func _get_effective_max_panel_size() -> Vector2:
	var limit := _get_viewport_panel_limit()

	return Vector2(
		minf(MAX_PANEL_SIZE.x, limit.x),
		minf(MAX_PANEL_SIZE.y, limit.y)
	)
	
func _on_header_gui_input(event: InputEvent) -> void:
	_handle_overlay_pointer_event(event)
	if _resizing:
		return

	if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT:
		var pointer_pos := _gui_event_to_viewport_pos(_header, event)

		if _is_over_header_button(pointer_pos):
			return

		if event.pressed:
			_begin_panel_drag(pointer_pos, -1)
		else:
			_end_panel_drag()

		get_viewport().set_input_as_handled()
		return

	if event is InputEventScreenTouch:
		var pointer_pos := _gui_event_to_viewport_pos(_header, event)

		if _is_over_header_button(pointer_pos):
			return

		if event.pressed:
			_begin_panel_drag(pointer_pos, event.index)
		else:
			if event.index == _drag_touch_index:
				_end_panel_drag()

		get_viewport().set_input_as_handled()
		return

func _is_over_header_button(global_pos: Vector2) -> bool:
	if _pause_button == null:
		return false

	var parent := _pause_button.get_parent()
	if parent == null:
		return false

	for child in parent.get_children():
		if child is Button and child.get_global_rect().has_point(global_pos):
			return true

	return false

func _get_input_block_rect() -> Rect2:
	if _panel == null:
		return Rect2()

	return Rect2(
		_panel.position - Vector2(INPUT_BLOCK_MARGIN, INPUT_BLOCK_MARGIN),
		_panel.size + Vector2(INPUT_BLOCK_MARGIN * 2.0, INPUT_BLOCK_MARGIN * 2.0)
	)

func _update_input_blocker_rect() -> void:
	if _input_blocker == null or _panel == null:
		return

	var rect := _get_input_block_rect()
	_input_blocker.position = rect.position
	_input_blocker.size = rect.size
	
	# Keep resize handles above the input blocker so edge/corner resizing works
	# even when the hit area extends outside the panel.
	for zone in _resize_zones:
		_root.move_child(zone, _root.get_child_count() - 1)

func _on_input_blocker_gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton:
		if event.pressed:
			_begin_local_focus_block(-1)
		else:
			_end_local_focus_block(-1)

		get_viewport().set_input_as_handled()
		return

	if event is InputEventScreenTouch:
		if event.pressed:
			_begin_local_focus_block(event.index)
		else:
			_end_local_focus_block(event.index)

		get_viewport().set_input_as_handled()
		return

	if event is InputEventScreenDrag:
		get_viewport().set_input_as_handled()
		return

func _begin_local_focus_block(touch_index: int) -> void:
	if _focus_block_pointer_active:
		return

	_focus_block_pointer_active = true
	_focus_block_touch_index = touch_index
	_block_explorer_input()

func _end_local_focus_block(touch_index: int) -> void:
	if not _focus_block_pointer_active:
		return

	if _focus_block_touch_index != touch_index:
		return

	_focus_block_pointer_active = false
	_focus_block_touch_index = -1
	_restore_explorer_input_if_safe()

func _handle_overlay_pointer_event(event: InputEvent) -> void:
	if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT:
		if event.pressed:
			_begin_local_focus_block(-1)
		else:
			_end_local_focus_block(-1)
		return

	if event is InputEventScreenTouch:
		if event.pressed:
			_begin_local_focus_block(event.index)
		else:
			_end_local_focus_block(event.index)
		return

	if event is InputEventScreenDrag:
		if not _focus_block_pointer_active:
			_begin_local_focus_block(event.index)

func _on_panel_gui_input(event: InputEvent) -> void:
	_handle_overlay_pointer_event(event)

	if (
		event is InputEventMouseButton
		or event is InputEventMouseMotion
		or event is InputEventScreenTouch
		or event is InputEventScreenDrag
	):
		get_viewport().set_input_as_handled()
