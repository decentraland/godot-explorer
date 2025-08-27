## Script de prueba para el sistema de Mobile Input Manager
## Úsalo para probar la funcionalidad del overlay en diferentes escenarios

extends Control

var test_line_edit: LineEdit
var test_text_edit: TextEdit
var test_dcl_text_edit: Control

func _ready():
	create_test_ui()
	setup_testing()

func create_test_ui():
	"""Crea una UI simple para testing"""
	# Configurar el control principal
	set_anchors_and_offsets_preset(Control.PRESET_CENTER)
	custom_minimum_size = Vector2(500, 400)
	
	# Crear contenedor principal
	var main_vbox = VBoxContainer.new()
	main_vbox.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	main_vbox.add_theme_constant_override("separation", 20)
	add_child(main_vbox)
	
	# Título
	var title = Label.new()
	title.text = "Mobile Input Manager - Test"
	title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	title.add_theme_font_size_override("font_size", 24)
	main_vbox.add_child(title)
	
	# Información
	var info = Label.new()
	info.text = "Toca los campos de texto para probar el overlay móvil"
	info.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	info.modulate = Color.GRAY
	main_vbox.add_child(info)
	
	# Contenedor de tests
	var tests_container = VBoxContainer.new()
	tests_container.add_theme_constant_override("separation", 15)
	main_vbox.add_child(tests_container)
	
	# Test 1: LineEdit básico
	var line_label = Label.new()
	line_label.text = "LineEdit básico:"
	tests_container.add_child(line_label)
	
	test_line_edit = LineEdit.new()
	test_line_edit.name = "TestLineEdit"
	test_line_edit.placeholder_text = "Toca para probar overlay..."
	test_line_edit.custom_minimum_size = Vector2(400, 40)
	tests_container.add_child(test_line_edit)
	
	# Test 2: TextEdit multilínea
	var text_label = Label.new()
	text_label.text = "TextEdit multilínea:"
	tests_container.add_child(text_label)
	
	test_text_edit = TextEdit.new()
	test_text_edit.name = "TestTextEdit"
	test_text_edit.placeholder_text = "Área de texto multilínea..."
	test_text_edit.custom_minimum_size = Vector2(400, 120)
	tests_container.add_child(test_text_edit)
	
	# Botones de control
	var button_container = HBoxContainer.new()
	button_container.add_theme_constant_override("separation", 10)
	tests_container.add_child(button_container)
	
	var btn_debug = Button.new()
	btn_debug.text = "Debug Status"
	btn_debug.pressed.connect(_on_debug_pressed)
	button_container.add_child(btn_debug)
	
	var btn_force_test = Button.new()
	btn_force_test.text = "Force Test"
	btn_force_test.pressed.connect(_on_force_test_pressed)
	button_container.add_child(btn_force_test)
	
	var btn_connect = Button.new()
	btn_connect.text = "Connect All"
	btn_connect.pressed.connect(_on_connect_pressed)
	button_container.add_child(btn_connect)

func setup_testing():
	"""Configura las pruebas iniciales"""
	print("🧪 === MOBILE INPUT TEST INICIADO ===")
	
	# Verificar disponibilidad del manager
	if Global.mobile_input_manager:
		print("✅ Mobile Input Manager disponible")
		
		# Conectar automáticamente esta escena
		Global.mobile_input_manager.auto_connect_scene_controls(self)
		
		# Conectar señales para feedback
		Global.mobile_input_manager.overlay_opened.connect(_on_test_overlay_opened)
		Global.mobile_input_manager.overlay_closed.connect(_on_test_overlay_closed)
		Global.mobile_input_manager.text_confirmed.connect(_on_test_text_confirmed)
		
		print("🔗 Controles conectados automáticamente")
	else:
		print("❌ Mobile Input Manager NO disponible")
		print("   Posibles causas:")
		print("   1. No estás ejecutando en explorer")
		print("   2. No es un dispositivo móvil")
		print("   3. El manager no está correctamente configurado")

# Callbacks de prueba
func _on_test_overlay_opened(control: Control):
	print("🚀 TEST: Overlay abierto para ", control.name)

func _on_test_overlay_closed(control: Control):
	print("❌ TEST: Overlay cerrado para ", control.name)

func _on_test_text_confirmed(text: String, control: Control):
	print("✅ TEST: Texto confirmado '", text, "' en ", control.name)

# Botones de control
func _on_debug_pressed():
	print("🔍 === DEBUG STATUS ===")
	if Global.mobile_input_manager:
		Global.mobile_input_manager.debug_status()
	else:
		print("❌ No hay manager disponible")
	print("========================")

func _on_force_test_pressed():
	print("🧪 === FORCE TEST ===")
	if Global.mobile_input_manager:
		Global.mobile_input_manager.force_test_overlay("Texto de prueba forzada")
	else:
		print("❌ No hay manager disponible")

func _on_connect_pressed():
	print("🔌 === CONNECT ALL ===")
	if Global.mobile_input_manager:
		Global.mobile_input_manager.connect_control(test_line_edit)
		Global.mobile_input_manager.connect_control(test_text_edit)
		print("✅ Controles conectados manualmente")
	else:
		print("❌ No hay manager disponible")

# Atajos de teclado para testing
func _input(event):
	if event is InputEventKey and event.pressed:
		match event.keycode:
			KEY_F1:
				_on_debug_pressed()
			KEY_F2:
				_on_force_test_pressed()
			KEY_F3:
				_on_connect_pressed()
			KEY_F4:
				print("🎯 Forzando focus en LineEdit...")
				test_line_edit.grab_focus()
			KEY_F5:
				print("🎯 Forzando focus en TextEdit...")
				test_text_edit.grab_focus()

# Función para usar desde código
static func create_and_add_to_scene(parent: Node) -> Control:
	"""Crea una instancia del test y la agrega a una escena"""
	var test_instance = preload("res://src/ui/components/mobile_input_test.gd").new()
	parent.add_child(test_instance)
	return test_instance

# Para usar este test:
# 1. Instancia este script en cualquier escena
# 2. O llama MobileInputTest.create_and_add_to_scene(get_tree().current_scene)
# 3. Usa F1-F5 para testing rápido
# 4. Toca los campos para probar el overlay
