extends VBoxContainer

@onready
var button_save_profile = $ColorRect_Background/HBoxContainer/Control/VBoxContainer/Button_SaveProfile
@onready var line_edit_name = $ColorRect_Background/HBoxContainer/Control/VBoxContainer/LineEdit_Name

@onready var avatar_preview = %AvatarPreview

@onready
var button_equip = $ColorRect_Background/HBoxContainer/ScrollContainer/ColorRect_Sidebar/MarginContainer/VBoxContainer/HBoxContainer2/VBoxContainer/MarginContainer/PanelContainer/HBoxContainer/MarginContainer/Button_Equip
@onready
var v_box_container_category = $ColorRect_Background/HBoxContainer/ScrollContainer/ColorRect_Sidebar/MarginContainer/VBoxContainer/HBoxContainer2/ScrollContainer/MarginContainer/VBoxContainer
@onready
var wearable_item_instanceable = preload("res://src/ui/components/wearable_item/wearable_item.tscn")
@onready
var grid_container_wearables_list = $ColorRect_Background/HBoxContainer/ScrollContainer/ColorRect_Sidebar/MarginContainer/VBoxContainer/HBoxContainer2/VBoxContainer/ScrollContainer/GridContainer_WearablesList
@onready
var sprite_2d_preview = $ColorRect_Background/HBoxContainer/ScrollContainer/ColorRect_Sidebar/MarginContainer/VBoxContainer/HBoxContainer2/VBoxContainer/MarginContainer/PanelContainer/HBoxContainer/MarginContainer2/Sprite2D_Preview
@onready
var wearable_panel = $ColorRect_Background/HBoxContainer/ScrollContainer/ColorRect_Sidebar/MarginContainer/VBoxContainer/HBoxContainer2/VBoxContainer/MarginContainer/WearablePanel

var filtered_data: Array
var items_button_group = ButtonGroup.new()

var avatar_body_shape: String
var avatar_wearables: PackedStringArray
var avatar_eyes_color: Color
var avatar_hair_color: Color
var avatar_skin_color: Color
var avatar_emotes: Array

var base_wearable_request_id: int = -1
var wearable_data: Dictionary = {}

var renderer_avatar_dictionary: Dictionary = {}

var wearable_buttons: Array = []


func _ready():
	for child in v_box_container_category.get_children():
		# TODO: check if it's a wearable_button
		for wearable_button in child.get_children():
			wearable_button.filter_type.connect(self._on_wearable_button_filter_type)
			wearable_button.filter_type.connect(self._on_wearable_button_clear_filter)
			wearable_buttons.push_back(wearable_button)

	Global.content_manager.wearable_data_loaded.connect(self._on_wearable_data_loaded)

	for wearable_id in Wearables.BASE_WEARABLES:
		var key = "urn:decentraland:off-chain:base-avatars:" + wearable_id
		wearable_data[key] = null

	base_wearable_request_id = Global.content_manager.fetch_wearables(
		wearable_data.keys(), "https://peer.decentraland.org/content/"
	)

	avatar_body_shape = Global.config.avatar_profile.body_shape
	avatar_wearables = Global.config.avatar_profile.wearables
	avatar_eyes_color = Global.config.avatar_profile.eyes
	avatar_hair_color = Global.config.avatar_profile.hair
	avatar_skin_color = Global.config.avatar_profile.skin
	avatar_emotes = Global.config.avatar_profile.emotes
	line_edit_name.text = Global.config.avatar_profile.name


func _on_wearable_data_loaded(req_id: int):
	if base_wearable_request_id == -1 or req_id != base_wearable_request_id:
		return

	for wearable_id in wearable_data:
		wearable_data[wearable_id] = Global.content_manager.get_wearable(wearable_id)

	_update_avatar()


