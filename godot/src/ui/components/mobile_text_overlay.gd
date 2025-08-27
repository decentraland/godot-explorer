class_name MobileTextOverlay
extends ColorRect

## Overlay que aparece sobre el teclado virtual para edición de texto en móvil
## Se sincroniza bidireccionalmente con el control original

signal text_confirmed(text: String)
signal overlay_closed

# Referencias a nodos
@onready var margin_container: MarginContainer = %MarginContainer
@onready var text_input: TextEdit = %TextInput
@onready var line_input: LineEdit = %LineInput
@onready var char_counter: Label = %CharCounter
@onready var confirm_button: Button = %ConfirmButton
@onready var cancel_button: Button = %CancelButton

# Control original que se está editando
var original_control: Control = null
var original_text: String = ""
var is_multiline: bool = false
var max_length: int = -1
var placeholder_text: String = ""
var is_overlay_active: bool = false  # Flag para controlar el estado real del overlay

# Timer para monitoreo del teclado
var keyboard_monitor_timer: Timer
var last_keyboard_height: int = 0

func _ready() -> void:
	# Configurar overlay
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	color = Color(0, 0, 0, 0.5)  # Fondo semi-transparente
	
	# Asegurar que el overlay esté completamente cerrado al inicio
	close_overlay()
	
	# Configurar inputs inicialmente ocultos
	text_input.hide()
	line_input.hide()
	
	# Configurar timer para monitoreo del teclado
	keyboard_monitor_timer = Timer.new()
	keyboard_monitor_timer.wait_time = 0.1
	keyboard_monitor_timer.timeout.connect(_on_keyboard_height_changed)
	add_child(keyboard_monitor_timer)
	
	print("📱 MobileTextOverlay inicializado")

func open_for_control(control: Control) -> void:
	"""Abre el overlay para editar un control específico"""
	if not Global.is_mobile():
		print("⚠️ MobileTextOverlay: Solo funciona en dispositivos móviles")
		return
	
	if not (control is TextEdit or control is LineEdit):
		print("❌ MobileTextOverlay: Solo soporta TextEdit y LineEdit")
		return
	
	print("🚀 Abriendo overlay para: ", control.name, " (", control.get_class(), ")")
	
	# Guardar referencia al control original
	original_control = control
	original_text = control.text
	is_multiline = control is TextEdit
	
	# Configurar propiedades según el control
	_setup_for_control_type(control)
	
	# Mostrar el overlay
	show()
	is_overlay_active = true  # Marcar como activo
	
	# Configurar el input apropiado
	if is_multiline:
		text_input.show()
		line_input.hide()
		text_input.text = original_text
		text_input.placeholder_text = placeholder_text
		text_input.grab_focus()
	else:
		line_input.show()
		text_input.hide()
		line_input.text = original_text
		line_input.placeholder_text = placeholder_text
		line_input.grab_focus()
	
	# Iniciar monitoreo del teclado
	keyboard_monitor_timer.start()
	
	# Actualizar contador de caracteres
	_update_char_counter()
	
	print("✅ Overlay abierto exitosamente")

func _setup_for_control_type(control: Control) -> void:
	"""Configura el overlay según el tipo de control"""
	# Obtener placeholder
	if control.has_method("get") and control.get("placeholder_text"):
		placeholder_text = control.placeholder_text
	else:
		placeholder_text = "Escribe aquí..."
	
	# Obtener límite de caracteres si existe
	max_length = -1
	if control.has_method("get"):
		var max_chars = control.get("max_length")
		if max_chars and max_chars > 0:
			max_length = max_chars
	
	print("📝 Configurado - Placeholder: '", placeholder_text, "', Max length: ", max_length)

func close() -> void:
	"""Cierra el overlay sin guardar cambios"""
	print("❌ Cerrando overlay sin guardar")
	
	# Detener monitoreo
	keyboard_monitor_timer.stop()
	
	# Limpiar referencias
	original_control = null
	original_text = ""
	
	# Ocultar overlay
	hide()
	is_overlay_active = false  # Marcar como inactivo
	
	# Emitir señal
	emit_signal("overlay_closed")

func confirm() -> void:
	"""Confirma los cambios y cierra el overlay"""
	var current_text = get_current_text()
	print("✅ Confirmando texto: '", current_text, "'")
	
	# Aplicar cambios al control original
	if original_control:
		original_control.text = current_text
		
		# Emitir señales del control original si las tiene
		if original_control.has_signal("text_changed"):
			original_control.text_changed.emit()
		if original_control.has_signal("text_submitted"):
			original_control.text_submitted.emit(current_text)
	
	# Emitir nuestra señal
	emit_signal("text_confirmed", current_text)
	
	# Cerrar overlay
	close()

