extends Control

signal set_new_emotes(emotes_run: PackedStringArray)
signal emote_grid_selected(emote_name: String)
signal emote_equipped(equipped: bool)

const EMOTE_SQUARE_ITEM = preload("res://src/ui/components/emotes/emote_square_item.tscn")

@export var avatar: Avatar = null:
	set(new_value):
		avatar = new_value
		avatar.avatar_loaded.connect(self._on_avatar_loaded)

var last_equipped_emote_urn: String = ""
var avatar_emote_items: Array[EmoteEditorItem] = []
var all_emote_items: Array[EmoteItemUi] = []
var current_selected_index: int = -1

var _currently_selected_emote_item: EmoteItemUi = null
var _equipped_emote_urns: PackedStringArray = []
var _only_collectibles: bool = false

@onready var container_avatar_emotes = %VBoxContainer_AvatarEmotes
@onready var container_all_emotes = %GridContainer_Emotes
@onready var button_group_avatar_emotes = ButtonGroup.new()
@onready var button_group_all_emotes = ButtonGroup.new()
@onready var scroll_container_grid: ScrollContainer = %ScrollContainer_Grid
@onready var inner_margin_container: MarginContainer = %InnerMarginContainer
@onready var emote_grid_outter_margin_container: MarginContainer = %EmoteGridOutterMarginContainer
@onready var external_margin_container: MarginContainer = %ExternalMarginContainer
@onready var outter_margin_container: MarginContainer = %OutterMarginContainer


func _ready():
	var first_button: BaseButton = null
	for child in container_avatar_emotes.get_children():
		if child is EmoteEditorItem:
			if first_button == null:
				first_button = child
			child.button_group = button_group_avatar_emotes
			child.use_equipped_border = true
			var index = avatar_emote_items.size()
			child.select_emote.connect(self._on_emote_editor_item_select_emote.bind(index))
			child.clear_emote.connect(self._on_emote_editor_item_clear_emote.bind(index))
			avatar_emote_items.push_back(child)

	first_button.set_pressed(true)
	current_selected_index = 0

	button_group_all_emotes.allow_unpress = false

	_async_load_emotes()


func async_set_only_collectibles(new_state: bool):
	_only_collectibles = new_state
	await _async_load_emotes()


func _add_default_emotes():
	for emote_urn in Emotes.DEFAULT_EMOTE_NAMES.keys():
		var emote_item: EmoteItemUi = EMOTE_SQUARE_ITEM.instantiate()
		emote_item.button_group = button_group_all_emotes
		emote_item.async_load_from_urn(emote_urn)
		emote_item.play_emote.connect(self._on_emote_item_play_emote.bind(emote_item))
		emote_item.emote_name_ready.connect(self.emote_grid_selected.emit)
		container_all_emotes.add_child(emote_item)
		all_emote_items.push_back(emote_item)


func _async_load_remote_emotes():
	var remote_emotes = await WearableRequest.async_request_all_emotes()
	if remote_emotes != null:
		remote_emotes.elements.sort_custom(func(a, b): return a.transferet_at > b.transferet_at)
		var count := 0
		for emote in remote_emotes.elements:
			var emote_item: EmoteItemUi = EMOTE_SQUARE_ITEM.instantiate()
			emote_item.button_group = button_group_all_emotes
			emote_item.async_load_from_urn(emote.urn)
			emote_item.play_emote.connect(self._on_emote_item_play_emote.bind(emote_item))
			emote_item.emote_name_ready.connect(self.emote_grid_selected.emit)
			container_all_emotes.add_child(emote_item)
			all_emote_items.push_back(emote_item)
			count += 1
			if count % 10 == 0:
				await get_tree().process_frame

	if not _only_collectibles:
		_add_default_emotes()
	_update_grid_equipped_state()


func _async_load_emotes():
	# Clear
	for child in container_all_emotes.get_children():
		container_all_emotes.remove_child(child)
		child.queue_free()

	all_emote_items.clear()

	await _async_load_remote_emotes()
	_sync_grid_selection()


func _on_avatar_loaded():
	_equipped_emote_urns = avatar.avatar_data.get_emotes()

	for i in range(avatar_emote_items.size()):
		# get_emotes() always returns 10 emotes, but just in case
		if i >= _equipped_emote_urns.size():
			# Set default or
			continue

		var emote_editor_item: EmoteEditorItem = avatar_emote_items[i]
		emote_editor_item.async_load_from_urn(_equipped_emote_urns[i], i)  # Forget await

	_update_grid_equipped_state()
	_sync_grid_selection()


func _normalize_emote_urn(urn: String) -> String:
	if Emotes.is_base_emote_urn(urn):
		return Emotes.get_base_emote_id_from_urn(urn)
	return urn


func _on_emote_editor_item_select_emote(_emote_urn: String, index: int):
	if is_instance_valid(avatar) and not _emote_urn.is_empty():
		avatar.async_play_emote(_emote_urn)
	current_selected_index = index
	_sync_grid_selection()


