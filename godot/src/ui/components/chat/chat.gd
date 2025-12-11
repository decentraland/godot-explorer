extends PanelContainer

signal submit_message(message: String)
signal on_exit_chat
signal on_open_chat
signal release_mouse

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
@onready var margin_container_go_to_new_messages: MarginContainer = %MarginContainer_GoToNewMessages
@onready var button_go_to_last: Button = %Button_GoToLast
@onready var panel_container_new_messages: PanelContainer = %PanelContainer_NewMessages
@onready var label_new_messages: Label = %Label_NewMessages


# gdlint:ignore = async-function-name
func _ready():
	Global.on_chat_message.connect(self._on_chat_message_arrived)
	Global.change_virtual_keyboard.connect(self._async_on_change_virtual_keyboard)
	submit_message.connect(self._on_submit_message)

	exit_chat.call_deferred()
	button_go_to_last.hide()

	scroll_container_chats_list.get_v_scroll_bar().scrolling.connect(
		self._on_chat_scrollbar_scrolling
	)

	await Global.loading_finished
	Global.on_chat_message.emit(
		"system",
		"[color=#cfc][b]Welcome to Decentraland! Respect others and have fun.[/b]",
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
	var scrollbar = scroll_container_chats_list.get_v_scroll_bar()
	if scrollbar:
		scroll_container_chats_list.set_v_scroll.call_deferred(scrollbar.max_value - scrollbar.page)
		scrolled = false
		button_go_to_last.hide()


func _on_button_send_pressed():
	submit_message.emit(line_edit_command.text)
	line_edit_command.text = ""
	exit_chat()
	DisplayServer.virtual_keyboard_hide()


func _on_line_edit_command_text_submitted(new_text):
	submit_message.emit(new_text)
	line_edit_command.text = ""
	line_edit_command.focus_exited.emit()
	grab_focus.call_deferred()
	exit_chat()


func finish():
	if line_edit_command.text.size() > 0:
		submit_message.emit(line_edit_command.text)
		line_edit_command.text = ""


func toggle_chat_visibility(visibility: bool):
	if visibility:
		UiSounds.play_sound("widget_chat_open")
	else:
		Global.explorer_grab_focus()
		UiSounds.play_sound("widget_chat_close")


func _on_gui_input(event: InputEvent) -> void:
	if event is InputEventKey:
		if event.pressed and event.keycode == KEY_ESCAPE:
			exit_chat()
		if event.pressed and event.keycode == KEY_ENTER:
			toggle_chat_visibility(true)
			async_start_chat()
			line_edit_command.grab_focus.call_deferred()


func exit_chat() -> void:
	hide()
	on_exit_chat.emit()
	DisplayServer.virtual_keyboard_hide()


func async_start_chat():
	show()
	panel_container_navbar.show()

	Global.get_explorer().release_mouse()
	DisplayServer.virtual_keyboard_show("")
	h_box_container_line_edit.show()
	line_edit_command.grab_focus()
	on_open_chat.emit()
	if !scrolled:
		await get_tree().process_frame
		_scroll_to_bottom()


func _on_chat_message_arrived(address: String, message: String, timestamp: float):
	var new_chat = Global.preload_assets.CHAT_MESSAGE.instantiate()
	v_box_container_chat.add_child(new_chat)
	new_chat.compact_view = true
	new_chat.reduce_text = false
	new_chat.max_panel_width = 550
	new_chat.set_chat(address, message, timestamp)

	if !scrolled:
		_scroll_to_bottom()
	else:
		new_messages_count = new_messages_count + 1
		if new_messages_count == 0:
			panel_container_new_messages.hide()
		else:
			panel_container_new_messages.show()
			label_new_messages.text = str(new_messages_count)


func _on_line_edit_command_gui_input(event: InputEvent) -> void:
	if event is InputEventKey:
		if event.pressed and event.keycode == KEY_ESCAPE:
			exit_chat()


func is_at_bottom() -> bool:
	if not scroll_container_chats_list or not is_instance_valid(scroll_container_chats_list):
		return true  # Consider it "at bottom" if container doesn't exist

	var scrollbar = scroll_container_chats_list.get_v_scroll_bar()
	if not scrollbar or not is_instance_valid(scrollbar) or not scrollbar.visible:
		return true  # No scrollbar means all content visible, so we're "at bottom"

	# Check if at bottom with small tolerance
	var tolerance = 5.0
	return scrollbar.value + scrollbar.page >= scrollbar.max_value - tolerance


func _on_chat_scrollbar_scrolling() -> void:
	scrolled = !is_at_bottom()
	button_go_to_last.visible = scrolled


func _on_button_go_to_last_pressed() -> void:
	_scroll_to_bottom()


func _async_on_change_virtual_keyboard(_new_safe_area) -> void:
	await get_tree().process_frame
	_scroll_to_bottom()


func _on_line_edit_command_focus_exited() -> void:
	exit_chat()
