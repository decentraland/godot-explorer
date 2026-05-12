extends Control

const DISPLAY_DURATION = 5.0
const SLIDE_IN_DURATION = 0.3
const SLIDE_OUT_DURATION = 0.2

var _timer: Timer

@onready var panel: PanelContainer = $Panel


func _ready() -> void:
	_timer = Timer.new()
	_timer.one_shot = true
	_timer.timeout.connect(_on_timer_timeout)
	add_child(_timer)


func async_show() -> void:
	await get_tree().process_frame
	position.y = -size.y

	var tween = create_tween()
	tween.set_ease(Tween.EASE_OUT)
	tween.set_trans(Tween.TRANS_BACK)
	tween.tween_property(self, "position:y", 85.0, SLIDE_IN_DURATION)
	await tween.finished

	_timer.start(DISPLAY_DURATION)


func _on_timer_timeout() -> void:
	async_hide_toast()


func async_hide_toast() -> void:
	var tween = create_tween()
	tween.set_ease(Tween.EASE_IN)
	tween.set_trans(Tween.TRANS_BACK)
	tween.tween_property(self, "position:y", -size.y - 20.0, SLIDE_OUT_DURATION)
	await tween.finished
	queue_free()
