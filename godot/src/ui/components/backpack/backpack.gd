class_name Backpack
extends Control

const WEARABLE_ITEM_INSTANTIABLE = preload(
	"res://src/ui/components/wearable_item/wearable_item.tscn"
)

@export var hide_navbar: bool = false

var wearable_button_group_per_category: Dictionary = {}
var filtered_data: Array
var current_filter: String = ""
var only_collectibles: bool = false

var base_wearable_request_id: int = -1
var wearable_data: Dictionary = {}

var wearable_filter_buttons: Array[WearableFilterButton] = []
var main_category_selected: String = "body"
var request_update_avatar: bool = false  # debounce
var request_show_wearables: bool = false  # debounce

var avatar_wearables_body_shape_cache: Dictionary = {}

var avatar_loading_counter: int = 0

# Timer for debounced blacklist changes
var blacklist_deploy_timer: Timer
var is_loading_profile: bool = false

@onready var skin_color_picker = %Color_Picker_Button
@onready var color_picker_panel = $Color_Picker_Panel
@onready var grid_container_wearables_list = %GridContainer_WearablesList

@onready var avatar_preview: AvatarPreview = %AvatarPreview
@onready var avatar_loading = %TextureProgressBar_AvatarLoading

@onready var container_main_categories = %HBoxContainer_MainCategories
@onready var container_sub_categories = %HBoxContainer_SubCategories

@onready var vboxcontainer_wearable_selector = %VBoxContainer_WearableSelector

@onready var control_no_items = %Control_NoItems
@onready var backpack_loading = %TextureProgressBar_BackpackLoading
@onready var container_backpack = %HBoxContainer_Backpack

@onready var wearable_editor = %WearableEditor
@onready var emote_editor = %EmoteEditor

@onready var container_navbar = %PanelContainer_Navbar
@onready var button_emotes = %Button_Emotes
@onready var button_wearables = %Button_Wearables


# gdlint:ignore = async-function-name
func _ready():
	for category in Wearables.Categories.ALL_CATEGORIES:
		var button_group = ButtonGroup.new()
		button_group.allow_unpress = _can_unequip(category)
		wearable_button_group_per_category[category] = button_group

	if hide_navbar:
		container_navbar.hide()

	emote_editor.avatar = avatar_preview.avatar
	emote_editor.set_new_emotes.connect(self._on_set_new_emotes)
	wearable_editor.show()
	emote_editor.hide()

	container_backpack.hide()
	backpack_loading.show()

	skin_color_picker.hide()

	# Setup blacklist change timer
	blacklist_deploy_timer = Timer.new()
	blacklist_deploy_timer.wait_time = 5.0
	blacklist_deploy_timer.one_shot = true
	blacklist_deploy_timer.timeout.connect(self._on_blacklist_deploy_timer_timeout)
	add_child(blacklist_deploy_timer)

	# Connect to blacklist changes
	Global.social_blacklist.blacklist_changed.connect(self._on_blacklist_changed)

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

	# Load all remote wearables that you own...
	var remote_wearables = await WearableRequest.async_request_all_wearables()
	if remote_wearables != null:
		for wearable_item in remote_wearables.elements:
			wearable_data[wearable_item.urn] = null

	var promise = Global.content_provider.fetch_wearables(
		wearable_data.keys(), Global.realm.get_profile_content_url()
	)
	await PromiseUtils.async_all(promise)

	for wearable_id in wearable_data:
		var wearable = Global.content_provider.get_wearable(wearable_id)
		wearable_data[wearable_id] = wearable
		if wearable == null:
			printerr("Error loading wearable_id ", wearable_id)
	_update_visible_categories()

	request_update_avatar = true

	container_backpack.show()
	backpack_loading.hide()

	request_show_wearables = true

	# responsive
	if get_window() != null:
		get_window().size_changed.connect(self._on_size_changed)
	_on_size_changed.call_deferred()


func _on_size_changed():
	var window_size: Vector2i = DisplayServer.window_get_size()
	var portrait = window_size.x < window_size.y
	var right_editor_container: MarginContainer = %RightEditorContainer
	if portrait:
		right_editor_container.add_theme_constant_override("margin_top", 0)
		right_editor_container.add_theme_constant_override("margin_left", 0)
		right_editor_container.add_theme_constant_override("margin_right", 0)
		right_editor_container.add_theme_constant_override("margin_bottom", 0)
	else:
		right_editor_container.add_theme_constant_override("margin_top", 10)
		right_editor_container.add_theme_constant_override("margin_left", 20)
		right_editor_container.add_theme_constant_override("margin_right", 20)
		right_editor_container.add_theme_constant_override("margin_bottom", 10)


