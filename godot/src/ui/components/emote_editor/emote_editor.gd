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
			avatar_emote_items.push_back(child)

	first_button.set_pressed(true)
	current_selected_index = 0

	_async_load_emotes()


func async_set_only_collectibles(new_state: bool):
	_only_collectibles = new_state
	await _async_load_emotes()


func _async_add_remote_emotes(page_number: int):
	var remote_emotes = await EmotesRequest.async_request_emotes(page_number, PAGE_SIZE)
	if remote_emotes != null:
		_can_load_more = remote_emotes.elements.size() == PAGE_SIZE
		for emotes in remote_emotes.elements:
			var emote_item: EmoteItemUi = EMOTE_SQUARE_ITEM.instantiate()
			emote_item.button_group = button_group_all_emotes
			emote_item.async_load_from_urn(emotes.urn)
			emote_item.play_emote.connect(self._on_emote_item_play_emote)
			container_all_emotes.add_child(emote_item)
			all_emote_items.push_back(emote_item)


func _async_load_emotes():
	# Clear
	for child in container_all_emotes.get_children():
		container_all_emotes.remove_child(child)

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


func _on_avatar_loaded():
	var emote_urns = avatar.avatar_data.get_emotes()

	for i in range(avatar_emote_items.size()):
		# get_emotes() always returns 10 emotes, but just in case
		if i >= emote_urns.size():
			# Set default or
			continue

		var emote_editor_item: EmoteEditorItem = avatar_emote_items[i]
		emote_editor_item.async_load_from_urn(emote_urns[i], i)  # Forget await


func _on_emote_editor_item_select_emote(_emote_urn: String, index: int):
	if is_instance_valid(avatar):
		avatar.async_play_emote(_emote_urn)
	current_selected_index = index

	for emote_item in all_emote_items:
		if emote_item is EmoteItemUi:
			if emote_item.emote_urn == _emote_urn:
				emote_item.set_pressed(true)
				break


func _on_emote_item_play_emote(_emote_urn: String):
	avatar.async_play_emote(_emote_urn)
	var emote_urns = avatar.avatar_data.get_emotes()
	emote_urns[current_selected_index] = _emote_urn
	set_new_emotes.emit(emote_urns)
	_on_avatar_loaded()


func _async_on_scrollbar_value_changed(new_value):
	if _can_load_more:
		var max_value = vscroll_bar.max_value
		var end = max_value <= (new_value + scroll_container.size.y)
		if end:
			_can_load_more = false  # avoid processing until the add finishes
			_last_loaded_page += 1
			await _async_add_remote_emotes(_last_loaded_page)
