extends Control

const WEARABLE_PANEL = preload("res://src/ui/components/wearable_panel/wearable_panel.tscn")
const WEARABLE_ITEM_INSTANTIABLE = preload(
	"res://src/ui/components/wearable_item/wearable_item.tscn"
)
const FILTER: Texture = preload("res://assets/ui/Filter.svg")

var wearable_button_group = ButtonGroup.new()
var filtered_data: Array

var base_wearable_request_id: int = -1
var wearable_data: Dictionary = {}

var mutable_avatar: DclAvatarWireFormat
var mutable_profile: DclUserProfile

var wearable_filter_buttons: Array[WearableFilterButton] = []
var main_category_selected: String = "body_shape"
var request_update_avatar: bool = false  # debounce
var request_show_wearables: bool = false  # debounce

var avatar_wearables_body_shape_cache: Dictionary = {}

var avatar_loading_counter: int = 0

@onready var skin_color_picker = %Color_Picker_Button
@onready var color_picker_panel = $Color_Picker_Panel
@onready var grid_container_wearables_list = %GridContainer_WearablesList

@onready var line_edit_name = %LineEdit_Name
@onready var avatar_preview = %AvatarPreview
@onready var avatar_loading = %TextureProgressBar_AvatarLoading
@onready var button_save_profile = %Button_SaveProfile

@onready var container_main_categories = %HBoxContainer_MainCategories
@onready var container_sub_categories = %HBoxContainer_SubCategories
@onready var scroll_container_sub_categories = %ScrollContainer_SubCategories
@onready var menu_button_filter = %MenuButton_Filter

@onready var vboxcontainer_wearable_selector = %VBoxContainer_WearableSelector

@onready var control_no_items = %Control_NoItems
@onready var backpack_loading = %TextureProgressBar_BackpackLoading
@onready var container_backpack = %HBoxContainer_Backpack


# gdlint:ignore = async-function-name
func _ready():
	mutable_profile = DclUserProfile.new()
	mutable_avatar = mutable_profile.get_avatar()

	container_backpack.hide()
	backpack_loading.show()

	skin_color_picker.hide()
	Global.player_identity.profile_changed.connect(self._on_profile_changed)

	menu_button_filter.text = "FILTER"
	menu_button_filter.icon = FILTER

	for wearable_filter_button in container_main_categories.get_children():
		if wearable_filter_button is WearableFilterButton:
			wearable_filter_button.filter_type.connect(self._on_main_category_filter_type)

	for wearable_filter_button in container_sub_categories.get_children():
		if wearable_filter_button is WearableFilterButton:
			wearable_filter_button.filter_type.connect(self._on_wearable_filter_button_filter_type)
			wearable_filter_buttons.push_back(wearable_filter_button)

	for wearable_id in Wearables.BASE_WEARABLES:
		var key = Wearables.get_base_avatar_urn(wearable_id)
		wearable_data[key] = null

	var promise = Global.content_provider.fetch_wearables(
		wearable_data.keys(), "https://peer.decentraland.org/content/"
	)
	await PromiseUtils.async_all(promise)

	for wearable_id in wearable_data:
		wearable_data[wearable_id] = Global.content_provider.get_wearable(wearable_id)
		if wearable_data[wearable_id] == null:
			printerr("Error loading wearable_id ", wearable_id)

	_update_visible_categories()

	request_update_avatar = true

	container_backpack.show()
	backpack_loading.hide()


func _update_visible_categories():
	var showed_subcategories: int = 0
	var first_wearable_filter_button: WearableFilterButton = null
	for wearable_filter_button: WearableFilterButton in wearable_filter_buttons:
		var category = wearable_filter_button.get_category_name()
		var filter_categories: Array = Wearables.Categories.MAIN_CATEGORIES.get(
			main_category_selected
		)
		var category_is_visible: bool = filter_categories.has(category)
		wearable_filter_button.visible = category_is_visible
		if category_is_visible:
			showed_subcategories += 1
			if first_wearable_filter_button == null:
				first_wearable_filter_button = wearable_filter_button

	scroll_container_sub_categories.set_visible(showed_subcategories >= 2)
	if first_wearable_filter_button:
		first_wearable_filter_button.set_pressed(true)


