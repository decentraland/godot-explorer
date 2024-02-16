extends Panel

signal submit_message(message: String)

const EMOTE: String = "␐"
const REQUEST_PING: String = "␑"
const ACK: String = "␆"

@onready var rich_text_label_chat = $MarginContainer/VBoxContainer/RichTextLabel_Chat
@onready
var line_edit_command = $MarginContainer/VBoxContainer/HBoxContainer_LineEdit/LineEdit_Command


func _ready():
	add_chat_message(
		"[color=#cfc][b]Welcome to the Godot Client! Navigate to Advanced Settings > Realm tab to change the realm. Press Enter or click in the Talk button to say something to nearby.[/b][/color]"
	)

	Global.comms.chat_message.connect(self.on_chats_arrived)


func add_chat_message(bb_text: String) -> void:
	rich_text_label_chat.append_text(bb_text)
	rich_text_label_chat.newline()


func on_chats_arrived(chats: Array):
	for chat in chats:
		var address: String = chat[0]
		# var _timestamp: float = chat[1]
		var message: String = chat[2]

		var avatar = Global.avatars.get_avatar_by_address(address)
		if avatar == null:
			if address == Global.player_identity.get_address_str():
				avatar = Global.scene_runner.player_node.avatar

		var avatar_name: String = ""
		if avatar != null:
			avatar_name = avatar.get_avatar_name()

		if avatar_name.is_empty():
			if address.length() > 32:
				avatar_name = DclEther.shorten_eth_address(address)
			else:
				avatar_name = "Unknown"

		if message.begins_with(EMOTE):
			message = message.substr(1)  # Remove prefix
			var expression_id = message.split(" ")[0]  # Get expression id ([1] is timestamp)
			avatar.emote_controller.play_emote(expression_id)
		elif message.begins_with(REQUEST_PING):
			pass  # TODO: Send ACK
		elif message.begins_with(ACK):
			pass  # TODO: Calculate ping
		else:
			var text = "[b][color=#1cc]%s[/color] > [color=#fff]%s[/color]" % [avatar_name, message]
			add_chat_message(text)


func _on_button_send_pressed():
	submit_message.emit(line_edit_command.text)
	line_edit_command.text = ""
	line_edit_command.grab_focus()


func _on_line_edit_command_text_submitted(new_text):
	submit_message.emit(new_text)
	line_edit_command.text = ""


func finish():
	if line_edit_command.text.size() > 0:
		submit_message.emit(line_edit_command.text)
		line_edit_command.text = ""


func _on_visibility_changed():
	if is_instance_valid(line_edit_command):
		line_edit_command.text = ""
		if visible:
			line_edit_command.grab_focus()
		else:
			Global.explorer_grab_focus()


func _on_line_edit_command_focus_exited():
	self.hide()
