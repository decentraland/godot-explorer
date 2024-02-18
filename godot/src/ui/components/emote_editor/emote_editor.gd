extends Control

@export var avatar: Avatar = null:
	set(new_value):
		avatar = new_value
		avatar.avatar_loaded.connect(self._on_avatar_loaded)

@onready var emote_container = %VBoxContainer_EmoteContainer

var emote_items: Array[EmoteEditorItem] = []
@onready var button_group = ButtonGroup.new()

func _ready():
	var first_button = null
	for child in emote_container.get_children():
		if child is EmoteEditorItem:
			if first_button == null:
				first_button = child
			child.button_group = button_group
			emote_items.push_back(child)
	
	first_button.set_pressed(true)

func _on_avatar_loaded():
	var emote_urns = []
	if avatar == null:
		# test
		printerr("No avatar presented in EmoteEditor. Using default just for testing.")
		emote_urns = Emotes.DEFAULT_EMOTE_NAMES.keys()
	else:
		emote_urns = avatar.avatar_data.get_emotes()

	for i in range(emote_items.size()):
		# get_emotes() always returns 10 emotes, but just in case
		if i >= emote_urns.size():
			# Set default or
			continue

		var emote_item: EmoteEditorItem = emote_items[i]
		emote_item.async_load_from_urn(emote_urns[i], i) # Forget await
