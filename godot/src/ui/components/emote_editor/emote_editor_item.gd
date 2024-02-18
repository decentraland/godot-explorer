class_name EmoteEditorItem
extends BaseButton

signal select_emote(emote_urn: String)

var _emote_urn: String = ""

@onready var panel_pressed = %Pressed
@onready var emote_square_item = %EmoteSquareItem
@onready var label_number = %Label_Number
@onready var label_emote_name = %Label_EmoteName
@onready var texture_rect_wheel = %TextureRect_Wheel


func _on_toggled(toggled_on):
	panel_pressed.visible = toggled_on
	if toggled_on:
		select_emote.emit(_emote_urn)


func async_load_from_urn(_emote_urn: String, index: int):
	if self._emote_urn == _emote_urn:  # No need to reload
		return

	self._emote_urn = _emote_urn
	label_number.text = str(index)
	texture_rect_wheel.rotation_degrees = (36.0 * index) - 36.0
	await emote_square_item.async_load_from_urn(_emote_urn)

	# get emote name from emote_emote_ui
	label_emote_name.text = emote_square_item.emote_name
