extends PanelContainer

@onready var rich_text_label: RichTextLabel = %RichTextLabel
@onready var timer: Timer = %Timer


func _ready():
	timer.timeout.connect(_on_timer_timeout)
	timer.start()
	_on_timer_timeout()


func _on_timer_timeout():
	if not is_instance_valid(Global.comms):
		return

	var info: Dictionary = Global.comms.get_debug_room_info()
	var adapter: String = info.get("adapter", "")
	var scene_room: String = info.get("scene_room", "")
	var scene_connected: bool = info.get("scene_connected", false)

	var text := "[b]LiveKit Debug[/b]\n"
	text += "Adapter: " + adapter + "\n"
	text += "Scene Room: " + scene_room + "\n"
	text += "Scene Connected: " + str(scene_connected)

	rich_text_label.text = text