func _on_profile_changed(new_profile: DclUserProfile):
	line_edit_name.text = new_profile.get_name()

	mutable_profile = new_profile.duplicated()
	mutable_avatar = mutable_profile.get_avatar()

	request_update_avatar = true
	request_show_wearables = true


func _physics_process(_delta):
	if request_update_avatar:
		request_update_avatar = false
		_async_update_avatar()

	if request_show_wearables:
		request_show_wearables = false
		show_wearables()


func set_avatar_loading() -> int:
	avatar_preview.hide()
	avatar_loading.show()
	avatar_loading_counter += 1
	return avatar_loading_counter


func unset_avatar_loading(current: int):
	if current != avatar_loading_counter:
		return
	avatar_loading.hide()
	avatar_preview.show()


func _async_update_avatar():
	mutable_profile.set_avatar(mutable_avatar)

	var loading_id := set_avatar_loading()
	await avatar_preview.avatar.async_update_avatar_from_profile(mutable_profile)
	unset_avatar_loading(loading_id)
	button_save_profile.disabled = false


func load_filtered_data(filter: String):
	if mutable_avatar == null:
		return

	filtered_data = []
	for wearable_id in wearable_data:
		var wearable = wearable_data[wearable_id]
		if wearable != null:
			if wearable.get_category() == filter:
				if (
					Wearables.can_equip(wearable, mutable_avatar.get_body_shape())
					or wearable.get_category() == "body_shape"
				):
					filtered_data.push_back(wearable_id)

	request_show_wearables = true


func can_unequip(category: String) -> bool:
	return (
		category != Wearables.Categories.BODY_SHAPE
		and category != Wearables.Categories.EYES
		and category != Wearables.Categories.MOUTH
	)


func show_wearables():
	for child in grid_container_wearables_list.get_children():
		child.queue_free()

	control_no_items.visible = filtered_data.is_empty()
	grid_container_wearables_list.visible = not filtered_data.is_empty()

	for wearable_id in filtered_data:
		var wearable_item = WEARABLE_ITEM_INSTANTIABLE.instantiate()
		var wearable = wearable_data[wearable_id]
		grid_container_wearables_list.add_child(wearable_item)
		wearable_button_group.allow_unpress = can_unequip(wearable.get_category())
		wearable_item.button_group = wearable_button_group
		wearable_item.async_set_wearable(wearable)

		# Connect signals
		wearable_item.equip.connect(self._on_wearable_equip.bind(wearable_id))
		wearable_item.unequip.connect(self._on_wearable_unequip.bind(wearable_id))

		# Check if the item is equipped
		var is_wearable_pressed = (
			mutable_avatar.get_wearables().has(wearable_id)
			or mutable_avatar.get_body_shape() == wearable_id
		)
		wearable_item.set_pressed_no_signal(is_wearable_pressed)
		wearable_item.set_equiped(is_wearable_pressed)


func _on_main_category_filter_type(type: String):
	main_category_selected = type
	_update_visible_categories()


func _on_wearable_filter_button_filter_type(type):
	load_filtered_data(type)
	avatar_preview.focus_camera_on(type)

	var should_hide = false
	if type == Wearables.Categories.BODY_SHAPE:
		skin_color_picker.color_target = skin_color_picker.ColorTarget.SKIN
		skin_color_picker.set_color(mutable_avatar.get_skin_color())
	elif type == Wearables.Categories.HAIR or type == Wearables.Categories.FACIAL_HAIR:
		skin_color_picker.color_target = skin_color_picker.ColorTarget.HAIR
		skin_color_picker.set_color(mutable_avatar.get_hair_color())
	elif type == Wearables.Categories.EYES:
		skin_color_picker.color_target = skin_color_picker.ColorTarget.EYE
		skin_color_picker.set_color(mutable_avatar.get_eyes_color())
	else:
		should_hide = true

	if should_hide:
		skin_color_picker.hide()
	else:
		skin_color_picker.show()


func _on_line_edit_name_text_changed(_new_text):
	button_save_profile.disabled = false


func save_profile():
	mutable_profile.set_has_connected_web3(!Global.player_identity.is_guest)
	mutable_profile.set_name(line_edit_name.text)
	mutable_avatar.set_name(line_edit_name.text)

	mutable_profile.set_avatar(mutable_avatar)
	Global.player_identity.async_deploy_profile(mutable_profile)