func _update_avatar():
	renderer_avatar_dictionary = {
		"base_url": "https://peer.decentraland.org/content",
		"name": "",
		"body_shape": avatar_body_shape,
		"eyes": avatar_eyes_color,
		"hair": avatar_hair_color,
		"skin": avatar_skin_color,
		"wearables": avatar_wearables,
		"emotes": avatar_emotes
	}

	# TODO: make this more performant
	for wearable_button in wearable_buttons:
		for wearable in avatar_wearables:
			wearable_button.set_wearable(Global.content_manager.get_wearable(wearable))
		wearable_button.set_wearable(Global.content_manager.get_wearable(avatar_body_shape))

	avatar_preview.avatar.update_avatar(renderer_avatar_dictionary)
	button_save_profile.disabled = false


func load_filtered_data(filter: String):
	wearable_panel.unset_wearable()
	filtered_data = []
	for wearable_id in wearable_data:
		var wearable = wearable_data[wearable_id]
		if Wearables.get_category(wearable) == filter:
			filtered_data.push_back(wearable_id)

	show_wearables()


func show_wearables():
	for child in grid_container_wearables_list.get_children():
		child.queue_free()

	for wearable_id in filtered_data:
		var wearable_item = wearable_item_instanceable.instantiate()
		grid_container_wearables_list.add_child(wearable_item)
		wearable_item.button_group = items_button_group
		wearable_item.set_wearable(wearable_data[wearable_id])
		wearable_item.toggled.connect(self._on_wearable_toggled.bind(wearable_id))


func _on_wearable_toggled(_button_toggled: bool, wearable_id: String) -> void:
	var desired_wearable = wearable_data[wearable_id]
	var category = Wearables.get_category(desired_wearable)

	var equipped = false
	var can_equip = true
	if category != Wearables.Categories.BODY_SHAPE:
		can_equip = Wearables.can_equip(desired_wearable, avatar_body_shape)
		for current_wearable_id in avatar_wearables:
			if current_wearable_id == wearable_id:
				equipped = true
				break
	else:
		equipped = avatar_body_shape == wearable_id

	wearable_panel.set_wearable(wearable_data[wearable_id], wearable_id)
	wearable_panel.set_equipable_and_equip(can_equip, equipped)


func _on_wearable_button_filter_type(type):
	load_filtered_data(type)


func _on_wearable_button_clear_filter():
	filtered_data = []
	show_wearables()


func _on_line_edit_name_text_changed(new_text):
	button_save_profile.disabled = false


func _on_button_save_profile_pressed():
	button_save_profile.disabled = true
	renderer_avatar_dictionary["name"] = line_edit_name.text

	Global.config.avatar_profile = renderer_avatar_dictionary
	Global.config.save_to_settings_file()

	Global.comms.update_profile_avatar(renderer_avatar_dictionary)


func _on_wearable_panel_equip(wearable_id: String):
	var desired_wearable = wearable_data[wearable_id]
	var category = Wearables.get_category(desired_wearable)

	if category == Wearables.Categories.BODY_SHAPE:
		avatar_body_shape = wearable_id
	else:
		var to_remove = []
		# Unequip current wearable with category
		for current_wearable_id in avatar_wearables:
			# TODO: put the fetch wearable function
			var wearable = wearable_data[current_wearable_id]
			if Wearables.get_category(wearable) == category:
				to_remove.push_back(current_wearable_id)

		for to_remove_id in to_remove:
			var index = avatar_wearables.find(to_remove_id)
			avatar_wearables.remove_at(index)

		avatar_wearables.append(wearable_id)

	_update_avatar()


func _on_wearable_panel_unequip(wearable_id: String):
	var desired_wearable = wearable_data[wearable_id]
	var category = Wearables.get_category(desired_wearable)

	if category == Wearables.Categories.BODY_SHAPE:
		# TODO: can not unequip a body shape
		return

	else:
		var index = avatar_wearables.find(wearable_id)
		if index != -1:
			avatar_wearables.remove_at(index)

	_update_avatar()
