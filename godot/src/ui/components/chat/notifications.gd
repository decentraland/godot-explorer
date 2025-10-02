extends Control

const MAX_MESSAGES = 10

var notifications: Array[Control] = []

@onready var notifications_container: VBoxContainer = %NotificationsContainer


func _ready() -> void:
	Global.on_chat_message.connect(self._async_on_chat_message_arrived)


func _async_on_chat_message_arrived(address: String, message: String, timestamp: float):
	if notifications.size() == MAX_MESSAGES:
		notifications.pop_front().queue_free()

	var new_chat = Global.preload_assets.CHAT_MESSAGE.instantiate()
	notifications.push_back(new_chat)
	notifications_container.add_child(new_chat)
	new_chat.compact_view = true
	new_chat.max_panel_width = notifications_container.size.x - 50
	new_chat.set_chat(address, message, timestamp)

	await get_tree().create_timer(6.5).timeout

	if is_instance_valid(new_chat):
		new_chat.queue_free()
		notifications.pop_front()
