extends Button

signal toggle_color_panel(toggled, color_target)

enum ColorTarget { SKIN, HAIR, EYE }

@export var color_target := ColorTarget.SKIN

@onready var panel_color = $Panel_Color

var stylebox: StyleBoxFlat

func _ready():
	stylebox = panel_color.get_theme_stylebox("panel").duplicate()
	panel_color.add_theme_stylebox_override("panel", stylebox)


func set_color(color: Color) -> void:
	stylebox.bg_color = color


func _on_toggled(toggled_on):
	toggle_color_panel.emit(toggled_on, color_target)
