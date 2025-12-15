extends Control

@onready var animation_player: AnimationPlayer = %AnimationPlayer
@onready var panel_container: PanelContainer = %PanelContainer
@onready var v_box_container_buttons: VBoxContainer = %VBoxContainer_Buttons



func _on_button_toggled(toggled_on: bool) -> void:
	if toggled_on:
		animation_player.play("open")
	else:
		animation_player.play("close")
