extends Control

signal close_all
signal close_only_panels
signal navbar_opened

@onready var animation_player: AnimationPlayer = %AnimationPlayer
@onready var panel_container: PanelContainer = %PanelContainer
@onready var v_box_container_buttons: VBoxContainer = %VBoxContainer_Buttons
@onready var hud_button_friends: Button = %HudButton_Friends
@onready var hud_button_notifications: Button = %HudButton_Notifications
@onready var hud_button_backpack: Button = %HudButton_Backpack
@onready var hud_button_settings: Button = %HudButton_Settings
@onready var button: Button = %Button
@onready var portrait_button_profile: Button = %Portrait_Button_Profile

var _manually_hidden: bool = false


func _ready() -> void:
	var btn_group = ButtonGroup.new()
	btn_group.allow_unpress = false
	hud_button_friends.button_group = btn_group
	hud_button_notifications.button_group = btn_group
	hud_button_backpack.button_group = btn_group
	hud_button_settings.button_group = btn_group
	portrait_button_profile.button_group = btn_group
	# Asegurar que siempre haya un botón presionado al inicio
	# El ButtonGroup con allow_unpress = false garantiza que siempre haya uno presionado

	get_window().size_changed.connect(self._on_size_changed)
	_on_size_changed()


func _on_size_changed():
	# Si el navbar fue ocultado manualmente, no cambiar su visibilidad
	if _manually_hidden:
		return
	
	# Verificar si discover o chat están abiertos - si lo están, mantener oculto
	var explorer = Global.get_explorer()
	if explorer != null:
		# Si discover está abierto, mantener oculto
		if explorer.control_menu != null and explorer.control_menu.visible and explorer.control_menu.control_discover.visible:
			hide()
			return
		# Si chat está abierto, mantener oculto
		if explorer.chat_container != null and explorer.chat_container.visible:
			hide()
			return
	
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
	
	
func capture_mouse():
	Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)


func _on_portrait_button_profile_toggled(toggled_on: bool) -> void:
	if toggled_on:
		Global.open_own_profile.emit()


func close_from_discover_button():
	button.set_pressed_no_signal(false)
	animation_player.play("close")


func set_manually_hidden(is_hidden: bool) -> void:
	_manually_hidden = is_hidden
	if is_hidden:
		hide()
	else:
		# Verificar si discover o chat están abiertos antes de restaurar la visibilidad
		var explorer = Global.get_explorer()
		if explorer != null:
			# Si discover está abierto, mantener oculto
			if explorer.control_menu.visible and explorer.control_menu.control_discover.visible:
				return
			# Si chat está abierto, mantener oculto
			if explorer.chat_container.visible:
				return
		
		# Restaurar visibilidad basada en el tamaño de la ventana solo si discover y chat están cerrados
		var window_size: Vector2i = DisplayServer.window_get_size()
		visible = window_size.x > window_size.y
