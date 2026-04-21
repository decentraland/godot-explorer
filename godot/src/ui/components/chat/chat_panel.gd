extends MarginContainer

signal submit_message(message: String)
signal share_place
signal load_scenes_pressed

@onready var chat: Control = %Chat
@onready var notifications: Control = %Notifications
@onready var virtual_keyboard_margin: Control = %VirtualKeyboardMargin
@onready var button_load_scenes: Button = %Button_LoadScenes
@onready var chatbar: Control = %Chatbar


func _ready() -> void:
	Global.open_chat.connect(_on_global_open_chat)
	Global.close_chat.connect(_on_global_close_chat)
	Global.change_virtual_keyboard.connect(_on_change_virtual_keyboard)
	chatbar.share_place.connect(func(): share_place.emit())


func _on_global_open_chat() -> void:
	notifications.hide()
	chat.async_start_chat()


func _on_global_close_chat() -> void:
	chat.exit_chat()
	notifications.show()


func _on_panel_chat_on_open_chat() -> void:
	notifications.hide()


func _on_panel_chat_on_exit_chat() -> void:
	notifications.show()


func _on_panel_chat_submit_message(message: String) -> void:
	submit_message.emit(message)


func _on_change_virtual_keyboard(virtual_keyboard_height: int) -> void:
	var window_size: Vector2i = DisplayServer.window_get_size()
	var viewport_size: Vector2 = get_viewport().get_visible_rect().size
	var safe_window_height: float = max(float(window_size.y), 1.0)
	var y_factor: float = viewport_size.y / safe_window_height
	var keyboard_height_scaled: float = ceil(max(float(virtual_keyboard_height) * y_factor, 0.0))
	virtual_keyboard_margin.custom_minimum_size.y = keyboard_height_scaled


func _on_button_load_scenes_pressed() -> void:
	load_scenes_pressed.emit()


func show_load_scenes_button() -> void:
	button_load_scenes.show()


func hide_load_scenes_button() -> void:
	button_load_scenes.hide()


func is_chat_visible() -> bool:
	return chat.visible
