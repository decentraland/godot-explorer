class_name MobileInputManager
extends Node

## Manager que detecta cuando TextEdit/LineEdit reciben focus en móvil
## y abre automáticamente el overlay para una mejor experiencia de usuario

signal overlay_opened(control: Control)
signal overlay_closed(control: Control)
signal text_confirmed(text: String, control: Control)

@onready var mobile_overlay: MobileTextOverlay = %MobileTextOverlay

# Lista de controles conectados
var connected_controls: Array[Control] = []
var current_editing_control: Control = null

func _ready() -> void:
	# Conectar señales del overlay
	if mobile_overlay:
		mobile_overlay.text_confirmed.connect(_on_overlay_text_confirmed)
		mobile_overlay.overlay_closed.connect(_on_overlay_closed)
		print("📱 MobileInputManager inicializado correctamente")
	else:
		print("❌ MobileInputManager: No se encontró MobileTextOverlay")

## Conecta automáticamente todos los TextEdit y LineEdit de una escena
func auto_connect_scene_controls(scene_root: Node) -> void:
	"""
	Escanea recursivamente una escena y conecta todos los controles de texto
	Args:
		scene_root: Nodo raíz de la escena a escanear
	"""
	if not Global.is_mobile():
		print("⚠️ MobileInputManager: Auto-conexión deshabilitada (no es móvil)")
		return
	
	print("🔍 Escaneando controles en: ", scene_root.name)
	var controls_found = 0
	_recursive_connect_controls(scene_root, controls_found)
	print("✅ Auto-conexión completada. Controles encontrados: ", controls_found)

func _recursive_connect_controls(node: Node, controls_found: int) -> int:
	"""Conecta recursivamente todos los controles de texto encontrados"""
	
	# Conectar si es un control de texto directo
	if node is TextEdit or node is LineEdit:
		connect_control(node)
		controls_found += 1
		print("🔗 Conectado: ", node.name, " (", node.get_class(), ")")
	
	# Buscar en componentes que contengan TextEdit/LineEdit (como dcl_text_edit)
	elif _has_text_editing_methods(node):
		var text_control = _find_text_control_in_component(node)
		if text_control:
			connect_control(text_control)
			controls_found += 1
			print("🔗 Conectado componente: ", node.name, " -> ", text_control.name)
	
	# Continuar recursivamente con los hijos
	for child in node.get_children():
		controls_found = _recursive_connect_controls(child, controls_found)
	
	return controls_found

func _has_text_editing_methods(node: Node) -> bool:
	"""Verifica si un nodo tiene métodos típicos de componentes de texto"""
	return (node.has_method("get_text_value") and 
			node.has_method("set_text")) or \
		   (node.has_method("get_text") and 
			node.has_method("set_text"))

func _find_text_control_in_component(component: Node) -> Control:
	"""Busca un TextEdit o LineEdit dentro de un componente"""
	# Buscar hasta 3 niveles de profundidad
	return _find_text_control_recursive(component, 3)

func _find_text_control_recursive(node: Node, max_depth: int) -> Control:
	"""Busca recursivamente un TextEdit o LineEdit"""
	if max_depth <= 0:
		return null
	
	for child in node.get_children():
		if child is TextEdit or child is LineEdit:
			return child
		var found = _find_text_control_recursive(child, max_depth - 1)
		if found:
			return found
	
	return null

## Conecta manualmente un control específico
func connect_control(control: Control) -> void:
	"""
	Conecta manualmente un TextEdit o LineEdit al sistema de overlay
	Args:
		control: El control a conectar
	"""
	if not Global.is_mobile():
		print("⚠️ MobileInputManager: Conexión ignorada (no es móvil)")
		return
	
	if not (control is TextEdit or control is LineEdit):
		print("❌ MobileInputManager: Solo soporta TextEdit y LineEdit, recibido: ", control.get_class())
		return
	
	if control in connected_controls:
		print("⚠️ MobileInputManager: Control ya conectado: ", control.name)
		return
	
	# Conectar señal de focus
	if not control.focus_entered.is_connected(_on_control_focus_entered):
		control.focus_entered.connect(_on_control_focus_entered.bind(control))
		connected_controls.append(control)
		print("✅ Control conectado: ", control.name)
	else:
		print("⚠️ Control ya tenía conexión de focus: ", control.name)

## Desconecta un control del sistema
func disconnect_control(control: Control) -> void:
	"""Desconecta un control del sistema de overlay"""
	if control in connected_controls:
		if control.focus_entered.is_connected(_on_control_focus_entered):
			control.focus_entered.disconnect(_on_control_focus_entered)
		connected_controls.erase(control)
		print("🔌 Control desconectado: ", control.name)

## Abre el overlay manualmente para un control específico
func open_overlay_for_control(control: Control) -> void:
	"""
	Abre manualmente el overlay para un control específico
	Args:
		control: El control a editar
	"""
	if not Global.is_mobile():
		print("⚠️ MobileInputManager: Overlay solo disponible en móvil")
		return
	
	if not mobile_overlay:
		print("❌ MobileInputManager: Overlay no disponible")
		return
	
	print("🚀 Abriendo overlay manualmente para: ", control.name)
	current_editing_control = control
	mobile_overlay.open_for_control(control)
	emit_signal("overlay_opened", control)

