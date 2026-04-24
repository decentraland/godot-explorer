extends Control

const MAX_MESSAGES = 10

var notifications: Array[Control] = []

@onready var notifications_container: VBoxContainer = %NotificationsContainer


func _ready() -> void:
	Global.on_chat_message.connect(self._async_on_chat_message_arrived)
	notifications_container.resized.connect(self._update_mouse_filter)
	resized.connect(self._update_mouse_filter)


func _update_mouse_filter() -> void:
	# IGNORE when content fits so scene UI (DclUiControl/base_ui) receives touch events through
	# the empty notification area. Switch to STOP only when content overflows and scrolling is needed.
	if notifications_container.size.y > size.y:
		mouse_filter = MOUSE_FILTER_STOP
	else:
		mouse_filter = MOUSE_FILTER_IGNORE


func _async_on_chat_message_arrived(address: String, message: String, timestamp: float):
	if notifications.size() == MAX_MESSAGES:
		notifications.pop_front().queue_free()

	var new_chat = Global.preload_assets.CHAT_MESSAGE.instantiate()
	notifications.push_back(new_chat)
	notifications_container.add_child(new_chat)
	new_chat.reduce_text = true
	new_chat.set_chat(address, message, timestamp)

	await get_tree().create_timer(6.5).timeout

	if is_instance_valid(new_chat):
		new_chat.queue_free()
		notifications.pop_front()
