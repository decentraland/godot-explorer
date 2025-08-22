extends "res://addons/dcl_dev_tools/dev_tools/dcl_dev_tool.gd"

var places = {
	"Genesis Plaza": Vector2i(0, 0),
	"Soul Magic": Vector2i(96, -110),
	"Tower of Hanoi": Vector2i(61, -22),
	"Meta gamimall": Vector2i(1, 95),
	"Wondermine": Vector2i(-29, 55),
	"Exodus": Vector2i(125, 41),
	"BitCinema": Vector2i(-109, -93),
	"DAO HQ": Vector2i(65, 13),
	"Casa Roustan": Vector2i(37, -114),
	"Fashion Week Scene": Vector2i(142, -78),
	"Game Night": Vector2i(1, 81)
}

var launch_dialog: AcceptDialog
var place_list: ItemList
var launch_button: Button
var selected_place_index: int = -1


func populate_menu(menu: PopupMenu, id: int):
	menu.add_item("Launch on...", id)
	menu.set_item_shortcut(menu.get_item_index(id), _create_shortcut())


func _create_shortcut() -> Shortcut:
	var shortcut = Shortcut.new()
	var event = InputEventKey.new()
	event.keycode = KEY_L
	event.ctrl_pressed = true
	shortcut.events = [event]
	return shortcut


func _create_launch_dialog():
	if launch_dialog:
		return

	launch_dialog = AcceptDialog.new()
	launch_dialog.title = "Launch on Location"
	launch_dialog.size = Vector2(400, 500)
	launch_dialog.unresizable = false
	launch_dialog.get_ok_button().hide()

	var vbox = VBoxContainer.new()
	vbox.add_theme_constant_override("separation", 10)

	var label = Label.new()
	label.text = "Select a location to launch:"
	vbox.add_child(label)

	place_list = ItemList.new()
	place_list.custom_minimum_size = Vector2(0, 350)
	place_list.select_mode = ItemList.SELECT_SINGLE

	for place in places.keys():
		place_list.add_item(place)

	# Select first item by default
	if place_list.item_count > 0:
		place_list.select(0)
		selected_place_index = 0

	place_list.item_selected.connect(_on_place_selected)
	place_list.item_activated.connect(_on_place_activated)
	vbox.add_child(place_list)

	var button_container = HBoxContainer.new()
	button_container.alignment = BoxContainer.ALIGNMENT_END
	button_container.add_theme_constant_override("separation", 10)

	var cancel_button = Button.new()
	cancel_button.text = "Cancel"
	cancel_button.pressed.connect(func(): launch_dialog.hide())
	button_container.add_child(cancel_button)

	launch_button = Button.new()
	launch_button.text = "Launch"
	launch_button.disabled = false  # Enabled by default since first item is selected
	launch_button.pressed.connect(_launch_selected_place)
	button_container.add_child(launch_button)

	vbox.add_child(button_container)
	launch_dialog.add_child(vbox)

	plugin.get_editor_interface().get_base_control().add_child(launch_dialog)


func _on_place_selected(index: int):
	selected_place_index = index
	launch_button.disabled = false


func _on_place_activated(_index: int):
	_launch_selected_place()


func _launch_selected_place():
	if selected_place_index < 0 or selected_place_index >= places.size():
		return

	var place = places.keys()[selected_place_index]
	var coord = places[place]

	var old_args = ProjectSettings.get("editor/run/main_run_args")
	ProjectSettings.set(
		"editor/run/main_run_args",
		(
			"--skip-lobby --realm https://realm-provider.decentraland.org/main --location %d,%d"
			% [coord.x, coord.y]
		)
	)
	plugin.get_editor_interface().play_main_scene()
	ProjectSettings.set("editor/run/main_run_args", old_args)

	launch_dialog.hide()
	# Reset to first item for next time
	if place_list and place_list.item_count > 0:
		place_list.select(0)
		selected_place_index = 0
		launch_button.disabled = false


func execute():
	_create_launch_dialog()
	launch_dialog.popup_centered()
	# Autofocus the launch button so Enter key works immediately
	if launch_button:
		launch_button.grab_focus()


func cleanup():
	if launch_dialog and is_instance_valid(launch_dialog):
		launch_dialog.queue_free()
		launch_dialog = null