func _update_visible_categories():
	var showed_subcategories: int = 0
	var first_wearable_filter_button: WearableFilterButton = null
	for wearable_filter_button: WearableFilterButton in wearable_filter_buttons:
		var category = wearable_filter_button.get_category_name()
		var filter_categories: Array = Wearables.Categories.MAIN_CATEGORIES.get(
			main_category_selected
		)
		var category_is_visible: bool = (
			filter_categories != null and filter_categories.has(category)
		)
		wearable_filter_button.visible = category_is_visible
		if category_is_visible:
			showed_subcategories += 1
			if first_wearable_filter_button == null:
				first_wearable_filter_button = wearable_filter_button

	container_sub_categories.set_visible(showed_subcategories >= 2)
	if first_wearable_filter_button:
		first_wearable_filter_button.set_pressed(true)


func _on_set_new_emotes(emotes_urns: PackedStringArray):
	Global.mutable_avatar.set_emotes(emotes_urns)
	# Don't trigger request_update_avatar - emotes are loaded separately by the emote controller
	# and don't require reloading the avatar mesh/wearables


func _physics_process(_delta):
	if request_update_avatar:
		request_update_avatar = false
		_async_update_avatar()

	if request_show_wearables:
		request_show_wearables = false
		_show_wearables()


func _set_avatar_loading() -> int:
	avatar_preview.hide()
	avatar_loading.show()
	avatar_loading_counter += 1
	return avatar_loading_counter


func _unset_avatar_loading(current: int):
	if current != avatar_loading_counter:
		return
	avatar_loading.hide()
	avatar_preview.show()


func _async_update_avatar():
	Global.mutable_profile.set_avatar(Global.mutable_avatar)

	var loading_id := _set_avatar_loading()
	await avatar_preview.avatar.async_update_avatar_from_profile(Global.mutable_profile)
	_unset_avatar_loading(loading_id)


func _load_filtered_data(filter: String):
	if Global.mutable_avatar == null:
		return

	filtered_data = []
	current_filter = filter
	for wearable_id in wearable_data:
		var wearable = wearable_data[wearable_id]
		if wearable != null:
			var is_filter_all = filter == "all"
			if wearable.get_category() == filter or is_filter_all:
				var is_body_shape = wearable.get_category() == "body_shape"
				var is_equipable = Wearables.can_equip(
					wearable, Global.mutable_avatar.get_body_shape()
				)
				var is_base_wearable = Wearables.is_base_wearable(wearable_id)
				var can_use = (
					(is_equipable and (!is_base_wearable or !only_collectibles))
					or (is_body_shape and !is_filter_all)
				)
				if can_use:
					filtered_data.push_back(wearable_id)

	request_show_wearables = true


func _can_unequip(category: String) -> bool:
	return (
		category != Wearables.Categories.BODY_SHAPE
		and category != Wearables.Categories.EYES
		and category != Wearables.Categories.MOUTH
	)


func _show_wearables():
	for child in grid_container_wearables_list.get_children():
		child.queue_free()

	control_no_items.visible = filtered_data.is_empty()
	grid_container_wearables_list.visible = not filtered_data.is_empty()

	for wearable_id in filtered_data:
		var wearable_item = WEARABLE_ITEM_INSTANTIABLE.instantiate()
		var wearable = wearable_data[wearable_id]
		grid_container_wearables_list.add_child(wearable_item)
		wearable_item.button_group = wearable_button_group_per_category.get(wearable.get_category())
		wearable_item.async_set_wearable(wearable)

		# Connect signals
		wearable_item.equip.connect(self._on_wearable_equip.bind(wearable_id))
		wearable_item.unequip.connect(self._on_wearable_unequip.bind(wearable_id))

		# Check if the item is equipped
		var is_wearable_pressed = (
			Global.mutable_avatar.get_wearables().has(wearable_id)
			or Global.mutable_avatar.get_body_shape() == wearable_id
		)
		wearable_item.set_pressed_no_signal(is_wearable_pressed)
		wearable_item.set_equiped(is_wearable_pressed)


func _on_main_category_filter_type(type: String):
	main_category_selected = type
	_update_visible_categories()


func _on_wearable_filter_button_filter_type(type):
	_load_filtered_data(type)
	avatar_preview.focus_camera_on(type)

	var should_hide = false
	if type == Wearables.Categories.BODY_SHAPE:
		skin_color_picker.color_target = skin_color_picker.ColorTarget.SKIN
		skin_color_picker.set_color(Global.mutable_avatar.get_skin_color())
	elif type == Wearables.Categories.HAIR or type == Wearables.Categories.FACIAL_HAIR:
		skin_color_picker.color_target = skin_color_picker.ColorTarget.HAIR
		skin_color_picker.set_color(Global.mutable_avatar.get_hair_color())
	elif type == Wearables.Categories.EYES:
		skin_color_picker.color_target = skin_color_picker.ColorTarget.EYE
		skin_color_picker.set_color(Global.mutable_avatar.get_eyes_color())
	else:
		should_hide = true

	if should_hide:
		skin_color_picker.hide()
	else:
		skin_color_picker.show()


