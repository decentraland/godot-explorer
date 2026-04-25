extends MarginContainer

signal submit_message(message: String)
signal share_place
signal load_scenes_pressed

enum ChatState { CLOSED, OPEN, WRITING }

const LANDSCAPE_MARGINS := {"left": 0, "top": 24, "right": 0, "bottom": 0}
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
	if not Global.is_orientation_portrait():
		safe_bottom_area.show()
	virtual_keyboard_margin.custom_minimum_size.y = 0


## State 2: Chat open — chat visible, notifications hidden, chatbar visible
func _apply_open_state() -> void:
	_current_state = ChatState.OPEN
	chat.show()
	chatbar.show()
	notifications.hide()
	add_theme_constant_override("margin_top", LANDSCAPE_MARGINS["top"])
	if Global.is_orientation_portrait():
		add_theme_constant_override("margin_bottom", PORTRAIT_MARGINS["bottom"])
	else:
		safe_bottom_area.show()
	_update_chat_layout()


## State 3: Writing — safe area hidden, chatbar visible in portrait
func _apply_writing_state() -> void:
	_current_state = ChatState.WRITING
	notifications.hide()
	safe_bottom_area.hide()
	if Global.is_orientation_portrait():
		add_theme_constant_override("margin_bottom", 5)
	else:
		chatbar.hide()
		add_theme_constant_override("margin_top", 0)
	_update_chat_layout()


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
	# Hide during transition to avoid layout flicker
	chat.modulate.a = 0
	_apply_open_state()
	Global.chat_write_mode_changed.emit(false)
	await get_tree().process_frame
	chat.modulate.a = 1


func _on_panel_chat_submit_message(message: String) -> void:
	submit_message.emit(message)


func _on_change_virtual_keyboard(virtual_keyboard_height: int) -> void:
	var window_size: Vector2i = DisplayServer.window_get_size()
	var viewport_size: Vector2 = get_viewport().get_visible_rect().size
	var safe_window_height: float = max(float(window_size.y), 1.0)
	var y_factor: float = viewport_size.y / safe_window_height
	var keyboard_height_scaled: float = ceil(max(float(virtual_keyboard_height) * y_factor, 0.0))
	virtual_keyboard_margin.custom_minimum_size.y = keyboard_height_scaled


func show_load_scenes_button() -> void:
	chatbar.show_load_scenes_button()


func hide_load_scenes_button() -> void:
	chatbar.hide_load_scenes_button()


func is_chat_visible() -> bool:
	return chat.visible


func is_interactive_area_at(position: Vector2) -> bool:
	if chatbar.visible and chatbar.get_global_rect().has_point(position):
		return true
	if chat.visible and chat.get_global_rect().has_point(position):
		return true
	return false


func _on_orientation_changed(is_portrait: bool) -> void:
	var m := PORTRAIT_MARGINS if is_portrait else LANDSCAPE_MARGINS
	add_theme_constant_override("margin_left", m["left"])
	add_theme_constant_override("margin_top", m["top"])
	add_theme_constant_override("margin_right", m["right"])
	add_theme_constant_override("margin_bottom", m["bottom"])
	safe_bottom_area.visible = not is_portrait
	_update_chat_layout()
