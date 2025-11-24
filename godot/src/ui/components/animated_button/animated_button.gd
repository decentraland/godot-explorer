@abstract class_name AnimatedButton
extends Button

@export var sprite_frames_source: SpriteFrames
@export var sprite_frames_scale: Vector2 = Vector2(0.7, 0.7)

var animated_sprite: AnimatedSprite2D
var _unread_count: int = 0
var _is_panel_open: bool = false
var _animation_tween: Tween

@onready var label_badge: Label = %Label_Badge
@onready var badge_container: PanelContainer = %Badge_Container


func _ready() -> void:
	pressed.connect(_on_pressed)
	#toggled.connect(_on_toggled)
	instantiate_animatd_sprite()
	# Las clases hijas deben implementar este método para conectar sus señales
	_connect_update_signals()

	# Initial update
	_update_badge()
	_update_button_state()


func _on_pressed() -> void:
	# Haptic feedback on mobile
	if Global.is_mobile():
		Input.vibrate_handheld(20)  # 20ms subtle vibration
		print("vibra")
	# Track metric: notification menu opened
	var metric_name = _get_button_metric_name()
	if metric_name != "":
		Global.metrics.track_click_button(metric_name, "HUD", "")

	# Las clases hijas deben implementar este método para emitir su señal específica
	_on_button_clicked()


func instantiate_animatd_sprite():
	animated_sprite = AnimatedSprite2D.new()
	add_child(animated_sprite)
	move_child(animated_sprite, 0)
	animated_sprite.sprite_frames = sprite_frames_source
	var sprite_position = Vector2(size.x, size.y) / 2
	print(sprite_position)
	animated_sprite.position = sprite_position
	animated_sprite.scale = sprite_frames_scale


func set_panel_open(is_open: bool) -> void:
	_is_panel_open = is_open
	_update_button_state()


func _update_button_state() -> void:
	if animated_sprite == null:
		return

	# Cancel any existing tween
	if _animation_tween:
		_animation_tween.kill()

	if _is_panel_open:
		# Play animation forward with ease-in (stronger curve for faster start)
		animated_sprite.play("toggle")
		_animation_tween = create_tween()
		_animation_tween.set_ease(Tween.EASE_IN)
		_animation_tween.set_trans(Tween.TRANS_QUINT)
		_animation_tween.tween_property(animated_sprite, "speed_scale", 1.0, 0.0)
	else:
		# Play animation backward with ease-out (stronger curve for faster end)
		animated_sprite.play_backwards("toggle")
		_animation_tween = create_tween()
		_animation_tween.set_ease(Tween.EASE_OUT)
		_animation_tween.set_trans(Tween.TRANS_QUINT)
		_animation_tween.tween_property(animated_sprite, "speed_scale", 1.0, 0.0)


func _on_notifications_updated(_notifications: Array = []) -> void:
	_update_badge()


func _update_badge() -> void:
	# Las clases hijas deben implementar este método para obtener su conteo específico
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
