extends VBoxContainer

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

var primary_player_profile_dictionary: Dictionary = {}

var wearable_buttons: Array = []

@onready var color_picker_panel = $Color_Picker_Panel

@onready
var button_save_profile = $ColorRect_Background/HBoxContainer/Control/VBoxContainer/Button_SaveProfile
@onready var line_edit_name = $ColorRect_Background/HBoxContainer/Control/VBoxContainer/LineEdit_Name

@onready var avatar_preview = %AvatarPreview

@onready
var v_box_container_category = $ColorRect_Background/HBoxContainer/ScrollContainer/ColorRect_Sidebar/MarginContainer/VBoxContainer/HBoxContainer2/ScrollContainer/MarginContainer/VBoxContainer
@onready
var wearable_item_instanceable = preload("res://src/ui/components/wearable_item/wearable_item.tscn")
@onready
var grid_container_wearables_list = $ColorRect_Background/HBoxContainer/ScrollContainer/ColorRect_Sidebar/MarginContainer/VBoxContainer/HBoxContainer2/VBoxContainer/ScrollContainer/GridContainer_WearablesList
@onready
var wearable_panel = $ColorRect_Background/HBoxContainer/ScrollContainer/ColorRect_Sidebar/MarginContainer/VBoxContainer/HBoxContainer2/VBoxContainer/MarginContainer/WearablePanel

@onready
var skin_color_picker = $ColorRect_Background/HBoxContainer/ScrollContainer/ColorRect_Sidebar/MarginContainer/VBoxContainer/HBoxContainer2/VBoxContainer/HBoxContainer/skin_color_picker


# gdlint:ignore = async-function-name
func _ready():
	Global.player_identity.profile_changed.connect(self._on_profile_changed)

	for child in v_box_container_category.get_children():
		# TODO: check if it's a wearable_button
		for wearable_button in child.get_children():
			wearable_button.filter_type.connect(self._on_wearable_button_filter_type)
			wearable_button.clear_filter.connect(self._on_wearable_button_clear_filter)
			wearable_buttons.push_back(wearable_button)

	for wearable_id in Wearables.BASE_WEARABLES:
		var key = "urn:decentraland:off-chain:base-avatars:" + wearable_id
		wearable_data[key] = null

	var promise = Global.content_manager.fetch_wearables(
		wearable_data.keys(), "https://peer.decentraland.org/content/"
	)
	if promise != null:
		await PromiseUtils.async_awaiter(promise)

	for wearable_id in wearable_data:
		wearable_data[wearable_id] = Global.content_manager.get_wearable(wearable_id)

	_update_avatar()


func _on_profile_changed(new_profile: Dictionary):
	var profile_content = new_profile.get("content", {})
	line_edit_name.text = profile_content.get("name")
	
	var profile_avatar = profile_content.get("avatar", {})
	avatar_body_shape = profile_avatar.bodyShape
	avatar_wearables = profile_avatar.wearables
	avatar_eyes_color = Avatar.from_color_object(profile_avatar.eyes.color)
	avatar_hair_color = Avatar.from_color_object(profile_avatar.hair.color)
	avatar_skin_color = Avatar.from_color_object(profile_avatar.skin.color)
	avatar_emotes = profile_avatar.emotes
	
	if primary_player_profile_dictionary.is_empty():
		primary_player_profile_dictionary = new_profile.duplicate()
	
	_update_avatar()


func _update_avatar():
	if primary_player_profile_dictionary.is_empty():
		return
		
	var profile_avatar: Dictionary = primary_player_profile_dictionary.get("content", {}).get("avatar", {})
	profile_avatar["bodyShape"] = avatar_body_shape
	profile_avatar["eyes"] = Avatar.to_color_object(avatar_eyes_color)
	profile_avatar["hair"] = Avatar.to_color_object(avatar_hair_color)
	profile_avatar["skin"] = Avatar.to_color_object(avatar_skin_color)
	profile_avatar["wearables"] = avatar_wearables
	profile_avatar["emotes"] = avatar_emotes
	
	var wearable_body_shape = Global.content_manager.get_wearable(avatar_body_shape)

	# TODO: make this more performant
	for wearable_button in wearable_buttons:
		for wearable_hash in avatar_wearables:
			var wearable = Global.content_manager.get_wearable(wearable_hash)
			if wearable != null:
				wearable_button.async_set_wearable(wearable)

		if wearable_body_shape != null:
			wearable_button.async_set_wearable(wearable_body_shape)

	avatar_preview.avatar.async_update_avatar_from_profile(primary_player_profile_dictionary)
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
		wearable_item.async_set_wearable(wearable_data[wearable_id])
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

	wearable_panel.async_set_wearable(wearable_data[wearable_id], wearable_id)
	wearable_panel.set_equipable_and_equip(can_equip, equipped)


