extends PanelContainer

signal toast_clicked(notification: Dictionary)
signal toast_closed

const DISPLAY_DURATION = 5.0
const SLIDE_IN_DURATION = 0.3
const SLIDE_OUT_DURATION = 0.2

var notification_data: Dictionary = {}
var _timer: Timer

@onready var notification_content: Control = %NotificationContent


func _ready() -> void:
	gui_input.connect(_on_gui_input)

	_timer = Timer.new()
	_timer.one_shot = true
	_timer.timeout.connect(_on_timer_timeout)
	add_child(_timer)

	# Start above screen
	position.y = -size.y


func show_notification(notification: Dictionary) -> void:
	notification_data = notification
	notification_content.set_notification(notification)

	# Animate slide in from top
	var tween = create_tween()
	tween.set_ease(Tween.EASE_OUT)
	tween.set_trans(Tween.TRANS_BACK)
	tween.tween_property(self, "position:y", 20.0, SLIDE_IN_DURATION)

	# Start auto-hide timer
	_timer.start(DISPLAY_DURATION)


func _on_timer_timeout() -> void:
	async_hide_toast()


func async_hide_toast() -> void:
	# Animate slide out to top
	var tween = create_tween()
	tween.set_ease(Tween.EASE_IN)
	tween.set_trans(Tween.TRANS_BACK)
	tween.tween_property(self, "position:y", -size.y - 20.0, SLIDE_OUT_DURATION)
	await tween.finished
	toast_closed.emit()  # Emit signal before freeing
	queue_free()


func _on_gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton:
		if event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
			_track_notification_opened()
			toast_clicked.emit(notification_data)
			Global.notification_clicked.emit(notification_data)
			async_hide_toast()
	elif event is InputEventScreenTouch:
		if event.pressed:
			_track_notification_opened()
			toast_clicked.emit(notification_data)
			Global.notification_clicked.emit(notification_data)
			async_hide_toast()


func _track_notification_opened() -> void:
	# Track metric: notification opened from HUD toast
	var extra_properties = JSON.stringify({
		"notification_id": notification_data.get("id", ""),
		"ui_source": "HUD"
	})
	Global.metrics.track_click_button("notification_opened", "HUD", extra_properties)