# Callbacks de eventos

func _on_control_focus_entered(control: Control) -> void:
	"""Se ejecuta cuando un control conectado recibe focus"""
	print("🎯 Focus detectado en: ", control.name)
	
	# Solo abrir overlay si no está ya visible
	if mobile_overlay and not mobile_overlay.is_really_visible():
		current_editing_control = control
		mobile_overlay.open_for_control(control)
		emit_signal("overlay_opened", control)
	else:
		if mobile_overlay:
			print("⚠️ Overlay ya visible, ignorando focus. Visible: ", mobile_overlay.visible, ", Activo: ", mobile_overlay.is_overlay_active)
		else:
			print("❌ Mobile overlay es null")

func _on_overlay_text_confirmed(text: String) -> void:
	"""Se ejecuta cuando se confirma texto en el overlay"""
	print("✅ Texto confirmado: '", text, "'")
	if current_editing_control:
		emit_signal("text_confirmed", text, current_editing_control)
		current_editing_control = null

func _on_overlay_closed() -> void:
	"""Se ejecuta cuando se cierra el overlay"""
	print("❌ Overlay cerrado")
	if current_editing_control:
		emit_signal("overlay_closed", current_editing_control)
		current_editing_control = null

## Funciones de utilidad y debug

func get_connected_controls_count() -> int:
	"""Retorna el número de controles conectados"""
	return connected_controls.size()

func get_connected_controls() -> Array[Control]:
	"""Retorna la lista de controles conectados"""
	return connected_controls.duplicate()

func is_overlay_visible() -> bool:
	"""Verifica si el overlay está visible"""
	return mobile_overlay and mobile_overlay.is_really_visible()

func debug_status() -> void:
	"""Imprime el estado actual del manager para debugging"""
	print("=== MOBILE INPUT MANAGER DEBUG ===")
	print("Es móvil: ", Global.is_mobile())
	print("Overlay disponible: ", mobile_overlay != null)
	if mobile_overlay:
		print("Overlay visible (propiedad): ", mobile_overlay.visible)
		print("Overlay activo (real): ", mobile_overlay.is_overlay_active)
		print("Overlay realmente visible: ", mobile_overlay.is_really_visible())
	else:
		print("Overlay visible: false (overlay es null)")
	print("Controles conectados: ", connected_controls.size())
	print("Control actual: ", current_editing_control.name if current_editing_control else "null")
	print("Lista de controles:")
	for i in range(connected_controls.size()):
		var ctrl = connected_controls[i]
		print("  ", i + 1, ". ", ctrl.name, " (", ctrl.get_class(), ")")
	print("=================================")

func debug_overlay_state() -> void:
	"""Debug completo del estado del overlay"""
	print("=== OVERLAY STATE DEBUG ===")
	if mobile_overlay:
		print("Overlay encontrado: Sí")
		print("Propiedad visible: ", mobile_overlay.visible)
		print("Flag is_overlay_active: ", mobile_overlay.is_overlay_active)
		print("is_really_visible(): ", mobile_overlay.is_really_visible())
		print("Control original: ", mobile_overlay.original_control.name if mobile_overlay.original_control else "null")
		print("Texto actual: '", mobile_overlay.get_current_text(), "'")
	else:
		print("Overlay encontrado: No")
	print("Control actual en manager: ", current_editing_control.name if current_editing_control else "null")
	print("=========================")

func force_test_overlay(test_text: String = "Prueba manual") -> void:
	"""Función de testing para abrir el overlay forzadamente"""
	print("🧪 FORCE TEST: Abriendo overlay...")
	if mobile_overlay:
		# Crear un control temporal para testing
		var test_control = LineEdit.new()
		test_control.name = "TestControl"
		test_control.text = test_text
		add_child(test_control)
		
		mobile_overlay.open_for_control(test_control)
		print("🧪 FORCE TEST: Overlay abierto con control temporal")
	else:
		print("❌ FORCE TEST: Overlay no disponible")

func force_close_overlay() -> void:
	"""Función de debug para forzar el cierre del overlay"""
	print("🔒 FORCE CLOSE: Cerrando overlay...")
	if mobile_overlay:
		mobile_overlay.force_close()
		current_editing_control = null
		print("🔒 FORCE CLOSE: Overlay cerrado")
	else:
		print("❌ FORCE CLOSE: Overlay no disponible")

## Funciones para integración con otros sistemas

func connect_profile_popup(popup: Control) -> void:
	"""Conecta específicamente los controles de un profile popup"""
	if not popup:
		return
	
	print("🔗 Conectando profile popup: ", popup.name)
	
	# Buscar dcl_text_edit específicos
	var dcl_controls = []
	_find_dcl_text_edits(popup, dcl_controls)
	
	for dcl_control in dcl_controls:
		var text_edit = _find_text_control_in_component(dcl_control)
		if text_edit:
			connect_control(text_edit)

func _find_dcl_text_edits(node: Node, found_controls: Array) -> void:
	"""Busca específicamente controles dcl_text_edit"""
	if node.name.begins_with("DclTextEdit"):
		found_controls.append(node)
	
	for child in node.get_children():
		_find_dcl_text_edits(child, found_controls)
