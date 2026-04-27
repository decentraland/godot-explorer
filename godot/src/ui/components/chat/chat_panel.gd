extends MarginContainer

signal submit_message(message: String)
signal share_place
signal load_scenes_pressed

enum ChatState { CLOSED, OPEN, WRITING }

const LANDSCAPE_MARGINS := {"left": 0, "top": 0, "right": 0, "bottom": 0}
const PORTRAIT_MARGINS := {"left": 40, "top": 0, "right": 40, "bottom": 64}
const WIDTH_OPEN := 420

var _current_state: ChatState = ChatState.CLOSED

@onready var chat: Control = %Chat
@onready var notifications: Control = %Notifications
@onready var virtual_keyboard_margin: Control = %VirtualKeyboardMargin
@onready var safe_bottom_area: PanelContainer = %Control_SafeBottomArea
@onready var chatbar: Control = %Chatbar


func _ready() -> void:
	Global.open_chat.connect(_on_global_open_chat)
	Global.close_chat.connect(_on_global_close_chat)
	Global.change_virtual_keyboard.connect(_on_change_virtual_keyboard)
	Global.orientation_changed.connect(_on_orientation_changed)
	chatbar.share_place.connect(func(): share_place.emit())
	chatbar.load_scenes_pressed.connect(func(): load_scenes_pressed.emit())
	_apply_closed_state()


## State 1: Chat closed — only chatbar, notifications, safe area visible
func _apply_closed_state() -> void:
	_current_state = ChatState.CLOSED
	chat.hide()
	chatbar.show()
	notifications.show()
	_apply_default_margins()
	if not Global.is_orientation_portrait():
		safe_bottom_area.show()
	virtual_keyboard_margin.custom_minimum_size.y = 0


## State 2: Chat open — chat visible, notifications hidden, chatbar visible
func _apply_open_state() -> void:
	_current_state = ChatState.OPEN
	chat.show()
	chatbar.show()
	notifications.hide()
	_apply_default_margins()
	if not Global.is_orientation_portrait():
		safe_bottom_area.show()
	_update_chat_layout()


## State 3: Writing — safe area hidden, chatbar visible in portrait
func _apply_writing_state() -> void:
	_current_state = ChatState.WRITING
	notifications.hide()
	safe_bottom_area.hide()
	_apply_default_margins()
	if Global.is_orientation_portrait():
		add_theme_constant_override("margin_bottom", 5)
	else:
		chatbar.hide()
	_update_chat_layout()


func _apply_default_margins() -> void:
	var m := PORTRAIT_MARGINS if Global.is_orientation_portrait() else LANDSCAPE_MARGINS
	add_theme_constant_override("margin_left", m["left"])
	add_theme_constant_override("margin_top", m["top"])
	add_theme_constant_override("margin_right", m["right"])
	add_theme_constant_override("margin_bottom", m["bottom"])


func _update_chat_layout() -> void:
	if Global.is_orientation_portrait():
		chat.set_layout_portrait()
		return

	match _current_state:
		ChatState.OPEN:
			chat.set_layout_reading(WIDTH_OPEN)
		ChatState.WRITING:
			chat.set_layout_writing()


func _on_global_open_chat() -> void:
	chat.async_start_chat()


func _on_global_close_chat() -> void:
	chat.exit_chat()


func _on_panel_chat_on_open_chat() -> void:
	_apply_open_state()


func _on_panel_chat_on_exit_chat() -> void:
	_apply_closed_state()
	if Global.is_orientation_portrait():
		Global.set_orientation_landscape()


func _on_chat_enter_write_mode() -> void:
	_apply_writing_state()
	Global.chat_write_mode_changed.emit(true)


func _on_chat_exit_write_mode() -> void:
	_apply_open_state()
	Global.chat_write_mode_changed.emit(false)


func _on_panel_chat_submit_message(message: String) -> void:
	submit_message.emit(message)


func _on_change_virtual_keyboard(virtual_keyboard_height: int) -> void:
	var window_size: Vector2i = DisplayServer.window_get_size()
	var viewport_size: Vector2 = get_viewport().get_visible_rect().size
	var safe_window_height: float = max(float(window_size.y), 1.0)
	var y_factor: float = viewport_size.y / safe_window_height
	var adjusted_height: int = virtual_keyboard_height
	# iOS keyboard height includes the bottom safe area, but SafeMarginContainerHUD
	# already accounts for it — subtract to avoid double-counting
	if OS.get_name() == "iOS":
		var safe_area := Global.get_safe_area()
		var bottom_inset: int = window_size.y - safe_area.position.y - safe_area.size.y
		adjusted_height = max(virtual_keyboard_height - bottom_inset, 0)
	var keyboard_height_scaled: float = ceil(max(float(adjusted_height) * y_factor, 0.0))
	virtual_keyboard_margin.custom_minimum_size.y = keyboard_height_scaled

	if virtual_keyboard_height <= 0:
		chat.close_write_mode_if_active()
		chat.reset_safe_area_insets()
		return
	chat.async_apply_system_bar_insets()
	chat.scroll_to_bottom_deferred()


func show_load_scenes_button() -> void:
	chatbar.show_load_scenes_button()


func hide_load_scenes_button() -> void:
	chatbar.hide_load_scenes_button()


func is_chat_visible() -> bool:
	return chat.visible


func is_interactive_area_at(position: Vector2) -> bool:
	if chatbar.visible and chatbar.is_point_inside(position):
		return true
	if chat.visible and chat.is_interactive_area_at(position):
		return true
	return false


func _on_orientation_changed(is_portrait: bool) -> void:
	# Delegate layout/font updates and write-mode close to chat first.
	# If chat was in write mode, _close_write_mode emits on_exit_write_mode
	# which synchronously transitions us to OPEN before we continue here.
	chat.apply_orientation(is_portrait)
	# Re-apply the full current state so visibility (chatbar, notifications,
	# safe_bottom_area) is correct for the new orientation.
	match _current_state:
		ChatState.CLOSED:
			_apply_closed_state()
		ChatState.OPEN:
			_apply_open_state()
		ChatState.WRITING:
			_apply_writing_state()
