extends "res://addons/dcl_dev_tools/dev_tools/dcl_dev_tool.gd"

var test_types = {
	"Avatar Tests": "avatar",
	"Scene Tests": "scene",
	"Client Tests": "client",
	"Run All Tests": "all"
}

var test_dialog: AcceptDialog
var test_list: ItemList
var run_button: Button
var selected_test_index: int = -1


func populate_menu(menu: PopupMenu, id: int):
	menu.add_item("Run Tests...", id)
	menu.set_item_shortcut(menu.get_item_index(id), _create_shortcut())


func _create_shortcut() -> Shortcut:
	var shortcut = Shortcut.new()
	var event = InputEventKey.new()
	event.keycode = KEY_T
	event.ctrl_pressed = true
	shortcut.events = [event]
	return shortcut


func _create_test_dialog():
	if test_dialog:
		return

	test_dialog = AcceptDialog.new()
	test_dialog.title = "Run Tests"
	test_dialog.size = Vector2(400, 400)
	test_dialog.unresizable = false
	test_dialog.get_ok_button().hide()

	var vbox = VBoxContainer.new()
	vbox.add_theme_constant_override("separation", 10)

	var label = Label.new()
	label.text = "Select a test suite to run:"
	vbox.add_child(label)

	test_list = ItemList.new()
	test_list.custom_minimum_size = Vector2(0, 250)
	test_list.select_mode = ItemList.SELECT_SINGLE

	for test_name in test_types.keys():
		test_list.add_item(test_name)

	# Select first item by default
	if test_list.item_count > 0:
		test_list.select(0)
		selected_test_index = 0

	test_list.item_selected.connect(_on_test_selected)
	test_list.item_activated.connect(_on_test_activated)
	vbox.add_child(test_list)

	var button_container = HBoxContainer.new()
	button_container.alignment = BoxContainer.ALIGNMENT_END
	button_container.add_theme_constant_override("separation", 10)

	var cancel_button = Button.new()
	cancel_button.text = "Cancel"
	cancel_button.pressed.connect(func(): test_dialog.hide())
	button_container.add_child(cancel_button)

	run_button = Button.new()
	run_button.text = "Run"
	run_button.disabled = false  # Enabled by default since first item is selected
	run_button.pressed.connect(_run_selected_test)
	button_container.add_child(run_button)

	vbox.add_child(button_container)
	test_dialog.add_child(vbox)

	plugin.get_editor_interface().get_base_control().add_child(test_dialog)


func _on_test_selected(index: int):
	selected_test_index = index
	run_button.disabled = false


func _on_test_activated(_index: int):
	_run_selected_test()


func _run_selected_test():
	if selected_test_index < 0 or selected_test_index >= test_types.size():
		return

	var test_name = test_types.keys()[selected_test_index]
	var test_type = test_types[test_name]

	test_dialog.hide()

	match test_type:
		"avatar":
			_run_avatar_tests()
		"scene":
			_run_scene_tests()
		"client":
			_run_client_tests()
		"all":
			_run_all_tests()

	# Reset to first item for next time
	if test_list and test_list.item_count > 0:
		test_list.select(0)
		selected_test_index = 0
		run_button.disabled = false


func execute():
	_create_test_dialog()
	test_dialog.popup_centered()
	# Autofocus the run button so Enter key works immediately
	if run_button:
		run_button.grab_focus()


func _run_avatar_tests():
	print("🎭 Running Avatar Tests...")
	var old_args = ProjectSettings.get("editor/run/main_run_args")
	ProjectSettings.set("editor/run/main_run_args", "--avatar-renderer --use-test-input")
	plugin.get_editor_interface().play_main_scene()
	ProjectSettings.set("editor/run/main_run_args", old_args)


func _run_scene_tests():
	print("🏗️ Running Scene Tests...")
	var old_args = ProjectSettings.get("editor/run/main_run_args")
	# TODO: Add UI to specify parcels or use default
	ProjectSettings.set("editor/run/main_run_args", "--scene-test [[52,-52]]")
	plugin.get_editor_interface().play_main_scene()
	ProjectSettings.set("editor/run/main_run_args", old_args)


func _run_client_tests():
	print("🎨 Running Client Tests...")
	var old_args = ProjectSettings.get("editor/run/main_run_args")
	ProjectSettings.set("editor/run/main_run_args", "--client-test")
	plugin.get_editor_interface().play_main_scene()
	ProjectSettings.set("editor/run/main_run_args", old_args)


func _run_all_tests():
	print("🚀 Running All Tests Sequentially...")
	_run_all_tests_async()


func _run_all_tests_async():
	var interface = plugin.get_editor_interface()
	var old_args = ProjectSettings.get("editor/run/main_run_args")

	# Run Avatar Tests
	print("🎭 Running Avatar Tests...")
	ProjectSettings.set("editor/run/main_run_args", "--avatar-renderer --use-test-input")
	await plugin.get_tree().create_timer(0.1).timeout
	interface.play_main_scene()
	while interface.get_playing_scene() != "":
		await plugin.get_tree().create_timer(0.1).timeout
	print("✅ Avatar Tests completed")

	# Run Scene Tests
	print("🏗️ Running Scene Tests...")
	ProjectSettings.set("editor/run/main_run_args", "--scene-test [[52,-52]]")
	await plugin.get_tree().create_timer(0.1).timeout
	interface.play_main_scene()
	while interface.get_playing_scene() != "":
		await plugin.get_tree().create_timer(0.1).timeout
	print("✅ Scene Tests completed")

	# Run Client Tests
	print("🎨 Running Client Tests...")
	ProjectSettings.set("editor/run/main_run_args", "--client-test")
	await plugin.get_tree().create_timer(0.1).timeout
	interface.play_main_scene()
	while interface.get_playing_scene() != "":
		await plugin.get_tree().create_timer(0.1).timeout
	print("✅ Client Tests completed")

	# Restore original arguments
	ProjectSettings.set("editor/run/main_run_args", old_args)
	print("🚀 All tests completed successfully!")


func cleanup():
	if test_dialog and is_instance_valid(test_dialog):
		test_dialog.queue_free()
		test_dialog = null
