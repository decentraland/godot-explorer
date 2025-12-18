extends "res://addons/dcl_dev_tools/dev_tools/dcl_dev_tool.gd"

const CONFIG_PATH = "user://dev_tools_places.json"

var default_places = {
	"Genesis Plaza": {"coords": Vector2i(0, 0)},
	"kuruk.dcl.eth": {"coords": Vector2i(0, 0), "realm": "kuruk.dcl.eth"},
	"Soul Magic": {"coords": Vector2i(96, -110)},
	"Tower of Hanoi": {"coords": Vector2i(61, -22)},
	"Meta gamimall": {"coords": Vector2i(1, 95)},
	"Wondermine": {"coords": Vector2i(-29, 55)},
	"Exodus": {"coords": Vector2i(125, 41)},
	"BitCinema": {"coords": Vector2i(-109, -93)},
	"DAO HQ": {"coords": Vector2i(65, 13)},
	"Casa Roustan": {"coords": Vector2i(37, -114)},
	"Fashion Week Scene": {"coords": Vector2i(142, -78)},
	"Game Night": {"coords": Vector2i(1, 81)}
}

var places = {}
var last_selected_place: String = ""
var last_coords: Vector2i = Vector2i(0, 0)
var last_realm: String = ""

var launch_dialog: AcceptDialog
var place_list: ItemList
var launch_button: Button
var delete_button: Button
var add_button: Button
var x_input: SpinBox
var y_input: SpinBox
var realm_input: LineEdit
var selected_place_index: int = -1

var name_dialog: AcceptDialog
var name_input: LineEdit


func _load_config():
	if FileAccess.file_exists(CONFIG_PATH):
		var file = FileAccess.open(CONFIG_PATH, FileAccess.READ)
		var json = JSON.new()
		var error = json.parse(file.get_as_text())
		file.close()
		if error == OK:
			var data = json.data
			if data.has("places"):
				places = {}
				for place_name in data["places"]:
					var place_data = data["places"][place_name]
					places[place_name] = {
						"coords": Vector2i(place_data["coords"][0], place_data["coords"][1])
					}
					if place_data.has("realm") and place_data["realm"] != "":
						places[place_name]["realm"] = place_data["realm"]
			if data.has("last_selected"):
				last_selected_place = data["last_selected"]
			if data.has("last_coords"):
				last_coords = Vector2i(data["last_coords"][0], data["last_coords"][1])
			if data.has("last_realm"):
				last_realm = data["last_realm"]
			return
	# Use defaults if no config exists
	places = default_places.duplicate(true)


func _save_config():
	var data = {
		"places": {},
		"last_selected": last_selected_place,
		"last_coords": [last_coords.x, last_coords.y],
		"last_realm": last_realm
	}
	for place_name in places:
		var place_data = places[place_name]
		data["places"][place_name] = {
			"coords": [place_data["coords"].x, place_data["coords"].y],
			"realm": place_data.get("realm", "")
		}
	var file = FileAccess.open(CONFIG_PATH, FileAccess.WRITE)
	file.store_string(JSON.stringify(data, "\t"))
	file.close()


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

	_load_config()

	launch_dialog = AcceptDialog.new()
	launch_dialog.title = "Launch on Location"
	launch_dialog.size = Vector2(400, 450)
	launch_dialog.unresizable = false
	launch_dialog.get_ok_button().hide()

	var vbox = VBoxContainer.new()
	vbox.add_theme_constant_override("separation", 10)

	var label = Label.new()
	label.text = "Saved locations:"
	vbox.add_child(label)

	place_list = ItemList.new()
	place_list.custom_minimum_size = Vector2(0, 200)
	place_list.select_mode = ItemList.SELECT_SINGLE
	place_list.item_selected.connect(_on_place_selected)
	place_list.item_activated.connect(_on_place_activated)
	vbox.add_child(place_list)

	# List buttons
	var list_buttons = HBoxContainer.new()
	list_buttons.add_theme_constant_override("separation", 10)

	add_button = Button.new()
	add_button.text = "+ Add"
	add_button.pressed.connect(_show_add_dialog)
	list_buttons.add_child(add_button)

	delete_button = Button.new()
	delete_button.text = "Delete"
	delete_button.pressed.connect(_delete_selected_place)
	delete_button.disabled = true
	list_buttons.add_child(delete_button)

	vbox.add_child(list_buttons)

	# Launch section
	var separator = HSeparator.new()
	vbox.add_child(separator)

	var launch_label = Label.new()
	launch_label.text = "Launch coordinates:"
	vbox.add_child(launch_label)

	# Coordinates input
	var coords_hbox = HBoxContainer.new()
	coords_hbox.add_theme_constant_override("separation", 10)

	var x_label = Label.new()
	x_label.text = "X:"
	coords_hbox.add_child(x_label)
	x_input = SpinBox.new()
	x_input.min_value = -150
	x_input.max_value = 150
	x_input.custom_minimum_size.x = 80
	x_input.value_changed.connect(_on_coord_changed)
	coords_hbox.add_child(x_input)

	var y_label = Label.new()
	y_label.text = "Y:"
	coords_hbox.add_child(y_label)
	y_input = SpinBox.new()
	y_input.min_value = -150
	y_input.max_value = 150
	y_input.custom_minimum_size.x = 80
	y_input.value_changed.connect(_on_coord_changed)
	coords_hbox.add_child(y_input)

	vbox.add_child(coords_hbox)

	# Realm input
	var realm_hbox = HBoxContainer.new()
	var realm_label = Label.new()
	realm_label.text = "Realm:"
	realm_label.custom_minimum_size.x = 50
	realm_hbox.add_child(realm_label)
	realm_input = LineEdit.new()
	realm_input.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	realm_input.placeholder_text = "Optional (e.g. kuruk.dcl.eth)"
	realm_input.text_changed.connect(_on_realm_changed)
	realm_hbox.add_child(realm_input)
	vbox.add_child(realm_hbox)

	# Bottom buttons
	var separator2 = HSeparator.new()
	vbox.add_child(separator2)

	var button_container = HBoxContainer.new()
	button_container.alignment = BoxContainer.ALIGNMENT_END
	button_container.add_theme_constant_override("separation", 10)

	var cancel_button = Button.new()
	cancel_button.text = "Cancel"
	cancel_button.pressed.connect(func(): launch_dialog.hide())
	button_container.add_child(cancel_button)

	launch_button = Button.new()
	launch_button.text = "Launch"
	launch_button.pressed.connect(_launch_selected_place)
	button_container.add_child(launch_button)

	vbox.add_child(button_container)
	launch_dialog.add_child(vbox)

	plugin.get_editor_interface().get_base_control().add_child(launch_dialog)

	# Create name dialog
	_create_name_dialog()


