extends Panel
const EMOTE: String = "␐"
const REQUEST_PING: String = "␑"
const ACK: String = "␆"

@onready var rich_text_label_chat = $MarginContainer/RichTextLabel_Chat


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
		#var _profile_name: StringName = chat[1]
		#var _timestamp: float = chat[2]
		var message: StringName = chat[3]
		var avatar = Global.avatars.get_avatar_by_address(address)

		if message.begins_with(EMOTE):
			message = message.substr(1)  # Remove prefix
			var expression_id = message.split(" ")[0]  # Get expression id ([1] is timestamp)
			avatar.play_emote(expression_id)
		elif message.begins_with(REQUEST_PING):
			pass  # TODO: Send ACK
		elif message.begins_with(ACK):
			pass  # TODO: Calculate ping
		else:
			var text = (
				"[b][color=#1cc]%s[/color] > [color=#fff]%s[/color]" % [avatar.avatar_name, message]
			)
			add_chat_message(text)


func _on_button_clear_chat_pressed():
	rich_text_label_chat.clear()
