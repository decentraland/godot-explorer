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

# Owning finger index for the raw-touch open/close (see _on_button_emotes_gui_input). -1 = none.
var _emote_touch_index: int = -1

@onready var emote_wheel_container = %EmoteWheelContainer
@onready var label_emote_name = %Label_EmoteName
@onready var control_wheel: Control = %Control_Wheel
@onready var button_emotes: HudButton = $Button_Emotes
@onready var emote_wheel_item_1: EmoteItemUi = %EmoteWheelItem1


## The label node is auto_translate_mode = 2, because _on_select_emote() writes a creator-authored
## emote name into it, which must never go through a lookup. That makes the default title this
## component's job: the scene used to carry the literal "EMOTES_EMOTES", which is a key, so a
## mode-2 node drew it verbatim on screen until the first hover.
func _reset_emote_name() -> void:
	label_emote_name.text = tr("EMOTES_EMOTES")


func _notification(what: int) -> void:
	# Only the default title re-translates; a selected emote name is data, not copy.
	if what == NOTIFICATION_TRANSLATION_CHANGED and is_node_ready():
		if last_selected_emote_urn.is_empty():
			_reset_emote_name()


func _ready():
	_reset_emote_name()
	button_emotes.set_meta("attenuated_sound", true)
	# Toggle the wheel from raw touch so a second finger opens it while the joystick is held
	# (Godot only synthesizes a mouse event from the primary touch). button_mask = 0 routes all
	# activation through _on_button_emotes_gui_input, covering both fingers (desktop clicks arrive
	# as emulated InputEventScreenTouch). visibility_changed clears a stuck press if the button is
	# hidden mid-touch.
	button_emotes.button_mask = 0
	button_emotes.gui_input.connect(_on_button_emotes_gui_input)
	button_emotes.visibility_changed.connect(_on_button_emotes_visibility_changed)
	control_wheel.hide()
	# Clone the template item 9 more times (10 total) and fan them 36° apart. The clone
	# copies %EmoteWheelItem1's scene-unique-name flag, so clear it to avoid ambiguous %.
	for i in 9:
		var new_emote_wheel_item: EmoteItemUi = emote_wheel_item_1.duplicate()
		new_emote_wheel_item.unique_name_in_owner = false
		new_emote_wheel_item.rotation_degrees = 36 * (i + 1)
		emote_wheel_container.add_child(new_emote_wheel_item)

	for child in emote_wheel_container.get_children():
		if child is EmoteItemUi:
			child.play_emote.connect(self._on_play_emote)
			child.select_emote.connect(self._on_select_emote.bind(child))
			# Fire the emote from raw touch so a second finger works too (Godot's
			# emulated mouse only covers the primary touch). button_mask = 0 keeps a
			# single, consistent input path. Scoped to the wheel only — the shared
			# EmoteItemUi is left untouched for the backpack grid.
			child.button_mask = 0
			child.gui_input.connect(_on_emote_item_gui_input.bind(child))
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
		var emote_item: EmoteItemUi = emote_items[i]
		# get_emotes() always returns 10 slots; an unequipped slot comes back empty.
		# Render it as an empty slot instead of leaving the previous (ghost) icon (#2458).
		if i >= emote_urns.size() or String(emote_urns[i]).is_empty():
			emote_item.set_empty()
			continue

		emote_item.async_load_from_urn(emote_urns[i], i)  # Forget await


func _on_play_emote(emote_urn: String):
	close()

	# Empty slot: nothing to play — send the user to the backpack emotes tab to equip one.
	if emote_urn.is_empty():
		Global.open_backpack.emit(true)
		Global.send_haptic_feedback()
		return

	# Check if emotes are disabled by the current scene
	if Global.is_emote_disabled():
		return

	if avatar_node != null:
		var emote_controller = avatar_node.emote_controller
		# Use async_play_emote to ensure base emotes are loaded from remote
		emote_controller.async_play_emote(emote_urn)
		# -1 = full body: wheel emotes carry no avatar mask.
		Global.comms.send_emote(emote_urn, -1)
	else:
		printerr("No avatar node in EmoteWheel!")


func _on_select_emote(selected: bool, emote_urn: String, child: EmoteItemUi):
	if !selected:
		_reset_emote_name()
		last_selected_emote_urn = ""
		return

	# Empty slot highlighted: hint the equip action instead of an emote name.
	if emote_urn.is_empty():
		label_emote_name.text = tr("EMOTES_EQUIP_EMOTE")
		last_selected_emote_urn = ""
		return

	if emote_urn == last_selected_emote_urn:
		return

	last_selected_emote_urn = emote_urn
	label_emote_name.text = child.emote_name
	UiSounds.play_sound("backpack_item_highlight", child.has_meta("attenuated_sound"))


func close() -> void:
	if not control_wheel.visible:
		return
	control_wheel.hide()
	emote_wheel_closed.emit()
	Global.explorer_grab_focus()
	# Clearing via button_pressed (not set_pressed_no_signal) lets HudButton's `toggled` handler
	# repaint to the default orb; the re-entrant close() early-returns since the wheel is hidden.
	if button_emotes != null and button_emotes.button_pressed:
		button_emotes.button_pressed = false


func open() -> void:
	if control_wheel.visible:
		return
	_reset_emote_name()
	last_selected_emote_urn = ""
	control_wheel.show()
	emote_wheel_opened.emit()
	grab_focus()
	Global.release_mouse()


func _on_control_wheel_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		close()


func _on_emote_item_gui_input(event: InputEvent, item: EmoteItemUi) -> void:
	if event is InputEventScreenTouch and event.pressed:
		# Re-emitting the item's own signal reuses the existing wiring (play + close
		# in _on_play_emote, and the equip SFX connected by UISounds).
		item.play_emote.emit(item.emote_urn)
		item.accept_event()


func _on_button_emotes_gui_input(event: InputEvent) -> void:
	# Full raw-touch handling (mirrors ButtonTouchAction) so a second finger opens the wheel while
	# the joystick is held AND the orb shows its pressed state during the hold. We own a single
	# finger index and drive button_down/up + the open/close toggle by hand — button_mask = 0 stops
	# the base Button from doing it (it's mouse-only, so blind to the second finger). Emitting
	# button_down/up flips HudButton's pressed orb; flipping button_pressed opens/closes the wheel.
	# Touch-only, like ButtonTouchAction: on desktop `emulate_touch_from_mouse` (project.godot) turns
	# clicks into InputEventScreenTouch, so the mouse is covered here without a separate branch.
	if event is InputEventScreenTouch:
		if event.pressed:
			if _emote_touch_index == -1:
				_emote_touch_index = event.index
				button_emotes.button_down.emit()
				button_emotes.button_pressed = not button_emotes.button_pressed
			button_emotes.accept_event()
		elif event.index == _emote_touch_index:
			_emote_touch_index = -1
			button_emotes.button_up.emit()
			button_emotes.accept_event()


func _on_button_emotes_visibility_changed() -> void:
	# If the button is hidden mid-press (hide-UI, orientation change, teardown) the release touch
	# never arrives — release the owned finger and settle the orb so it doesn't stay stuck pressed.
	if _emote_touch_index != -1 and not button_emotes.is_visible_in_tree():
		_emote_touch_index = -1
		button_emotes.button_up.emit()


func _on_button_toggled(toggled_on: bool) -> void:
	if toggled_on:
		open()
	else:
		close()


func _on_button_edit_pressed() -> void:
	Global.open_backpack.emit(true)
	Global.send_haptic_feedback()
	close()
