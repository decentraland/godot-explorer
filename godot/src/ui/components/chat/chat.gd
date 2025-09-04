extends PanelContainer

signal submit_message(message: String)
signal hide_parcel_info
signal show_parcel_info
signal release_mouse

const EMOTE: String = "␐"
const REQUEST_PING: String = "␑"
const ACK: String = "␆"
const CHAT_MESSAGE = preload("res://src/ui/components/chat/chat_message.tscn")

var hide_tween = null
var open_tween = null
var close_tween = null
var nearby_avatars = null
var is_open: bool = false

@onready var h_box_container_line_edit = %HBoxContainer_LineEdit
@onready var line_edit_command = %LineEdit_Command
@onready var button_nearby_users: Button = %Button_NearbyUsers
@onready var label_members_quantity: Label = %Label_MembersQuantity
@onready var margin_container_chat: MarginContainer = %MarginContainer_Chat
@onready var button_back: Button = %Button_Back
@onready var texture_rect_logo: TextureRect = %TextureRect_Logo
@onready var h_box_container_nearby_users: HBoxContainer = %HBoxContainer_NearbyUsers
@onready var timer_hide = %Timer_Hide
@onready var v_box_container_chat: VBoxContainer = %VBoxContainerChat
@onready var scroll_container_chats_list: ScrollContainer = %ScrollContainer_ChatsList
@onready var avatars_list: Control = %AvatarsList
@onready var panel_container_navbar: PanelContainer = %PanelContainer_Navbar
@onready var v_box_container_content: VBoxContainer = %VBoxContainer_Content
@onready var panel_container_notification: PanelContainer = %PanelContainer_Notification
@onready var v_box_container_notifications: VBoxContainer = %VBoxContainerNotifications
@onready var timer_delete_notifications: Timer = %Timer_DeleteNotifications
@onready var chat_message_notification: Control = %ChatMessage_Notification


func _ready():
	_on_button_back_pressed()
	avatars_list.async_update_nearby_users(Global.avatars.get_avatars())

	# Connect to avatar scene changed signal instead of using timer
	Global.avatars.avatar_scene_changed.connect(avatars_list.async_update_nearby_users)
	avatars_list.size_changed.connect(self.update_nearby_quantity)

	Global.comms.chat_message.connect(self.on_chats_arrived)
	submit_message.connect(self._on_submit_message)

	show_notification()

	initialize_notification_instance()
	_async_chat_ready.call_deferred()


func _async_chat_ready():
	await get_tree().process_frame
	await get_tree().process_frame
	await get_tree().process_frame

	if not v_box_container_chat or not scroll_container_chats_list:
		return

	v_box_container_chat.queue_redraw()
	scroll_container_chats_list.queue_redraw()
	await get_tree().process_frame

	var system_message = [
		"system",
		Time.get_unix_time_from_system(),
		"Welcome to the Godot Client! Navigate to Advanced Settings > Realm tab to change the realm. Press Enter or click in the Talk button to say something to nearby."
	]

	var new_chat = CHAT_MESSAGE.instantiate()
	v_box_container_chat.add_child(new_chat)
	new_chat.compact_view = Global.is_chat_compact
	new_chat.set_chat(system_message)

	await get_tree().process_frame
	await get_tree().process_frame
	if new_chat.is_inside_tree() and new_chat.get_parent():
		new_chat.async_adjust_panel_size.call_deferred()


func _on_submit_message(_message: String):
	UiSounds.play_sound("widget_chat_message_private_send")


func on_chats_arrived(chats: Array):
	var should_show_notification = not v_box_container_content.visible

	for i in range(chats.size()):
		var chat = chats[i]
		var is_last_message = i == chats.size() - 1
		async_create_chat(chat, should_show_notification and is_last_message)

	_async_scroll_to_bottom.call_deferred()


func _async_scroll_to_bottom() -> void:
	await get_tree().process_frame
	await get_tree().process_frame

	if scroll_container_chats_list and is_instance_valid(scroll_container_chats_list):
		var scrollbar = scroll_container_chats_list.get_v_scroll_bar()
		if scrollbar and is_instance_valid(scrollbar):
			scroll_container_chats_list.scroll_vertical = scrollbar.max_value


func _on_button_send_pressed():
	submit_message.emit(line_edit_command.text)
	line_edit_command.text = ""


func _on_line_edit_command_text_submitted(new_text):
	submit_message.emit(new_text)
	line_edit_command.text = ""
	line_edit_command.emit_signal("focus_exited")
	grab_focus.call_deferred()


func finish():
	if line_edit_command.text.size() > 0:
		submit_message.emit(line_edit_command.text)
		line_edit_command.text = ""


func toggle_chat_visibility(visibility: bool):
	_on_button_back_pressed()
	if visibility:
		UiSounds.play_sound("widget_chat_open")
		_tween_open()
	else:
		Global.explorer_grab_focus()
		UiSounds.play_sound("widget_chat_close")
		_tween_close()


func _tween_open() -> void:
	if open_tween != null:
		open_tween.stop()
	open_tween = get_tree().create_tween()
	v_box_container_content.show()
	open_tween.tween_property(self, "modulate", Color.WHITE, 0.5)
	is_open = true


