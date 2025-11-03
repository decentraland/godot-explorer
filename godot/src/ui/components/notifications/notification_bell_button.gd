extends Button

signal bell_clicked

var _unread_count: int = 0
var _is_panel_open: bool = false
var _animation_tween: Tween

@onready var label_badge: Label = %Label_Badge
@onready var badge_container: PanelContainer = %Badge_Container
@onready var bell_sprite: AnimatedSprite2D = %BellAnimatedSprite


func _ready() -> void:
	pressed.connect(_on_pressed)

	# Connect to NotificationsManager signals
	NotificationsManager.new_notifications.connect(_on_notifications_updated)
	NotificationsManager.notifications_updated.connect(_on_notifications_updated)

	# Initial update
	_update_badge()
	_update_button_state()


func _on_pressed() -> void:
	# Haptic feedback on mobile
	if Global.is_mobile():
		Input.vibrate_handheld(20)  # 20ms subtle vibration

	# Track metric: notification menu opened
	Global.metrics.track_click_button("notification_bell", "HUD", "")
	bell_clicked.emit()


func set_panel_open(is_open: bool) -> void:
	_is_panel_open = is_open
	_update_button_state()


func _update_button_state() -> void:
	if bell_sprite == null:
		return

	# Cancel any existing tween
	if _animation_tween:
		_animation_tween.kill()

	if _is_panel_open:
		# Play animation forward with ease-in (stronger curve for faster start)
		bell_sprite.play("toggle")
		_animation_tween = create_tween()
		_animation_tween.set_ease(Tween.EASE_IN)
		_animation_tween.set_trans(Tween.TRANS_QUINT)
		_animation_tween.tween_property(bell_sprite, "speed_scale", 1.0, 0.0)
	else:
		# Play animation backward with ease-out (stronger curve for faster end)
		bell_sprite.play_backwards("toggle")
		_animation_tween = create_tween()
		_animation_tween.set_ease(Tween.EASE_OUT)
		_animation_tween.set_trans(Tween.TRANS_QUINT)
		_animation_tween.tween_property(bell_sprite, "speed_scale", 1.0, 0.0)


func _on_notifications_updated(_notifications: Array = []) -> void:
	_update_badge()


func _update_badge() -> void:
	var notifications = NotificationsManager.get_notifications()
	_unread_count = 0

	for notif in notifications:
		if not notif.get("read", false):
			_unread_count += 1

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
