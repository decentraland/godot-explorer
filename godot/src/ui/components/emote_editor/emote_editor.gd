extends Control

signal set_new_emotes(emotes_run: PackedStringArray)

const PAGE_SIZE: int = 16
const EMOTE_SQUARE_ITEM = preload("res://src/ui/components/emotes/emote_square_item.tscn")

@export var avatar: Avatar = null:
	set(new_value):
		avatar = new_value
		avatar.avatar_loaded.connect(self._on_avatar_loaded)

var avatar_emote_items: Array[EmoteEditorItem] = []
var all_emote_items: Array[EmoteItemUi] = []
var current_selected_index: int = -1
var vscroll_bar: VScrollBar = null
var _equipped_emote_urns: PackedStringArray = []

var _only_collectibles: bool = false
var _can_load_more = false
var _last_loaded_page = 0

@onready var container_avatar_emotes = %VBoxContainer_AvatarEmotes
@onready var container_all_emotes = %GridContainer_Emotes
@onready var button_group_avatar_emotes = ButtonGroup.new()
@onready var button_group_all_emotes = ButtonGroup.new()
@onready var scroll_container = %ScrollContainer


func _ready():
	vscroll_bar = scroll_container.get_v_scroll_bar()
	vscroll_bar.value_changed.connect(self._async_on_scrollbar_value_changed)
	var first_button: BaseButton = null
	for child in container_avatar_emotes.get_children():
		if child is EmoteEditorItem:
			if first_button == null:
				first_button = child
			child.button_group = button_group_avatar_emotes
			var index = avatar_emote_items.size()
			child.select_emote.connect(self._on_emote_editor_item_select_emote.bind(index))
			child.clear_emote.connect(self._on_emote_editor_item_clear_emote.bind(index))
			avatar_emote_items.push_back(child)

	first_button.set_pressed(true)
	current_selected_index = 0

	button_group_all_emotes.allow_unpress = true

	_async_load_emotes()


func async_set_only_collectibles(new_state: bool):
	_only_collectibles = new_state
	await _async_load_emotes()


func _async_add_remote_emotes(page_number: int):
	var remote_emotes = await WearableRequest.async_request_emotes(page_number, PAGE_SIZE)
	if remote_emotes != null:
		_can_load_more = remote_emotes.elements.size() == PAGE_SIZE
		for emotes in remote_emotes.elements:
			var emote_item: EmoteItemUi = EMOTE_SQUARE_ITEM.instantiate()
			emote_item.button_group = button_group_all_emotes
			emote_item.async_load_from_urn(emotes.urn)
			emote_item.play_emote.connect(self._on_emote_item_play_emote)
			container_all_emotes.add_child(emote_item)
			all_emote_items.push_back(emote_item)
	_update_grid_equipped_state()


func _async_load_emotes():
	# Clear
	for child in container_all_emotes.get_children():
		container_all_emotes.remove_child(child)
		child.queue_free()

	all_emote_items.clear()

	if not _only_collectibles:
		# Load default emotes
		for emote_urn in Emotes.DEFAULT_EMOTE_NAMES.keys():
			var emote_item: EmoteItemUi = EMOTE_SQUARE_ITEM.instantiate()
			emote_item.button_group = button_group_all_emotes
			emote_item.async_load_from_urn(emote_urn)
			emote_item.play_emote.connect(self._on_emote_item_play_emote)
			container_all_emotes.add_child(emote_item)
			all_emote_items.push_back(emote_item)

	_last_loaded_page = 1
	await _async_add_remote_emotes(_last_loaded_page)
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


func _on_emote_item_play_emote(_emote_urn: String):
	# Tapping the already-equipped emote in the current slot clears it
	if (
		current_selected_index >= 0
		and current_selected_index < _equipped_emote_urns.size()
		and (
			_normalize_emote_urn(_equipped_emote_urns[current_selected_index])
			== _normalize_emote_urn(_emote_urn)
		)
	):
		_clear_slot(current_selected_index)
		return

	avatar.async_play_emote(_emote_urn)
	var emote_urns = avatar.avatar_data.get_emotes()
	emote_urns[current_selected_index] = _emote_urn
	# Update both the avatar's data (for display) and emit signal (for profile saving)
	avatar.avatar_data.set_emotes(emote_urns)
	set_new_emotes.emit(emote_urns)
	_on_avatar_loaded()


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
				break


func _async_on_scrollbar_value_changed(new_value):
	if _can_load_more:
		var max_value = vscroll_bar.max_value
		var end = max_value <= (new_value + scroll_container.size.y)
		if end:
			_can_load_more = false  # avoid processing until the add finishes
			_last_loaded_page += 1
			await _async_add_remote_emotes(_last_loaded_page)
