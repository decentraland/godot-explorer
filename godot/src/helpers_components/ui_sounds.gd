class_name UISounds
extends Node

# Should have the same name without the .wav from res://assets/sfx/ui/{name}.wav
const _SOUNDS_TO_LOAD = [
	&"toggle_enable",  # Checkbox/Optionbox on/off
	&"toggle_disable",
	&"generic_button_press",
	&"generic_button_release",
	&"generic_button_hover",
	&"backpack_item_equip",
	&"backpack_item_highlight",
	&"widget_chat_open",  # Open chat
	&"widget_chat_close",  # Close chat
	&"widget_chat_message_private_send",  # Send message
	&"notification_chatmessage_public_appear",  # Message received
	&"notification_chatmessage_private_appear",  # None
	&"voice_chat_mic_on",  # Start talking
	&"voice_chat_mic_off",
	&"mainmenu_widget_open",  # Open menu
	&"mainmenu_widget_close",  # Close menu
	&"mainmenu_tab_switch",  # Menu tab switch
	&"inputfield_entertext",  # On write texts
	&"mainmenu_tile_highlight",  # Highlight teleport or something
	&"widget_emotes_close",
	&"widget_emotes_highlight",
	&"widget_emotes_open",
	&"ui_fade_in",
	&"ui_fade_out",
]

var _sounds: Dictionary = {}


# Called when the node enters the scene tree for the first time.
func _ready():
	# set up audio streams
	for sound_to_load: String in _SOUNDS_TO_LOAD:
		var path: String = "res://assets/sfx/ui/%s.wav" % sound_to_load

		var audio_stream: AudioStreamPlayer = AudioStreamPlayer.new()
		audio_stream.stream = load(path)
		audio_stream.bus = &"UI"
		_sounds[sound_to_load] = audio_stream

		add_child(audio_stream)


func install_audio(node: Node):
	if node.has_meta("disable_ui_sounds"):
		return

	var sound_added = true

	if node is WearableItem:
		node.equip.connect(self.play_sound.bind(&"backpack_item_equip"))
	elif node is EmoteItemUi:
		node.play_emote.connect(func(_data): self.play_sound(&"backpack_item_equip"))
	elif node is EmoteEditorItem:
		node.select_emote.connect(func(_data): self.play_sound(&"mainmenu_tile_highlight"))
	elif node is PlaceItem:
		node.item_pressed.connect(func(_data): play_sound(&"mainmenu_tile_highlight"))
	elif node is CheckBox or node is OptionButton:
		node.mouse_entered.connect(self.play_sound.bind(&"generic_button_hover"))
		node.toggled.connect(
			func(toggled_on): play_sound(&"toggle_enable" if toggled_on else &"toggle_disable")
		)
	elif node is Button:
		node.mouse_entered.connect(self.play_sound.bind(&"generic_button_hover"))
		node.button_down.connect(self.play_sound.bind(&"generic_button_press"))
		node.button_up.connect(self.play_sound.bind(&"generic_button_release"))
	elif node is LineEdit:
		node.text_changed.connect(func(_new_text): play_sound(&"inputfield_entertext"))
	else:
		sound_added = false

	if sound_added:
		node.set_meta("disable_ui_sounds", true)


func install_audio_recusirve(node: Node):
	install_audio(node)

	for child in node.get_children():
		# recursion
		install_audio_recusirve(child)


func _play_sound_toggle(name_on: StringName, name_off: StringName, toggled_on: bool):
	play_sound(name_on if toggled_on else name_off)


func play_sound(sound_name: StringName):
	var audio_stream: AudioStreamPlayer = _sounds.get(sound_name)
	if is_instance_valid(audio_stream):
		audio_stream.play()
	else:
		printerr("Audio %s doesn't exists.", sound_name)
