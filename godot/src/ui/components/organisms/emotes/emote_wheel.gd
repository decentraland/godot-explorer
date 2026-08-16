extends Control

signal emote_wheel_opened
signal emote_wheel_closed

# Orb button textures (icon baked in). Swapped by state on the emote toggle:
# Default (closed) → Pressed (while the finger is down) → Hold (wheel open, after release).
const EMOTE_TEX_DEFAULT: Texture2D = preload(
	"res://src/ui/components/organisms/emotes/emote_default.png"
)
const EMOTE_TEX_PRESSED: Texture2D = preload(
	"res://src/ui/components/organisms/emotes/emote_pressed.png"
)
const EMOTE_TEX_HOLD: Texture2D = preload("res://src/ui/components/organisms/emotes/emote_hold.png")

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

# True while the finger is physically down on the emote button (the transient Pressed look).
var _emote_pressing: bool = false

@onready var emote_wheel_container = %EmoteWheelContainer
@onready var label_emote_name = %Label_EmoteName
@onready var control_wheel: Control = %Control_Wheel
@onready var button_emotes: TextureButton = $Button_Emotes
@onready var emote_wheel_item_1: EmoteItemUi = %EmoteWheelItem1


func _ready():
	button_emotes.set_meta("attenuated_sound", true)
	# Drive the toggle from raw touch so it works for every finger, not just the
	# primary one Godot synthesizes a mouse event from. button_mask = 0 makes the
	# base Button ignore the emulated mouse so there's a single, consistent path.
	button_emotes.button_mask = 0
	button_emotes.gui_input.connect(_on_button_emotes_gui_input)
	control_wheel.hide()
	_update_emote_texture()
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
		Global.comms.send_emote(emote_urn)
	else:
		printerr("No avatar node in EmoteWheel!")


func _on_select_emote(selected: bool, emote_urn: String, child: EmoteItemUi):
	if !selected:
		label_emote_name.text = "Emotes"
		last_selected_emote_urn = ""
		return

	# Empty slot highlighted: hint the equip action instead of an emote name.
	if emote_urn.is_empty():
		label_emote_name.text = "Equip Emote"
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
	if button_emotes != null and button_emotes.button_pressed:
		button_emotes.set_pressed_no_signal(false)
	_update_emote_texture()


func open() -> void:
	if control_wheel.visible:
		return
	control_wheel.show()
	emote_wheel_opened.emit()
	grab_focus()
	Global.release_mouse()
	_update_emote_texture()


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
	if event is InputEventScreenTouch:
		if event.pressed:
			# Show the transient Pressed look while the finger is down, then flip
			# button_pressed (emits `toggled` → open()/close(), which re-skins the button).
			_emote_pressing = true
			button_emotes.button_pressed = not button_emotes.button_pressed
			_update_emote_texture()
		else:
			# Released: settle onto Hold (wheel open) or Default (closed).
			_emote_pressing = false
			_update_emote_texture()
		button_emotes.accept_event()


## Skins the emote button by state: Pressed while the finger is down, else Hold when the
## wheel is open, else Default. Called from press/release and from open()/close().
func _update_emote_texture() -> void:
	if button_emotes == null:
		return
	var tex: Texture2D = EMOTE_TEX_DEFAULT
	if _emote_pressing:
		tex = EMOTE_TEX_PRESSED
	elif button_emotes.button_pressed:
		tex = EMOTE_TEX_HOLD
	button_emotes.texture_normal = tex


func _on_button_toggled(toggled_on: bool) -> void:
	if toggled_on:
		open()
	else:
		close()


func _on_button_edit_pressed() -> void:
	Global.open_backpack.emit(true)
	Global.send_haptic_feedback()
	close()
