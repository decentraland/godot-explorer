extends Control

signal set_new_emotes(emotes_run: PackedStringArray)

const EMOTE_SQUARE_ITEM = preload("res://src/ui/components/emotes/emote_square_item.tscn")

@export var avatar: Avatar = null:
	set(new_value):
		avatar = new_value
		avatar.avatar_loaded.connect(self._on_avatar_loaded)

var avatar_emote_items: Array[EmoteEditorItem] = []
var all_emote_items: Array[EmoteItemUI] = []
var current_selected_index: int = -1

@onready var container_avatar_emotes = %VBoxContainer_AvatarEmotes
@onready var container_all_emotes = %GridContainer_Emotes
@onready var button_group_avatar_emotes = ButtonGroup.new()
@onready var button_group_all_emotes = ButtonGroup.new()


func _ready():
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

	# Load default emotes
	for emote_urn in Emotes.DEFAULT_EMOTE_NAMES.keys():
		var emote_item: EmoteItemUI = EMOTE_SQUARE_ITEM.instantiate()
		emote_item.button_group = button_group_all_emotes
		emote_item.async_load_from_urn(emote_urn)
		emote_item.play_emote.connect(self._on_emote_item_play_emote)
		container_all_emotes.add_child(emote_item)
		all_emote_items.push_back(emote_item)

	# TODO: Load remote emotes


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
		if emote_item is EmoteItemUI:
			if emote_item.emote_urn == _emote_urn:
				emote_item.set_pressed(true)
				break


func _on_emote_item_play_emote(_emote_urn: String):
	avatar.async_play_emote(_emote_urn)
	var emote_urns = avatar.avatar_data.get_emotes()
	emote_urns[current_selected_index] = _emote_urn
	set_new_emotes.emit(emote_urns)
	_on_avatar_loaded()
