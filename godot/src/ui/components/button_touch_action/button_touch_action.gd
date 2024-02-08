@tool
extends Control

@export var text_color = Color.WHITE:
	set(new_value):
		%Label_Letter.add_theme_color_override("font_color", new_value)
		text_color = new_value

@export var text_letter = "E":
	set(new_value):
		%Label_Letter.text = new_value
		%Label_Letter.set_visible(icon == null)
		text_letter = new_value

@export var icon: Texture2D = null:
	set(new_value):
		icon = new_value
		%MarginContainer_Icon.set_visible(icon != null)
		%Label_Letter.set_visible(icon == null)
		%TextureRect_Icon.texture = icon

@export var trigger_action = "ia_primary"

var action_pressed = false

@onready var button_touch_action = %Button_TouchAction

@onready var default_position = button_touch_action.get_position()
@onready var pressed_position = button_touch_action.get_position() - Vector2(0, 2)


func _physics_process(_delta):
	if not Engine.is_editor_hint():
		var is_pressed = Input.is_action_pressed(trigger_action)
		if action_pressed != is_pressed:
			action_pressed = is_pressed
			if is_pressed:
				button_touch_action.set_position(pressed_position)
			else:
				button_touch_action.set_position(default_position)


func _on_button_touch_action_button_down():
	Input.action_press(trigger_action)


func _on_button_touch_action_button_up():
	Input.action_release(trigger_action)
