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
@onready var hud_button_friends: Button = %HudButton_Friends
@onready var hud_button_notifications: Button = %HudButton_Notifications
@onready var hud_button_backpack: Button = %HudButton_Backpack
@onready var hud_button_settings: Button = %HudButton_Settings



func _ready() -> void:
	var btn_group = ButtonGroup.new()
	btn_group.allow_unpress = false
	hud_button_friends.button_group = btn_group
	hud_button_notifications.button_group = btn_group
	hud_button_backpack.button_group = btn_group
	hud_button_settings.button_group = btn_group
	# Asegurar que siempre haya un botón presionado al inicio
	# El ButtonGroup con allow_unpress = false garantiza que siempre haya uno presionado

	get_window().size_changed.connect(self._on_size_changed)
	_on_size_changed()


func _on_size_changed():
	var window_size: Vector2i = DisplayServer.window_get_size()
	visible = window_size.x > window_size.y

		
		
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


func _on_hud_button_backpack_toggled(toggled_on: bool) -> void:
	if toggled_on:
		backpack_button_clicked.emit()


func _on_hud_button_settings_toggled(toggled_on: bool) -> void:
	if toggled_on:
		settings_button_clicked.emit()

func capture_mouse():
	Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)
