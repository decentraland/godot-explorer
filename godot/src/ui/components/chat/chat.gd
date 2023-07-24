extends Panel
@onready var rich_text_label_chat = $MarginContainer/RichTextLabel_Chat


func _ready():
	add_chat_message(
		"[color=#cfc][b]Welcome to the Godot Client! Navigate to Advanced Settings > Realm tab to change the realm. Press Enter or click in the Talk button to say something to nearby.[/b][/color]"
	)

	Global.comms.chat_message.connect(self._on_chats_arrived)


func add_chat_message(bb_text: String) -> void:
	rich_text_label_chat.append_text(bb_text)
	rich_text_label_chat.newline()


func _on_chats_arrived(chats: Array):
	for chat in chats:
		var text = "[b][color=#1cc]%s[/color] > [color=#fff]%s[/color]" % [chat[0], chat[2]]
		add_chat_message(text)


func _on_button_clear_chat_pressed():
	rich_text_label_chat.clear()
