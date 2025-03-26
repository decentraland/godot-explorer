extends Button
class_name MenuNavbarButton

@export var button_pressed_icon: Texture

var _button_unpressed_icon: Texture
@onready var highlight: Control = preload("res://src/ui/components/menu_navbar_button/menu_navbar_highlight.tscn").instantiate() as Control

func _ready():
	add_child(highlight)
	# we store the value
	_button_unpressed_icon = icon
	toggled.connect(self._on_toggled)

func _on_toggled(toggled_on: bool) -> void:
	highlight.visible = toggled_on
	icon = button_pressed_icon if toggled_on else _button_unpressed_icon