func _on_wearable_equip(wearable_id: String):
	var desired_wearable = wearable_data[wearable_id]
	var category = desired_wearable.get_category()

	if category == Wearables.Categories.BODY_SHAPE:
		var current_body_shape_id := Global.mutable_avatar.get_body_shape()
		var new_body_shape_id := wearable_id
		if current_body_shape_id != new_body_shape_id:
			avatar_wearables_body_shape_cache[current_body_shape_id] = (
				Global.mutable_avatar.get_wearables().duplicate()
			)

			Global.mutable_avatar.set_body_shape(new_body_shape_id)
			var default_wearables: Dictionary = Wearables.DefaultWearables.BY_BODY_SHAPES.get(
				new_body_shape_id
			)
			var new_avatar_wearables = avatar_wearables_body_shape_cache.get(new_body_shape_id, [])
			if new_avatar_wearables.is_empty():
				new_avatar_wearables = default_wearables.values()

			Global.mutable_avatar.set_wearables(PackedStringArray(new_avatar_wearables))
	else:
		var new_avatar_wearables := Global.mutable_avatar.get_wearables()
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
		Global.mutable_avatar.set_wearables(new_avatar_wearables)

	request_update_avatar = true


func _on_wearable_unequip(wearable_id: String):
	var desired_wearable = wearable_data[wearable_id]
	var category = desired_wearable.get_category()

	if category == Wearables.Categories.BODY_SHAPE:
		# can not unequip a body shape
		return

	var new_avatar_wearables := Global.mutable_avatar.get_wearables()
	var index = new_avatar_wearables.find(wearable_id)
	if index != -1:
		new_avatar_wearables.remove_at(index)

	Global.mutable_avatar.set_wearables(new_avatar_wearables)
	request_update_avatar = true


func _on_button_logout_pressed():
	Global.comms.disconnect(true)


func _on_color_picker_panel_pick_color(color: Color):
	match skin_color_picker.color_target:
		skin_color_picker.ColorTarget.EYE:
			Global.mutable_avatar.set_eyes_color(color)
		skin_color_picker.ColorTarget.SKIN:
			Global.mutable_avatar.set_skin_color(color)
		skin_color_picker.ColorTarget.HAIR:
			Global.mutable_avatar.set_hair_color(color)

	skin_color_picker.set_color(color)
	avatar_preview.avatar.update_colors(
		Global.mutable_avatar.get_eyes_color(),
		Global.mutable_avatar.get_skin_color(),
		Global.mutable_avatar.get_hair_color()
	)


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
				current_color = Global.mutable_avatar.get_eyes_color()
			skin_color_picker.ColorTarget.SKIN:
				color_picker_panel.color_type = color_picker_panel.ColorTargetType.SKIN
				current_color = Global.mutable_avatar.get_skin_color()
			skin_color_picker.ColorTarget.HAIR:
				color_picker_panel.color_type = color_picker_panel.ColorTargetType.OTHER
				current_color = Global.mutable_avatar.get_hair_color()

		color_picker_panel.custom_popup(rect, current_color)


func _on_color_picker_panel_hided():
	skin_color_picker.set_pressed(false)


func _on_rich_text_box_open_marketplace_meta_clicked(_meta):
	Global.open_url("https://decentraland.org/marketplace/browse?section=wearables")


func _on_button_wearables_pressed():
	avatar_preview.avatar.emote_controller.stop_emote()
	wearable_editor.show()
	emote_editor.hide()


func _on_button_emotes_pressed():
	show_emotes()


func show_emotes() -> void:
	avatar_preview.focus_camera_on(Wearables.Categories.BODY_SHAPE)
	wearable_editor.hide()
	emote_editor.show()


func press_button_emotes() -> void:
	button_emotes.set_pressed_no_signal(true)
	button_wearables.set_pressed_no_signal(false)


func _on_check_box_only_collectibles_toggled(toggled_on):
	emote_editor.async_set_only_collectibles(toggled_on)
	only_collectibles = toggled_on
	_load_filtered_data(current_filter)


func _exit_tree():
	# Clean up timer and disconnect signals
	if blacklist_deploy_timer:
		blacklist_deploy_timer.stop()
		blacklist_deploy_timer.queue_free()

	if Global.social_blacklist.blacklist_changed.is_connected(self._on_blacklist_changed):
		Global.social_blacklist.blacklist_changed.disconnect(self._on_blacklist_changed)


func _on_blacklist_changed():
	# Don't trigger deployment if we're loading a profile from server
	if is_loading_profile:
		return
	# Reset the timer if it's already running
	blacklist_deploy_timer.stop()
	blacklist_deploy_timer.start()


func _on_blacklist_deploy_timer_timeout():
	# Update the mutable profile with current blacklist before deploying
	Global.mutable_profile.set_blocked(Global.social_blacklist.get_blocked_list())
	Global.mutable_profile.set_muted(Global.social_blacklist.get_muted_list())
	# Deploy without incrementing version for blacklist changes (ADR-290: no snapshots)
	ProfileService.async_deploy_profile_with_version_control(Global.mutable_profile, false)