func _on_button_save_profile_pressed():
	button_save_profile.disabled = true
	save_profile()


func _on_wearable_equip(wearable_id: String):
	var desired_wearable = wearable_data[wearable_id]
	var category = desired_wearable.get_category()

	if category == Wearables.Categories.BODY_SHAPE:
		var current_body_shape_id := mutable_avatar.get_body_shape()
		var new_body_shape_id := wearable_id
		if current_body_shape_id != new_body_shape_id:
			avatar_wearables_body_shape_cache[current_body_shape_id] = (
				mutable_avatar.get_wearables().duplicate()
			)

			mutable_avatar.set_body_shape(new_body_shape_id)
			var default_wearables: Dictionary = Wearables.DefaultWearables.BY_BODY_SHAPES.get(
				new_body_shape_id
			)
			var new_avatar_wearables = avatar_wearables_body_shape_cache.get(new_body_shape_id, [])
			if new_avatar_wearables.is_empty():
				new_avatar_wearables = default_wearables.values()

			mutable_avatar.set_wearables(PackedStringArray(new_avatar_wearables))
	else:
		var new_avatar_wearables := mutable_avatar.get_wearables()
		var to_remove = []
		# Unequip current wearable with category
		for current_wearable_id in new_avatar_wearables:
			# TODO: put the fetch wearable function
			var wearable = wearable_data[current_wearable_id]
			if wearable.get_category() == category:
				to_remove.push_back(current_wearable_id)

		for to_remove_id in to_remove:
			var index = new_avatar_wearables.find(to_remove_id)
			new_avatar_wearables.remove_at(index)

		new_avatar_wearables.append(wearable_id)
		mutable_avatar.set_wearables(new_avatar_wearables)

	request_update_avatar = true


func _on_wearable_unequip(wearable_id: String):
	var desired_wearable = wearable_data[wearable_id]
	var category = desired_wearable.get_category()

	if category == Wearables.Categories.BODY_SHAPE:
		# TODO: can not unequip a body shape
		return

	var new_avatar_wearables := mutable_avatar.get_wearables()
	var index = new_avatar_wearables.find(wearable_id)
	if index != -1:
		new_avatar_wearables.remove_at(index)

	mutable_avatar.set_wearables(new_avatar_wearables)
	request_update_avatar = true


func _on_button_logout_pressed():
	Global.comms.disconnect(true)


func _on_color_picker_panel_pick_color(color: Color):
	match skin_color_picker.color_target:
		skin_color_picker.ColorTarget.EYE:
			mutable_avatar.set_eyes_color(color)
		skin_color_picker.ColorTarget.SKIN:
			mutable_avatar.set_skin_color(color)
		skin_color_picker.ColorTarget.HAIR:
			mutable_avatar.set_hair_color(color)

	skin_color_picker.set_color(color)
	avatar_preview.avatar.update_colors(
		mutable_avatar.get_eyes_color(),
		mutable_avatar.get_skin_color(),
		mutable_avatar.get_hair_color()
	)
	button_save_profile.disabled = false


func _on_color_picker_button_toggle_color_panel(toggled, color_target):
	if not toggled and color_picker_panel.visible:
		hide()

	if toggled:
		var rect = skin_color_picker.get_global_rect()
		rect.position.x += rect.size.x
		rect.position.y += rect.size.y + 10

		var current_color: Color
		match skin_color_picker.color_target:
			skin_color_picker.ColorTarget.EYE:
				color_picker_panel.color_type = color_picker_panel.ColorTargetType.OTHER
				current_color = mutable_avatar.get_eyes_color()
			skin_color_picker.ColorTarget.SKIN:
				color_picker_panel.color_type = color_picker_panel.ColorTargetType.SKIN
				current_color = mutable_avatar.get_skin_color()
			skin_color_picker.ColorTarget.HAIR:
				color_picker_panel.color_type = color_picker_panel.ColorTargetType.OTHER
				current_color = mutable_avatar.get_hair_color()

		color_picker_panel.custom_popup(rect, current_color)


func _on_color_picker_panel_hided():
	skin_color_picker.set_pressed(false)


func _on_hidden():
	save_profile()


func _on_rich_text_box_open_marketplace_meta_clicked(_meta):
	Global.open_url("https://decentraland.org/marketplace/browse?section=wearables")
