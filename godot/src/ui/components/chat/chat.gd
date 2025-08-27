extends Panel

signal submit_message(message: String)

const EMOTE: String = "␐"
const REQUEST_PING: String = "␑"
const ACK: String = "␆"
const CHAT_MESSAGE = preload("res://src/ui/components/chat/chat_message.tscn")

var hide_tween = null
var nearby_avatars = null

@onready var h_box_container_line_edit = %HBoxContainer_LineEdit
@onready var line_edit_command = %LineEdit_Command
@onready var button_nearby_users: Button = %Button_NearbyUsers
@onready var label_members_quantity: Label = %Label_MembersQuantity
@onready var margin_container_chat: MarginContainer = %MarginContainer_Chat
@onready var button_back: Button = %Button_Back
@onready var texture_rect_logo: TextureRect = %TextureRect_Logo
@onready var h_box_container_nearby_users: HBoxContainer = %HBoxContainer_NearbyUsers
@onready var timer_hide = %Timer_Hide
@onready var avatars_list: Control = $VBoxContainer/AvatarsList
@onready var v_box_container_chat: VBoxContainer = %VBoxContainerChat


func _ready():
	_on_button_back_pressed()
	avatars_list.async_update_nearby_users(Global.avatars.get_avatars())

	# Connect to avatar scene changed signal instead of using timer
	Global.avatars.avatar_scene_changed.connect(avatars_list.async_update_nearby_users)
	avatars_list.size_changed.connect(self.update_nearby_quantity)

	#add_chat_message(
		#"[color=#cfc][b]Welcome to the Godot Client! Navigate to Advanced Settings > Realm tab to change the realm. Press Enter or click in the Talk button to say something to nearby.[/b][/color]"
	#)

	Global.comms.chat_message.connect(self.on_chats_arrived)

	submit_message.connect(self._on_submit_message)

	h_box_container_line_edit.hide()


func _on_submit_message(_message: String):
	UiSounds.play_sound("widget_chat_message_private_send")
	_set_open_chat(false)


#func add_chat_message(bb_text: String) -> void:
	#rich_text_label_chat.append_text(bb_text)
	#rich_text_label_chat.newline()
#
	#if hide_tween != null:
		#hide_tween.stop()
	#modulate = Color.WHITE
	#timer_hide.start()


func on_chats_arrived(chats: Array):
	for chat in chats:
		print(chat)
		var new_chat = CHAT_MESSAGE.instantiate()
		v_box_container_chat.add_child(new_chat)
		new_chat.set_chat(chat)
		new_chat.compact_view = Global.is_chat_compact
		


func _on_button_send_pressed():
	submit_message.emit(line_edit_command.text)
	line_edit_command.text = ""


func _on_line_edit_command_text_submitted(new_text):
	submit_message.emit(new_text)
	line_edit_command.text = ""


func finish():
	if line_edit_command.text.size() > 0:
		submit_message.emit(line_edit_command.text)
		line_edit_command.text = ""


func _on_line_edit_command_focus_exited():
	_set_open_chat(false)


func toggle_open_chat():
	_on_button_back_pressed()
	_set_open_chat(not h_box_container_line_edit.visible)


func _set_open_chat(value: bool):
	h_box_container_line_edit.visible = value

	if hide_tween != null:
		hide_tween.stop()

	if value:
		line_edit_command.grab_focus()
		UiSounds.play_sound("widget_chat_open")
		timer_hide.stop()
		modulate = Color.WHITE
	else:
		Global.explorer_grab_focus()
		UiSounds.play_sound("widget_chat_close")
		timer_hide.start()
		modulate = Color.WHITE


func _on_timer_hide_timeout():
	if avatars_list.visible:
		return
	if hide_tween != null:
		hide_tween.stop()

	hide_tween = get_tree().create_tween()
	modulate = Color.WHITE
	hide_tween.tween_property(self, "modulate", Color.TRANSPARENT, 0.5)


func update_nearby_quantity() -> void:
	button_nearby_users.text = str(avatars_list.list_size)
	label_members_quantity.text = str(avatars_list.list_size)


func _on_button_nearby_users_pressed() -> void:
	self_modulate = "#00000080"
	avatars_list.show()
	button_back.show()
	h_box_container_nearby_users.show()
	margin_container_chat.hide()
	texture_rect_logo.hide()
	button_nearby_users.hide()
	timer_hide.stop()


func _on_button_back_pressed() -> void:
	self_modulate = "#00000040"
	avatars_list.hide()
	button_back.hide()
	h_box_container_nearby_users.hide()
	margin_container_chat.show()
	texture_rect_logo.show()
	button_nearby_users.show()
	timer_hide.start()
