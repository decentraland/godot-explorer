class_name EmoteEditorItem
extends BaseButton

signal select_emote(emote_urn: String)
signal clear_emote

var use_equipped_border: bool = false

var _emote_urn: String = ""

@onready var emote_square_item = %EmoteSquareItem


func _on_gui_input(event: InputEvent) -> void:
	if (
		event is InputEventMouseButton
		and event.button_index == MOUSE_BUTTON_LEFT
		and event.pressed
		and event.double_click
	):
		clear_slot()


func set_empty() -> void:
	_emote_urn = ""
	emote_square_item.set_empty()


func clear_slot() -> void:
	set_empty()
	clear_emote.emit()


func _on_toggled(toggled_on):
	#panel_pressed.visible = toggled_on
	if use_equipped_border:
		emote_square_item.set_slot_selected(toggled_on)
	else:
		emote_square_item.set_pressed(toggled_on)
	if toggled_on:
		select_emote.emit(_emote_urn)



func async_load_from_urn(new_emote_urn: String, _index: int):
	if new_emote_urn.is_empty():
		emote_square_item.set_empty()
		return

	if _emote_urn == new_emote_urn:  # No need to reload
		return

	_emote_urn = new_emote_urn

	#texture_rect_wheel.rotation_degrees = (36.0 * index) - 36.0
	await emote_square_item.async_load_from_urn(new_emote_urn)

	# get emote name from emote_emote_ui
	#label_emote_name.text = emote_square_item.emote_name