func _tween_close() -> void:
	if close_tween != null:
		close_tween.stop()
	close_tween = get_tree().create_tween()
	close_tween.tween_property(self, "modulate", Color.TRANSPARENT, 0.5)
	v_box_container_content.hide()
	is_open = false


func update_nearby_quantity() -> void:
	button_nearby_users.text = str(avatars_list.list_size)
	label_members_quantity.text = str(avatars_list.list_size)


func _on_button_nearby_users_pressed() -> void:
	show_nearby_players()


func _on_button_back_pressed() -> void:
	show_chat()


func _on_line_edit_command_focus_entered() -> void:
	panel_container_navbar.hide()
	emit_signal("hide_parcel_info")
	timer_hide.stop()


func _on_line_edit_command_focus_exited():
	emit_signal("show_parcel_info")
	timer_hide.start()


func _on_timer_hide_timeout() -> void:
	panel_container_navbar.hide()
	h_box_container_line_edit.hide()
	self_modulate = "#00000000"


func _on_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch or event is InputEventMouseButton:
		if margin_container_chat.visible:
			show_chat()
	if event is InputEventKey:
			if event.pressed and event.keycode == KEY_ESCAPE:
				show_notification()
				print("ESC from chat")
					
			if event.pressed and event.keycode == KEY_ENTER:
				print("ENTER")
				toggle_chat_visibility(true)
				show_chat()
				line_edit_command.grab_focus.call_deferred()

func show_chat() -> void:
	v_box_container_content.show()
	panel_container_notification.hide()
	self_modulate = "#00000040"
	avatars_list.hide()
	button_back.hide()
	h_box_container_line_edit.show()
	h_box_container_nearby_users.hide()
	margin_container_chat.show()
	panel_container_navbar.show()
	texture_rect_logo.show()
	button_nearby_users.show()
	timer_hide.start()

	_async_adjust_existing_messages.call_deferred()
	_async_scroll_to_bottom.call_deferred()
	grab_focus()
	Global.get_explorer().release_mouse()


func _async_adjust_existing_messages() -> void:
	await get_tree().process_frame
	await get_tree().process_frame

	for child in v_box_container_chat.get_children():
		if child.has_method("async_adjust_panel_size"):
			child.async_adjust_panel_size.call_deferred()


func show_nearby_players() -> void:
	v_box_container_content.show()
	panel_container_notification.hide()
	self_modulate = "#00000080"
	avatars_list.show()
	button_back.show()
	h_box_container_nearby_users.show()
	margin_container_chat.hide()
	texture_rect_logo.hide()
	button_nearby_users.hide()
	timer_hide.stop()


func show_notification() -> void:
	release_focus.call_deferred()
	var explorer = Global.get_explorer()
	explorer.capture_mouse()
	timer_hide.stop()
	panel_container_notification.modulate = Color.WHITE
	panel_container_notification.show()
	v_box_container_content.hide()
	self_modulate = "#00000000"


func async_create_chat(chat, should_create_notification = false) -> void:
	if not v_box_container_chat or not is_inside_tree():
		async_create_chat.call_deferred(chat, should_create_notification)
		return

	var new_chat = CHAT_MESSAGE.instantiate()
	v_box_container_chat.add_child(new_chat)
	new_chat.compact_view = Global.is_chat_compact
	new_chat.set_chat(chat)

	if should_create_notification:
		async_create_notification(chat)

	await get_tree().process_frame
	await get_tree().process_frame

	if new_chat.is_inside_tree() and v_box_container_content.visible:
		new_chat.async_adjust_panel_size.call_deferred()

	# Always scroll to bottom when a new message is added
	_async_scroll_to_bottom.call_deferred()


func initialize_notification_instance() -> void:
	if chat_message_notification and is_instance_valid(chat_message_notification):
		chat_message_notification.compact_view = true
		chat_message_notification.hide()


func async_create_notification(chat) -> void:
	timer_delete_notifications.stop()
	show_notification()
	await get_tree().process_frame

	if chat_message_notification and is_instance_valid(chat_message_notification):
		chat_message_notification.hide()
		chat_message_notification.set_chat(chat)

		await get_tree().process_frame
		await get_tree().process_frame

		if chat_message_notification.is_inside_tree():
			chat_message_notification.async_adjust_panel_size.call_deferred()
			await get_tree().process_frame
			chat_message_notification.show()
			UiSounds.play_sound("widget_chat_message_private_send")

	timer_delete_notifications.start()


func clear_notifications() -> void:
	if chat_message_notification and is_instance_valid(chat_message_notification):
		chat_message_notification.hide()


func _on_timer_delete_notifications_timeout() -> void:
	var hide_notification_tween = get_tree().create_tween()
	hide_notification_tween.tween_property(
		panel_container_notification, "modulate", Color.TRANSPARENT, 0.5
	)

	hide_notification_tween.tween_callback(
		func():
			clear_notifications()
			panel_container_notification.modulate = Color.WHITE
			panel_container_notification.hide()
	)


		


func _on_line_edit_command_gui_input(event: InputEvent) -> void:
	if event is InputEventKey:
			if event.pressed and event.keycode == KEY_ESCAPE:
				show_notification()
				print("ESC from lineedit")
