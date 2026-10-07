class_name FtueWelcomeModal
extends Control

## "Welcome to Decentraland!" card that offers the guided tutorial (issue #2767). Shown over the
## world without dimming it; the full-rect root keeps touches from reaching the HUD behind.

signal start_pressed
signal skip_pressed

@onready var modal_actions: ModalActions = %ModalActions


# The shared button row starts with its primary disabled, for forms that validate first.
func _ready() -> void:
	modal_actions.set_primary_enabled(true)


func _on_modal_actions_primary_pressed() -> void:
	start_pressed.emit()


func _on_modal_actions_secondary_pressed() -> void:
	skip_pressed.emit()
