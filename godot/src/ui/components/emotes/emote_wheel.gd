extends Control

@export var avatar_node: Avatar = null

var emote_items: Array[EmoteItemUi] = []

var last_selected_emote_urn: String = ""

@onready var emote_wheel_container = %EmoteWheelContainer
@onready var label_emote_name = %Label_EmoteName
@onready var label_for_desktop = %Label_ForDesktop


func _ready():
	self.hide()
	for child in emote_wheel_container.get_children():
		if child is EmoteItemUi:
			child.play_emote.connect(self._on_play_emote)
			child.select_emote.connect(self._on_select_emote.bind(child))
			emote_items.push_back(child)

	label_for_desktop.set_visible(not Global.is_mobile())

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


func _gui_input(event):
	if event is InputEventScreenTouch:
		UiSounds.play_sound("widget_emotes_close")
		hide()
		Global.explorer_grab_focus()

	if event is InputEventKey:
		# Play emotes
		if event.keycode >= KEY_0 and event.keycode <= KEY_9:
			if event.pressed:
				var index = event.keycode - KEY_0
				var emote_urn = emote_items[index].emote_urn
				_on_play_emote(emote_urn)


func _physics_process(_delta):
	if Input.is_action_just_pressed("ia_open_emote_wheel") and Global.explorer_has_focus():
		if not is_visible_in_tree():
			UiSounds.play_sound("widget_emotes_open")
		show()
		grab_focus()
		Global.release_mouse()


func _on_play_emote(emote_urn: String):
	self.hide()
	Global.explorer_grab_focus()
	if avatar_node != null:
		var emote_controller = avatar_node.emote_controller
		emote_controller.play_emote(emote_urn)
		emote_controller.broadcast_avatar_animation(emote_urn)
	else:
		printerr("No avatar node in EmoteWheel!")


func _on_select_emote(selected: bool, emote_urn: String, child: EmoteItemUi):
	if emote_urn == last_selected_emote_urn:
		return

	if !selected:
		label_emote_name.text = ""
		last_selected_emote_urn = ""
		return

	last_selected_emote_urn = emote_urn
	label_emote_name.text = child.emote_name
	UiSounds.play_sound("backpack_item_highlight")
