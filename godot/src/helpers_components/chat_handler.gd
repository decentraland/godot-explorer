extends Node

const EMOTE: String = "␐"
const REQUEST_PING: String = "␑"
const ACK: String = "␆"


func _ready():
	Global.comms.chat_message.connect(self._on_chats_arrived)


func _on_chats_arrived(chats: Array):
	for i in range(chats.size()):
		var chat = chats[i]
		var address: String = chat[0]
		var timestamp: float = chat[1]
		var message: String = chat[2]
		if message.begins_with(EMOTE):
			# Only needed because Bevy releases still send the legacy ␐ emote; drop it so it never
			# shows as chat text. Safe to remove entirely once Bevy stops sending it.
			pass
		elif message.begins_with(REQUEST_PING):
			pass  # TODO: Send ACK
		elif message.begins_with(ACK):
			pass  # TODO: Calculate ping
		else:
			Global.on_chat_message.emit(address, message, timestamp)