func _on_wearable_button_filter_type(type):
	load_filtered_data(type)
	avatar_preview.focus_camera_on(type)

	var should_hide = false
	if type == Wearables.Categories.BODY_SHAPE:
		skin_color_picker.color_target = skin_color_picker.ColorTarget.SKIN
		skin_color_picker.set_color(avatar_skin_color)
		skin_color_picker.set_text("SKIN COLOR")
	elif type == Wearables.Categories.HAIR or type == Wearables.Categories.FACIAL_HAIR:
		skin_color_picker.color_target = skin_color_picker.ColorTarget.HAIR
		skin_color_picker.set_color(avatar_hair_color)
		skin_color_picker.set_text("HAIR COLOR")
	elif type == Wearables.Categories.EYES:
		skin_color_picker.color_target = skin_color_picker.ColorTarget.EYE
		skin_color_picker.set_color(avatar_eyes_color)
		skin_color_picker.set_text("EYE COLOR")
	else:
		should_hide = true

	if should_hide:
		skin_color_picker.hide()
	else:
		skin_color_picker.show()


func _on_wearable_button_clear_filter(_type):
	filtered_data = []
	show_wearables()


func _on_line_edit_name_text_changed(_new_text):
	button_save_profile.disabled = false


func _on_button_save_profile_pressed():
	button_save_profile.disabled = true
	
	var profile_content = primary_player_profile_dictionary.get("content", {})
	var profile_avatar = profile_content.get("avatar", {})
	
	profile_content["name"] = line_edit_name.text
	profile_content["hasConnectedWeb3"] = !Global.player_identity.is_guest
	profile_avatar["name"] = line_edit_name.text
	
	Global.player_identity.async_deploy_profile(primary_player_profile_dictionary)


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

	var index = avatar_wearables.find(wearable_id)
	if index != -1:
		avatar_wearables.remove_at(index)

	_update_avatar()


func _on_skin_color_picker_toggle_color_panel(toggled, color_target):
	if not toggled and color_picker_panel.visible:
		hide()

	if toggled:
		var rect = skin_color_picker.get_global_rect()
		rect.position.y += rect.size.y + 10

		var current_color: Color
		match skin_color_picker.color_target:
			skin_color_picker.ColorTarget.EYE:
				color_picker_panel.color_type = color_picker_panel.ColorTargetType.OTHER
				current_color = avatar_eyes_color
			skin_color_picker.ColorTarget.SKIN:
				color_picker_panel.color_type = color_picker_panel.ColorTargetType.SKIN
				current_color = avatar_skin_color
			skin_color_picker.ColorTarget.HAIR:
				color_picker_panel.color_type = color_picker_panel.ColorTargetType.OTHER
				current_color = avatar_hair_color

		color_picker_panel.custom_popup(rect, current_color)


func _on_color_picker_panel_popup_hide():
	skin_color_picker.set_toggled(false)


func _on_color_picker_panel_pick_color(color):
	match skin_color_picker.color_target:
		skin_color_picker.ColorTarget.EYE:
			avatar_eyes_color = color
		skin_color_picker.ColorTarget.SKIN:
			avatar_skin_color = color
		skin_color_picker.ColorTarget.HAIR:
			avatar_hair_color = color

	skin_color_picker.set_color(color)
	avatar_preview.avatar.update_colors(avatar_eyes_color, avatar_skin_color, avatar_hair_color)

	primary_player_profile_dictionary["eyes"] = Avatar.to_color_object(avatar_eyes_color)
	primary_player_profile_dictionary["hair"] = Avatar.to_color_object(avatar_hair_color)
	primary_player_profile_dictionary["skin"] = Avatar.to_color_object(avatar_skin_color)
	button_save_profile.disabled = false


func _on_button_logout_pressed():
	Global.comms.disconnect(true)
