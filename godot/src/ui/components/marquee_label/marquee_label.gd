extends ScrollContainer

@export var label_settings: LabelSettings

var is_marquee_active: bool = false
var marquee_tween: Tween
var marquee_speed: float = 60.0
var pause_duration: float = 2

@onready var label: Label = %Label


func _ready() -> void:
	label.label_settings = label_settings


func set_text(text: String) -> void:
	label.text = text


func is_text_overflowing() -> bool:
	return label.size.x > size.x


func start_marquee_effect() -> void:
	if is_marquee_active:
		return

	is_marquee_active = true

	var max_scroll_distance = label.size.x - size.x
	if max_scroll_distance <= 0:
		return

	var scroll_duration = max_scroll_distance / marquee_speed

	if marquee_tween:
		marquee_tween.kill()

	label.position.x = 0

	marquee_tween = create_tween()
	marquee_tween.set_loops()
	marquee_tween.set_trans(Tween.TRANS_LINEAR)
	marquee_tween.set_ease(Tween.EASE_IN_OUT)

	marquee_tween.tween_interval(pause_duration)
	marquee_tween.tween_property(label, "position:x", -max_scroll_distance, scroll_duration)
	marquee_tween.tween_interval(pause_duration)
	marquee_tween.tween_property(label, "position:x", 0, scroll_duration)


func check_and_start_marquee() -> void:
	if is_text_overflowing():
		start_marquee_effect()
	else:
		label.position.x = 0


func stop_marquee_effect() -> void:
	if not is_marquee_active:
		return

	is_marquee_active = false

	if marquee_tween:
		marquee_tween.kill()
		marquee_tween = null

	label.position.x = 0
