extends Control

var _tween: Tween = null
var _tween_equiped: Tween = null

@onready var label: Label = $EmoteName
@onready var equiped_container: MarginContainer = $EquippedNotification


func _ready() -> void:
	label.position.y = 56.0
	equiped_container.position.y = 60.0


func set_equipped(equipped: bool) -> void:
	if _tween_equiped != null and _tween_equiped.is_running():
		_tween_equiped.kill()
	_tween_equiped = create_tween()
	if equipped:
		label.hide()
		_tween_equiped.tween_property(equiped_container, "position:y", 0.0, 0.3).from(60.0)
		_tween_equiped.tween_interval(0.5)
		_tween_equiped.tween_property(equiped_container, "position:y", 60.0, 0.3)
		_tween_equiped.tween_callback(label.show)
	else:
		_tween_equiped.tween_property(equiped_container, "position:y", 60.0, 0.3)


func set_emote_name(emote_name: String) -> void:
	label.text = emote_name
	label.show()
	if _tween != null and _tween.is_running():
		_tween.kill()
	_tween = create_tween()
	_tween.tween_property(label, "position:y", 0.0, 0.3).from(60.0)
