extends Control

signal submit_message(message: String)
signal on_exit_chat
signal on_open_chat
signal release_mouse

## Fixed width for the messages list column (scroll), aligned with `chat.tscn` PanelContainer.
const MESSAGES_COLUMN_WIDTH_PX: int = 709

var hide_tween = null
var open_tween = null
var close_tween = null
var nearby_avatars = null
var is_open: bool = false
var scrolled: bool = false
var new_messages_count: int = 0

@onready var panel_line_edit: PanelContainer = %PanelLineEdit
@onready var h_box_container_line_edit = %HBoxContainer_LineEdit
@onready var line_edit_command = %LineEdit_Command
@onready var margin_container_chat: MarginContainer = %MarginContainer_Chat
@onready var texture_rect_logo: TextureRect = %TextureRect_Logo
@onready var v_box_container_chat: VBoxContainer = %VBoxContainerChat
@onready var scroll_container_chats_list: ScrollContainer = %ScrollContainer_ChatsList
@onready var panel_container_navbar: PanelContainer = %PanelContainer_Navbar
@onready var button_go_to_last: Button = %Button_GoToLast
@onready var panel_container_new_messages: PanelContainer = %PanelContainer_NewMessages
@onready var label_new_messages: Label = %Label_NewMessages
@onready var button_send: Button = %Button_Send
@onready var panel_messages: PanelContainer = $VBoxContainer/HBoxContainer/PanelContainer
@onready var column_go_to_last: Control = $VBoxContainer/HBoxContainer/VSeparator


func _ready():
	if Global.is_mobile():
		# Full chat panel stretches with parent; scroll column keeps fixed width; VSeparator fills remaining X space.
		custom_minimum_size.x = 0
		panel_messages.custom_minimum_size.x = MESSAGES_COLUMN_WIDTH_PX
		panel_messages.size_flags_horizontal = Control.SIZE_SHRINK_BEGIN
		column_go_to_last.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	Global.on_chat_message.connect(self._on_chat_message_arrived)
	Global.change_virtual_keyboard.connect(self._async_on_change_virtual_keyboard)
	submit_message.connect(self._on_submit_message)

	exit_chat.call_deferred()
	button_go_to_last.hide()

	scroll_container_chats_list.get_v_scroll_bar().scrolling.connect(
		self._on_chat_scrollbar_scrolling
	)

	async_show_welcome_message.call_deferred()
	button_send.disabled = true


func async_show_welcome_message() -> void:
	await Global.loading_finished
	Global.on_chat_message.emit(
		"system",
		"[color=#cfc][b]Welcome to Decentraland! Respect others and have fun.[/b][/color]",
		Time.get_unix_time_from_system()
	)


func _on_submit_message(message: String):
	if !message.is_empty():
		var is_command: bool = message.begins_with("/")
		var is_mention: bool = message.contains("@")
		Global.metrics.track_chat_message_sent(
			message.length(), "nearby", false, is_mention, is_command, "", "CHAT"
		)
		UiSounds.play_sound("widget_chat_message_private_send")


func _scroll_to_bottom() -> void:
	if not scroll_container_chats_list:
		return

	new_messages_count = 0
	panel_container_new_messages.hide()
	var scrollbar = scroll_container_chats_list.get_v_scroll_bar()
	if scrollbar:
		var target_scroll: float = max(scrollbar.max_value - scrollbar.page, 0.0)
		scroll_container_chats_list.set_v_scroll(target_scroll)
		scrolled = false
		button_go_to_last.hide()
		_async_scroll_to_bottom_after_layout.call_deferred()


func _async_scroll_to_bottom_after_layout() -> void:
	await get_tree().process_frame
	if not scroll_container_chats_list or not is_instance_valid(scroll_container_chats_list):
		return

	var scrollbar = scroll_container_chats_list.get_v_scroll_bar()
	if not scrollbar or not is_instance_valid(scrollbar):
		return

	var target_scroll: float = max(scrollbar.max_value - scrollbar.page, 0.0)
	scroll_container_chats_list.set_v_scroll(target_scroll)
	scrolled = false
	button_go_to_last.hide()


func _on_button_send_pressed():
	var message = line_edit_command.text
	submit_message.emit(message)
	line_edit_command.text = ""
	button_send.disabled = true

	_scroll_to_bottom()
	# Always close chat if it's a command (starts with "/")
	# or if the configuration requires it
	if message.begins_with("/") or Global.get_config().submit_message_closes_chat:
		exit_chat()


func _on_line_edit_command_text_submitted(new_text):
	submit_message.emit(new_text)
	line_edit_command.text = ""
	button_send.disabled = true
	_scroll_to_bottom()
	# Always close chat if it's a command (starts with "/")
	# or if the configuration requires it
	if new_text.begins_with("/") or Global.get_config().submit_message_closes_chat:
		exit_chat()


func toggle_chat_visibility(visibility: bool):
	if visibility:
		UiSounds.play_sound("widget_chat_open")
	else:
		Global.explorer_grab_focus()
		UiSounds.play_sound("widget_chat_close")


func exit_chat() -> void:
	hide()
	on_exit_chat.emit()
	if Global.is_mobile():
		DisplayServer.virtual_keyboard_hide()


func async_start_chat():
	show()
	Global.get_explorer().release_mouse()
	DisplayServer.virtual_keyboard_show("")
	line_edit_command.text = ""
	button_send.disabled = true
	h_box_container_line_edit.show()
	line_edit_command.grab_focus()
	on_open_chat.emit()
	if !scrolled:
		await get_tree().process_frame
		_scroll_to_bottom()


func _on_chat_message_arrived(address: String, message: String, timestamp: float):
	var new_chat = Global.preload_assets.CHAT_MESSAGE.instantiate()
	v_box_container_chat.add_child(new_chat)
	new_chat.reduce_text = false
	new_chat.max_panel_width = 550
	new_chat.set_chat(address, message, timestamp)

	if !scrolled:
		_scroll_to_bottom()
	else:
		new_messages_count = new_messages_count + 1
		panel_container_new_messages.show()
		label_new_messages.text = str(new_messages_count)


func is_at_bottom() -> bool:
	if not scroll_container_chats_list or not is_instance_valid(scroll_container_chats_list):
		return true  # Consider it "at bottom" if container doesn't exist

	var scrollbar = scroll_container_chats_list.get_v_scroll_bar()
	if not scrollbar or not is_instance_valid(scrollbar):
		return true

	# Works even if the scrollbar is set to "never show".
	var max_scroll: float = max(scrollbar.max_value - scrollbar.page, 0.0)
	if max_scroll <= 0.0:
		return true

	# Check if at bottom with small tolerance
	var tolerance = 5.0
	return scrollbar.value >= max_scroll - tolerance


func _on_chat_scrollbar_scrolling() -> void:
	scrolled = !is_at_bottom()
	button_go_to_last.visible = scrolled


func _on_button_go_to_last_pressed() -> void:
	_scroll_to_bottom()


func _async_on_change_virtual_keyboard(keyboard_height: int) -> void:
	if keyboard_height <= 0:
		return
	await get_tree().process_frame
	_scroll_to_bottom()


func _on_line_edit_command_focus_exited() -> void:
	exit_chat()


func _on_line_edit_command_text_changed(new_text: String) -> void:
	button_send.disabled = new_text.length() == 0
