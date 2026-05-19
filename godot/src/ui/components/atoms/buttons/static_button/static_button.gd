@abstract class_name StaticButton
extends TextureButton

var _unread_count: int = 0
var _is_panel_open: bool = false

@onready var label_badge: Label = %Label_Badge
@onready var badge_container: PanelContainer = %Badge_Container


func _ready() -> void:
	pressed.connect(_on_pressed)
	toggled.connect(_on_toggled)
	_connect_update_signals()

	# Initial update
	_update_badge()


func _on_pressed() -> void:
	# Track metric: notification menu opened
	var metric_name = _get_button_metric_name()
	if metric_name != "":
		Global.metrics.track_click_button(metric_name, "HUD", "")

	_on_button_clicked()


func _on_toggled(_button_pressed: bool) -> void:
	set_panel_open(_button_pressed)


func set_panel_open(is_open: bool) -> void:
	_is_panel_open = is_open


func _on_notifications_updated(_notifications: Array = []) -> void:
	_update_badge()


func _update_badge() -> void:
	_unread_count = _get_unread_count()

	if _unread_count > 0:
		badge_container.visible = true
		if _unread_count > 99:
			label_badge.text = "99+"
		else:
			label_badge.text = str(_unread_count)
	else:
		badge_container.visible = false


func get_unread_count() -> int:
	return _unread_count


func _get_unread_count() -> int:
	return 0


func _connect_update_signals() -> void:
	pass


func _on_button_clicked() -> void:
	pass


func _get_button_metric_name() -> String:
	return ""