func _on_emote_item_play_emote(_emote_urn: String, emote_item: EmoteItemUi):
	if emote_item == _currently_selected_emote_item:
		_on_emote_item_equip_emote(not emote_item._is_equipped, _emote_urn, emote_item)
		return
	_currently_selected_emote_item = emote_item
	avatar.async_play_emote(_emote_urn)
	emote_grid_selected.emit(emote_item.emote_name)
	var normalized_urn := _normalize_emote_urn(_emote_urn)
	for i in range(_equipped_emote_urns.size()):
		if _normalize_emote_urn(_equipped_emote_urns[i]) == normalized_urn:
			current_selected_index = i
			avatar_emote_items[i].set_pressed(true)
			return


func _on_emote_item_equip_emote(equip: bool, _emote_urn: String, emote_item: EmoteItemUi):
	if not equip:
		_clear_slot(current_selected_index)
		emote_item.set_pressed(true)
		emote_equipped.emit(false)
		return
	var emote_urns = avatar.avatar_data.get_emotes()
	emote_urns[current_selected_index] = _emote_urn
	avatar.avatar_data.set_emotes(emote_urns)
	set_new_emotes.emit(emote_urns)
	last_equipped_emote_urn = _emote_urn
	_on_avatar_loaded()
	emote_equipped.emit(true)


func _on_emote_editor_item_clear_emote(index: int):
	_clear_slot(index)


func _clear_slot(index: int) -> void:
	if index < 0 or index >= _equipped_emote_urns.size():
		return
	_equipped_emote_urns[index] = ""
	var emote_urns = avatar.avatar_data.get_emotes()
	emote_urns[index] = ""
	avatar.avatar_data.set_emotes(emote_urns)
	set_new_emotes.emit(emote_urns)
	# Update the VBox slot display without re-emitting clear_emote
	avatar_emote_items[index].set_empty()
	_update_grid_equipped_state()
	_sync_grid_selection()


func _update_grid_equipped_state():
	var normalized_equipped: Array[String] = []
	for urn in _equipped_emote_urns:
		var normalized := _normalize_emote_urn(urn)
		if not normalized.is_empty():
			normalized_equipped.append(normalized)
	for emote_item in all_emote_items:
		if emote_item is EmoteItemUi:
			var item_urn := _normalize_emote_urn(emote_item.emote_urn)
			emote_item.set_equipped(not item_urn.is_empty() and item_urn in normalized_equipped)


func _sync_grid_selection():
	if current_selected_index < 0 or current_selected_index >= _equipped_emote_urns.size():
		return
	var selected_urn = _normalize_emote_urn(_equipped_emote_urns[current_selected_index])
	if selected_urn.is_empty():
		# Empty slot — unpress the currently pressed grid item (if any)
		for emote_item in all_emote_items:
			if emote_item is EmoteItemUi and emote_item.button_pressed:
				emote_item.set_pressed(false)
		return
	for emote_item in all_emote_items:
		if emote_item is EmoteItemUi:
			if _normalize_emote_urn(emote_item.emote_urn) == selected_urn:
				emote_item.set_pressed(true)
				_currently_selected_emote_item = emote_item
				if scroll_container_grid != null:
					_scroll_to_item_with_margin(emote_item, 20)
				if not emote_item.emote_name.is_empty():
					emote_grid_selected.emit(emote_item.emote_name)
				break


func _scroll_to_item_with_margin(item: Control, margin: float) -> void:
	var item_top := item.get_global_rect().position.y
	var item_bottom := item_top + item.size.y
	var scroll_top := scroll_container_grid.get_global_rect().position.y
	var scroll_bottom := scroll_top + scroll_container_grid.size.y

	if item_top < scroll_top + margin:
		var offset := scroll_top + margin - item_top
		scroll_container_grid.scroll_vertical -= int(offset)
	elif item_bottom > scroll_bottom - margin:
		var offset := item_bottom - (scroll_bottom - margin)
		scroll_container_grid.scroll_vertical += int(offset)


func _on_visibility_changed() -> void:
	if not is_node_ready():
		return
	if scroll_container_grid != null:
		scroll_container_grid.scroll_vertical = 0


func _on_landscape() -> void:
	outter_margin_container.add_theme_constant_override("margin_right", 48)
	outter_margin_container.add_theme_constant_override("margin_left", 60)
	inner_margin_container.add_theme_constant_override("margin_right", -20)
	inner_margin_container.add_theme_constant_override("margin_left", -20)
	emote_grid_outter_margin_container.add_theme_constant_override("margin_top", 0)
	external_margin_container.add_theme_constant_override("margin_top", 0)
	container_all_emotes.columns = 2
	for emote_item in avatar_emote_items:
		emote_item.custom_minimum_size = Vector2(138, 138)
