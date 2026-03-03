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
	var scene_on_hold: bool = info.get("scene_room_on_hold", false)

	var text := "[b]LiveKit Debug[/b]\n"
	text += "Adapter: " + adapter + "\n"

	var main_label := "CONNECTED" if main_connected else "DISCONNECTED"
	text += ("Archipelago: [color=%s]%s[/color]\n" % [_status_color(main_connected), main_label])

	var scene_label := ""
	if scene_on_hold:
		scene_label = "[color=yellow]PAUSED[/color]"
	elif scene_connected:
		scene_label = "[color=green]CONNECTED[/color]"
	else:
		scene_label = "[color=red]DISCONNECTED[/color]"
	text += "Scene Room: " + scene_room + " " + scene_label

	rich_text_label.text = text
