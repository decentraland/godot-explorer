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

		var avatar: DclAvatar
		if address == Global.player_identity.get_address_str():
			avatar = Global.scene_runner.player_avatar_node
		elif address != "system":
			avatar = Global.avatars.get_avatar_by_address(address)

		var message: String = chat[2]
		if message.begins_with(EMOTE):
			message = message.substr(1)  # Remove prefix
			var expression_id = message.split(" ")[0]  # Get expression id ([1] is timestamp)
			if avatar != null and is_instance_valid(avatar):
				avatar.emote_controller.async_play_emote(expression_id)
		elif message.begins_with(REQUEST_PING):
			pass  # TODO: Send ACK
		elif message.begins_with(ACK):
			pass  # TODO: Calculate ping
		else:
			Global.on_chat_message.emit(address, message, timestamp)
