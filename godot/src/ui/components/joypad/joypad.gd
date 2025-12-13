extends Control

var combo_opened: bool = false
var _button_combo_touch_start_time: float = 0.0
var _button_combo_touch_id: int = -1
var _button_combo_normal_texture: Texture2D
var _button_combo_pressed_texture: Texture2D

@onready var animation_player: AnimationPlayer = %AnimationPlayer
@onready var button_combo: TouchScreenButton = %Button_Combo

const TOGGLE_MAX_TIME = 0.2  # Tiempo máximo para considerar un toque como toggle (en segundos)


func _ready() -> void:
	# Asegurarse de que el Control padre pueda recibir eventos
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	
	# Guardar las texturas originales del botón
	if button_combo != null:
		_button_combo_normal_texture = button_combo.texture_normal
		_button_combo_pressed_texture = button_combo.texture_pressed


func _input(event: InputEvent) -> void:
	if not Global.is_mobile():
		return
	
	# Detectar cuando se toca el área del botón combo
	if event is InputEventScreenTouch:
		if event.pressed:
			# Verificar si el toque está dentro del área del botón combo
			if button_combo != null and _is_point_inside_button_combo(event.position):
				_button_combo_touch_id = event.index
				_button_combo_touch_start_time = Time.get_ticks_msec() / 1000.0
		else:
			# Cuando se suelta el toque
			if event.index == _button_combo_touch_id:
				var touch_duration = (Time.get_ticks_msec() / 1000.0) - _button_combo_touch_start_time
				# Si fue un toque rápido (menos de TOGGLE_MAX_TIME), hacer toggle
				if touch_duration < TOGGLE_MAX_TIME:
					_toggle_combo()
					get_viewport().set_input_as_handled()
				_button_combo_touch_id = -1


func _is_point_inside_button_combo(point: Vector2) -> bool:
	if button_combo == null:
		return false
	
	# Obtener el tamaño de la textura del botón
	var texture_size = Vector2(80, 80)  # Tamaño por defecto
	if button_combo.texture_normal != null:
		texture_size = button_combo.texture_normal.get_size()
	
	# Aplicar la escala
	var button_size = texture_size * button_combo.scale
	
	# Obtener el rectángulo global del botón
	var button_rect = Rect2(
		button_combo.global_position,
		button_size
	)
	return button_rect.has_point(point)


func _toggle_combo() -> void:
	combo_opened = not combo_opened
	
	if combo_opened:
		animation_player.play("open_combo")
		UiSounds.play_sound("widget_emotes_open")
		# Mostrar textura presionada cuando está abierto
		if button_combo != null and _button_combo_pressed_texture != null:
			button_combo.texture_normal = _button_combo_pressed_texture
	else:
		animation_player.play_backwards("open_combo")
		UiSounds.play_sound("widget_emotes_close")
		# Volver a la textura normal cuando está cerrado
		if button_combo != null and _button_combo_normal_texture != null:
			button_combo.texture_normal = _button_combo_normal_texture


func _on_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch and combo_opened and not event.pressed:
		# Si el combo está abierto y se toca fuera de los botones, cerrarlo
		var touch_pos = event.position
		var should_close = true
		
		# Verificar si el toque está dentro del botón combo principal
		if button_combo != null and _is_point_inside_button_combo(touch_pos):
			should_close = false
		else:
			# Verificar si el toque está dentro de algún botón del combo
			var combo_buttons = [
				button_combo.get_node_or_null("HBoxContainer/Button_Combo1"),
				button_combo.get_node_or_null("HBoxContainer2/Button_Combo2"),
				button_combo.get_node_or_null("HBoxContainer3/Button_Combo3"),
				button_combo.get_node_or_null("HBoxContainer4/Button_Combo4")
			]
			
			for btn in combo_buttons:
				if btn != null and btn.visible:
					var btn_rect = Rect2(
						btn.global_position,
						btn.size
					)
					if btn_rect.has_point(touch_pos):
						should_close = false
						break
		
		if should_close:
			_toggle_combo()
