extends ColorRect

signal text_confirmed(text: String)
signal overlay_closed
signal keyboard_height_changed(height: int)

@onready var dcl_text_edit: Control = %DclTextEdit
@onready var margin_container: MarginContainer = %MarginContainer

var last_keyboard_height: int = 0

func _ready() -> void:
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	color = Color(0, 0, 0, 0)
	hide()
	get_viewport().size_changed.connect(_on_viewport_size_changed)

#func _on_gui_input(event: InputEvent) -> void:
	#if event is InputEventScreenTouch:
		#if event.pressed:
			#close()

func open(placeholder_text: String = "Write here...", initial_text: String = "") -> void:
	if not Global.is_mobile():
		print("Only works in mobile")
		return
	
	dcl_text_edit.place_holder = placeholder_text
	dcl_text_edit.set_text(initial_text)
	show()
	
	dcl_text_edit.text_edit.grab_focus()
	
	_start_keyboard_monitoring()

func close() -> void:
	dcl_text_edit.set_text("")
	hide()
	_stop_keyboard_monitoring()
	emit_signal("overlay_closed")

func get_text() -> String:
	return dcl_text_edit.get_text_value()

func set_text(text: String) -> void:
	dcl_text_edit.set_text(text)

func _on_dcl_text_edit_changed() -> void:
	pass

func _input(event: InputEvent) -> void:
	if not visible or not Global.is_mobile():
		return
	
	if event is InputEventKey and event.pressed:
		if event.keycode == KEY_ENTER or event.keycode == KEY_KP_ENTER:
			_confirm_text()
			get_viewport().set_input_as_handled()
		elif event.keycode == KEY_ESCAPE:
			close()
			get_viewport().set_input_as_handled()

func _confirm_text() -> void:
	if not dcl_text_edit.error:
		var text = get_text()
		emit_signal("text_confirmed", text)
		close()

func _on_viewport_size_changed() -> void:
	if visible and Global.is_mobile():
		_update_keyboard_position()

func _start_keyboard_monitoring() -> void:
	if Global.is_mobile():
		var timer = Timer.new()
		timer.wait_time = 0.1
		timer.timeout.connect(_check_keyboard_height)
		add_child(timer)
		timer.start()

func _stop_keyboard_monitoring() -> void:
	for child in get_children():
		if child is Timer:
			child.queue_free()
	
	margin_container.add_theme_constant_override("margin_bottom", 0)
	last_keyboard_height = 0

func _check_keyboard_height() -> void:
	var current_height = DisplayServer.virtual_keyboard_get_height()
	if current_height != last_keyboard_height:
		last_keyboard_height = current_height
		_update_keyboard_position()
		emit_signal("keyboard_height_changed", current_height)

func _update_keyboard_position() -> void:
	var keyboard_height = DisplayServer.virtual_keyboard_get_height()
	var margin_bottom = keyboard_height + 20 if keyboard_height > 0 else 0
	margin_container.add_theme_constant_override("margin_bottom", margin_bottom)
