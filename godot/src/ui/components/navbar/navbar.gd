extends Control

signal notifications_button_clicked
signal friends_button_clicked
signal backpack_button_clicked
signal settings_button_clicked
signal close_all
signal navbar_opened

@onready var animation_player: AnimationPlayer = %AnimationPlayer
@onready var panel_container: PanelContainer = %PanelContainer
@onready var v_box_container_buttons: VBoxContainer = %VBoxContainer_Buttons
@onready
var hud_button_friends: AnimatedButton = $Control/Control/PanelContainer/VBoxContainer_Buttons/HudButton_Friends
@onready
var hud_button_notifications: AnimatedButton = $Control/Control/PanelContainer/VBoxContainer_Buttons/HudButton_Notifications


func _ready() -> void:
	var btn_group = ButtonGroup.new()
	btn_group.allow_unpress = false
	hud_button_friends.button_group = btn_group
	hud_button_notifications.button_group = btn_group
	# Asegurar que siempre haya un botón presionado al inicio
	# El ButtonGroup con allow_unpress = false garantiza que siempre haya uno presionado


func _on_button_toggled(toggled_on: bool) -> void:
	if toggled_on:
		animation_player.play("open")
		hud_button_friends.pressed.emit()
		hud_button_friends.button_pressed = true
		navbar_opened.emit()
	else:
		animation_player.play("close")
		close_all.emit()


func _on_hud_button_notifications_toggled(toggled_on: bool) -> void:
	if toggled_on:
		notifications_button_clicked.emit()


func _on_hud_button_friends_toggled(toggled_on: bool) -> void:
	if toggled_on:
		friends_button_clicked.emit()
