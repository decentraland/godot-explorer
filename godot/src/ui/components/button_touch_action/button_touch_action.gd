extends Button

@export var trigger_action = "ia_primary"

var _touch_index: int = -1
var _is_action_active: bool = false  # Rastrea si realmente estamos enviando la acción


func _ready() -> void:
	# Deshabilitar toggle_mode para comportamiento de botón normal
	toggle_mode = false


func _input(event: InputEvent) -> void:
	# Usar _input() para capturar eventos ANTES de que otros nodos los procesen
	# Actualizamos el estado visual inmediatamente aquí para que funcione
	# incluso cuando el joystick marca eventos como handled
	if not Global.is_mobile():
		return

	if event is InputEventScreenTouch:
		var touch_pos = event.position
		var is_inside = _is_point_inside_button(touch_pos)

		if event.pressed:
			if is_inside and _touch_index == -1:
				# Nuevo toque dentro del botón
				_touch_index = event.index
				_is_action_active = true

				# Actualizar estado visual INMEDIATAMENTE
				set_pressed_no_signal(true)

				# Disparar la acción de input
				Input.action_press(trigger_action)

				# NO marcamos como handled aquí para permitir que gui_input() también se ejecute
				# si el evento no fue handled por otro sistema (como el joystick)
			elif not is_inside:
				# Toque fuera del botón - no hacer nada
				pass
			else:
				# Toque dentro pero ya tenemos un toque activo - ignorar
				pass
		else:
			# Toque liberado
			if event.index == _touch_index:
				# Es nuestro toque
				if _is_action_active:
					# Liberar la acción solo si estaba activa
					Input.action_release(trigger_action)
					_is_action_active = false

				# Actualizar estado visual INMEDIATAMENTE
				set_pressed_no_signal(false)
				_touch_index = -1
	elif event is InputEventScreenDrag:
		# Manejar arrastre para detectar cuando sales/entras del botón
		if _touch_index == event.index:
			var touch_pos = event.position
			var is_inside = _is_point_inside_button(touch_pos)

			if is_inside and not _is_action_active:
				# Re-entrada al botón sin levantar el dedo - NO activar acción
				# NO mostrar visualmente como pressed porque no estamos enviando acción
				pass
			elif not is_inside and _is_action_active:
				# Salida del botón - liberar acción y quitar visual
				Input.action_release(trigger_action)
				_is_action_active = false
				set_pressed_no_signal(false)


func _on_gui_input(event: InputEvent) -> void:
	# gui_input() se ejecuta después de _input() y solo si el evento no fue handled
	# Aquí manejamos el feedback visual cuando el evento llega normalmente
	if event is InputEventScreenTouch:
		var touch_pos = event.position
		var is_inside = get_rect().has_point(touch_pos)

		if event.pressed and is_inside:
			# Si llegamos aquí, el evento no fue handled por otro sistema
			# Actualizar el estado visual inmediatamente
			if _touch_index == -1:
				_touch_index = event.index
				_is_action_active = true
				set_pressed_no_signal(true)
				Input.action_press(trigger_action)
			accept_event()
		elif not event.pressed and event.index == _touch_index:
			# Toque liberado
			if _is_action_active:
				Input.action_release(trigger_action)
				_is_action_active = false
			set_pressed_no_signal(false)
			_touch_index = -1
			accept_event()
	elif event is InputEventScreenDrag:
		if _touch_index == event.index:
			var touch_pos = event.position
			var is_inside = get_rect().has_point(touch_pos)

			if is_inside and not _is_action_active:
				# Re-entrada - NO mostrar visual ni activar acción
				pass
			elif not is_inside and _is_action_active:
				# Salida - liberar acción
				Input.action_release(trigger_action)
				_is_action_active = false
				set_pressed_no_signal(false)


func _is_point_inside_button(point: Vector2) -> bool:
	# Obtener el rectángulo global del botón
	var global_rect = Rect2(global_position, size * get_global_transform_with_canvas().get_scale())
	return global_rect.has_point(point)