func get_current_text() -> String:
	"""Obtiene el texto actual del input activo"""
	if is_multiline:
		return text_input.text
	else:
		return line_input.text

func is_really_visible() -> bool:
	"""Verifica si el overlay está realmente visible y activo"""
	return visible and is_overlay_active

func force_close() -> void:
	"""Fuerza el cierre completo del overlay"""
	print("🔒 Forzando cierre del overlay")
	hide()
	is_overlay_active = false
	original_control = null
	original_text = ""
	text_input.text = ""
	line_input.text = ""
	text_input.hide()
	line_input.hide()

func _on_text_changed() -> void:
	"""Se ejecuta cuando cambia el texto en cualquier input"""
	var current_text = get_current_text()
	
	# Sincronizar con el control original en tiempo real
	if original_control:
		original_control.text = current_text
	
	# Actualizar contador
	_update_char_counter()
	
	# Validar límite de caracteres
	_validate_text_length()

func _update_char_counter() -> void:
	"""Actualiza el contador de caracteres"""
	var current_length = get_current_text().length()
	
	if max_length > 0:
		char_counter.text = str(current_length) + "/" + str(max_length)
		char_counter.show()
		
		# Cambiar color si se acerca al límite
		if current_length > max_length * 0.9:
			char_counter.modulate = Color.ORANGE
		elif current_length >= max_length:
			char_counter.modulate = Color.RED
		else:
			char_counter.modulate = Color.WHITE
	else:
		char_counter.hide()

func _validate_text_length() -> void:
	"""Valida la longitud del texto y habilita/deshabilita botón confirmar"""
	var current_length = get_current_text().length()
	
	if max_length > 0 and current_length > max_length:
		confirm_button.disabled = true
		confirm_button.text = "Excede límite"
	else:
		confirm_button.disabled = false
		confirm_button.text = "Confirmar"

func _on_keyboard_height_changed() -> void:
	"""Monitorea cambios en la altura del teclado virtual"""
	var current_height = DisplayServer.virtual_keyboard_get_height()
	
	if current_height != last_keyboard_height:
		last_keyboard_height = current_height
		_update_position_for_keyboard(current_height)

func _update_position_for_keyboard(keyboard_height: int) -> void:
	"""Actualiza la posición del overlay según la altura del teclado"""
	var margin_bottom = keyboard_height + 20 if keyboard_height > 0 else 20
	margin_container.add_theme_constant_override("margin_bottom", margin_bottom)
	
	print("⌨️ Teclado altura: ", keyboard_height, "px - Margen: ", margin_bottom, "px")

# Señales de los botones
func _on_confirm_pressed() -> void:
	confirm()

func _on_cancel_pressed() -> void:
	close()

# Señales de los inputs
func _on_text_input_text_changed() -> void:
	_on_text_changed()

func _on_line_input_text_changed() -> void:
	_on_text_changed()

# Manejo de teclas especiales
func _on_text_input_gui_input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed:
		match event.keycode:
			KEY_ENTER:
				if event.ctrl_pressed or event.cmd_pressed:
					confirm()
					get_viewport().set_input_as_handled()
			KEY_ESCAPE:
				close()
				get_viewport().set_input_as_handled()

func _on_line_input_gui_input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed:
		match event.keycode:
			KEY_ENTER:
				confirm()
				get_viewport().set_input_as_handled()
			KEY_ESCAPE:
				close()
				get_viewport().set_input_as_handled()

# Cerrar overlay tocando fuera del área de input
func _on_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch and event.pressed:
		# Solo cerrar si se toca el fondo, no el área de input
		close()

## Funciones de utilidad para testing

func debug_status() -> void:
	"""Imprime el estado actual del overlay para debugging"""
	print("=== MOBILE TEXT OVERLAY DEBUG ===")
	print("Visible (propiedad): ", visible)
	print("Activo (flag): ", is_overlay_active)
	print("Realmente visible: ", is_really_visible())
	print("Control original: ", original_control.name if original_control else "null")
	print("Es multilínea: ", is_multiline)
	print("Texto actual: '", get_current_text(), "'")
	print("Altura teclado: ", last_keyboard_height)
	print("Límite caracteres: ", max_length)
	print("================================")

func close_overlay() -> void:
	hide()
	emit_signal("overlay_closed")
