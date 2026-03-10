extends PanelContainer

@onready var rich_text_label: RichTextLabel = %RichTextLabel
@onready var timer: Timer = %Timer


func _ready():
	timer.timeout.connect(_on_timer_timeout)
	timer.start()
	_on_timer_timeout()


func _status_color(connected: bool) -> String:
	return "green" if connected else "red"


func _on_timer_timeout():
	if not is_instance_valid(Global.comms):
		return

	var info: Dictionary = Global.comms.get_debug_room_info()
	var adapter: String = info.get("adapter", "")
	var main_connected: bool = info.get("main_connected", false)
	var scene_room: String = info.get("scene_room", "")
	var scene_connected: bool = info.get("scene_connected", false)
	var on_hold: bool = info.get("comms_on_hold", false)

	var text := "[b]LiveKit Debug[/b]\n"
	text += "Adapter: " + adapter + "\n"

	var main_label := ""
	if on_hold:
		main_label = "[color=yellow]PAUSED[/color]"
	elif main_connected:
		main_label = "[color=green]CONNECTED[/color]"
	else:
		main_label = "[color=red]DISCONNECTED[/color]"
	text += "Archipelago: " + main_label + "\n"

	var scene_label := ""
	if on_hold:
		scene_label = "[color=yellow]PAUSED[/color]"
	elif scene_connected:
		scene_label = "[color=green]CONNECTED[/color]"
	else:
		scene_label = "[color=red]DISCONNECTED[/color]"
	text += "Scene Room: " + scene_room + " " + scene_label

	rich_text_label.text = text