func _create_name_dialog():
	name_dialog = AcceptDialog.new()
	name_dialog.title = "Add New Place"
	name_dialog.size = Vector2(300, 100)
	name_dialog.get_ok_button().text = "Save"
	name_dialog.confirmed.connect(_add_new_place)

	var vbox = VBoxContainer.new()
	vbox.add_theme_constant_override("separation", 10)

	var label = Label.new()
	label.text = "Place name:"
	vbox.add_child(label)

	name_input = LineEdit.new()
	name_input.placeholder_text = "Enter name for this location"
	vbox.add_child(name_input)

	name_dialog.add_child(vbox)
	plugin.get_editor_interface().get_base_control().add_child(name_dialog)


func _populate_place_list():
	place_list.clear()
	for place_name in places.keys():
		var coords = places[place_name]["coords"]
		var realm = places[place_name].get("realm", "")
		var display = "%s (%d, %d)" % [place_name, coords.x, coords.y]
		if realm != "":
			display += " [%s]" % realm
		place_list.add_item(display)

	# Select last selected place
	if places.size() > 0 and last_selected_place != "" and places.has(last_selected_place):
		var keys = places.keys()
		var index = keys.find(last_selected_place)
		if index >= 0:
			place_list.select(index)
			selected_place_index = index
			delete_button.disabled = false


func _on_place_selected(index: int):
	selected_place_index = index
	delete_button.disabled = false

	var place_name = places.keys()[index]
	var place_data = places[place_name]

	# Load coords into inputs
	x_input.set_value_no_signal(place_data["coords"].x)
	y_input.set_value_no_signal(place_data["coords"].y)
	realm_input.text = place_data.get("realm", "")

	# Update last state
	last_selected_place = place_name
	last_coords = place_data["coords"]
	last_realm = place_data.get("realm", "")
	_save_config()


func _on_place_activated(_index: int):
	_launch_selected_place()


func _on_coord_changed(_value: float):
	last_coords = Vector2i(int(x_input.value), int(y_input.value))
	_save_config()


func _on_realm_changed(new_text: String):
	last_realm = new_text
	_save_config()


func _show_add_dialog():
	name_input.text = ""
	name_dialog.popup_centered()
	name_input.grab_focus()


func _add_new_place():
	var new_name = name_input.text.strip_edges()
	if new_name == "":
		return

	if places.has(new_name):
		var counter = 1
		var base_name = new_name
		while places.has(new_name):
			new_name = "%s %d" % [base_name, counter]
			counter += 1

	places[new_name] = {
		"coords": Vector2i(int(x_input.value), int(y_input.value))
	}
	if realm_input.text.strip_edges() != "":
		places[new_name]["realm"] = realm_input.text.strip_edges()

	last_selected_place = new_name
	_save_config()
	_populate_place_list()


func _delete_selected_place():
	if selected_place_index < 0 or places.size() <= 1:
		return

	var place_name = places.keys()[selected_place_index]
	places.erase(place_name)
	selected_place_index = -1
	delete_button.disabled = true
	last_selected_place = ""
	_save_config()
	_populate_place_list()


func _launch_selected_place():
	var coord = Vector2i(int(x_input.value), int(y_input.value))
	var realm = realm_input.text.strip_edges()

	# Save current coords
	last_coords = coord
	last_realm = realm
	_save_config()

	var launch_args = "--skip-lobby --location %d,%d" % [coord.x, coord.y]
	if realm != "":
		launch_args += " --realm %s" % realm

	print("Launch on: Setting args to: ", launch_args)
	ProjectSettings.set("editor/run/main_run_args", launch_args)
	launch_dialog.hide()
	await plugin.get_tree().create_timer(0.1).timeout
	plugin.get_editor_interface().play_main_scene()


func execute():
	_create_launch_dialog()
	_load_config()
	_populate_place_list()

	# Restore last coords
	x_input.set_value_no_signal(last_coords.x)
	y_input.set_value_no_signal(last_coords.y)
	realm_input.text = last_realm

	launch_dialog.popup_centered()
	if launch_button:
		launch_button.grab_focus()


func cleanup():
	if launch_dialog and is_instance_valid(launch_dialog):
		launch_dialog.queue_free()
		launch_dialog = null
	if name_dialog and is_instance_valid(name_dialog):
		name_dialog.queue_free()
		name_dialog = null
