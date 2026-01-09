extends Control

signal emote_wheel_opened
signal emote_wheel_closed

@export var avatar_node: Avatar = null:
	set(value):
		if value != avatar_node:  # Prevent redundant assignments
			if (
				avatar_node != null
				and avatar_node.avatar_loaded.is_connected(self._on_avatar_loaded)
			):
				avatar_node.avatar_loaded.disconnect(self._on_avatar_loaded)

			avatar_node = value
			avatar_node.avatar_loaded.connect(self._on_avatar_loaded)
	get():
		return avatar_node

var emote_items: Array[EmoteItemUi] = []

var last_selected_emote_urn: String = ""

@onready var emote_wheel_container = %EmoteWheelContainer
@onready var label_emote_name = %Label_EmoteName
@onready var control_wheel: Control = %Control_Wheel
@onready var button_emotes: Button = $Button_Emotes


func _ready():
	control_wheel.hide()

	for child in emote_wheel_container.get_children():
		if child is EmoteItemUi:
			child.play_emote.connect(self._on_play_emote)
			child.select_emote.connect(self._on_select_emote.bind(child))
			emote_items.push_back(child)

	if avatar_node != null:
		avatar_node.avatar_loaded.connect(self._on_avatar_loaded)

	# Load default emotes as initial state
	_update_wheel(Emotes.DEFAULT_EMOTE_NAMES.keys())


func _on_avatar_loaded():
	var emote_urns = avatar_node.avatar_data.get_emotes()
	_update_wheel(emote_urns)


func _update_wheel(emote_urns: Array):
	for i in range(emote_items.size()):
		# get_emotes() always returns 10 emotes, but just in case
		if i >= emote_urns.size():
			# Set default or
			continue

		var emote_item: EmoteItemUi = emote_items[i]
		emote_item.async_load_from_urn(emote_urns[i], i)  # Forget await


func _on_play_emote(emote_urn: String):
	close()
	if avatar_node != null:
		var emote_controller = avatar_node.emote_controller
		# Use async_play_emote to ensure base emotes are loaded from remote
		emote_controller.async_play_emote(emote_urn)
		Global.comms.send_emote(emote_urn)
	else:
		printerr("No avatar node in EmoteWheel!")


func _on_select_emote(selected: bool, emote_urn: String, child: EmoteItemUi):
	if emote_urn == last_selected_emote_urn and selected:
		return

	if !selected:
		label_emote_name.text = "Emotes"
		last_selected_emote_urn = ""
		return

	last_selected_emote_urn = emote_urn
	label_emote_name.text = child.emote_name
	UiSounds.play_sound("backpack_item_highlight")


func close(play_sound: bool = false) -> void:
	if not control_wheel.visible:
		return
	control_wheel.hide()
	emote_wheel_closed.emit()
	Global.explorer_grab_focus()
	if play_sound:
		UiSounds.play_sound("widget_emotes_close")
	if button_emotes != null and button_emotes.button_pressed:
		button_emotes.set_pressed_no_signal(false)


func open() -> void:
	if control_wheel.visible:
		return
	control_wheel.show()
	UiSounds.play_sound("widget_emotes_open")
	emote_wheel_opened.emit()
	grab_focus()
	Global.release_mouse()


func _on_control_wheel_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		UiSounds.play_sound("widget_emotes_close")
		close()


func _on_button_toggled(toggled_on: bool) -> void:
	if toggled_on:
		open()
	else:
		close(true)


func _on_button_edit_pressed() -> void:
	Global.open_backpack.emit(true)
	Global.send_haptic_feedback()
	close(false)
