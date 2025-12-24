extends Control

signal close_all
signal close_only_panels
signal navbar_opened

enum BUTTON { FRIENDS, NOTIFICATIONS, BACKPACK, SETTINGS }

var _manually_hidden: bool = false

@onready var animation_player: AnimationPlayer = %AnimationPlayer
@onready var panel_container: PanelContainer = %PanelContainer
@onready var v_box_container_buttons: VBoxContainer = %VBoxContainer_Buttons
@onready var hud_button_friends: Button = %HudButton_Friends
@onready var hud_button_notifications: Button = %HudButton_Notifications
@onready var hud_button_backpack: Button = %HudButton_Backpack
@onready var hud_button_settings: Button = %HudButton_Settings
@onready var button: Button = %Button
@onready var portrait_button_profile: Button = %Portrait_Button_Profile


func _ready() -> void:
	var btn_group = ButtonGroup.new()
	btn_group.allow_unpress = false
	hud_button_friends.button_group = btn_group
	hud_button_notifications.button_group = btn_group
	hud_button_backpack.button_group = btn_group
	hud_button_settings.button_group = btn_group
	portrait_button_profile.button_group = btn_group
	# Ensure there's always a pressed button at startup
	# The ButtonGroup with allow_unpress = false ensures one is always pressed

	Global.close_navbar.connect(_on_navbar_close)
	Global.open_navbar_silently.connect(_on_navbar_open_silently_on_backpack)

	get_window().size_changed.connect(self._on_size_changed)
	_on_size_changed()


func _on_size_changed():
	if _manually_hidden:
		return
	# If navbar was manually hidden, don't change its visibility

	var explorer = Global.get_explorer()
	if explorer != null:
		# Check if discover or chat are open - if so, keep hidden
		if (
			explorer.control_menu != null
			and explorer.control_menu.visible
			and explorer.control_menu.control_discover.visible
		):
			# If discover is open, keep hidden
			hide()
			return
		if explorer.chat_container != null and explorer.chat_container.visible:
			# If chat is open, keep hidden
			hide()
			return

	var window_size: Vector2i = DisplayServer.window_get_size()
	visible = window_size.x > window_size.y


func _on_navbar_close() -> void:
	close_from_discover_button()
	close_all.emit()


func _on_button_toggled(toggled_on: bool) -> void:
	Global.send_haptic_feedback()
	if toggled_on:
		animation_player.play("open")
		set_button_pressed(BUTTON.FRIENDS)
		navbar_opened.emit()
	else:
		animation_player.play("close")
		close_all.emit()


## Set a button as pressed
func set_button_pressed(button_to_press: BUTTON) -> void:
	match button_to_press:
		BUTTON.FRIENDS:
			hud_button_friends.button_pressed = true
		BUTTON.NOTIFICATIONS:
			hud_button_notifications.button_pressed = true
		BUTTON.BACKPACK:
			hud_button_backpack.button_pressed = true
		BUTTON.SETTINGS:
			hud_button_settings.button_pressed = true


func capture_mouse():
	Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)


func _on_portrait_button_profile_toggled(toggled_on: bool) -> void:
	if toggled_on:
		Global.open_own_profile.emit()


func close_from_discover_button():
	button.set_pressed_no_signal(false)
	animation_player.play("close")


func _on_navbar_open_silently_on_backpack() -> void:
	if not button.button_pressed:
		button.set_pressed_no_signal(true)
		animation_player.play("open")
	set_button_pressed(BUTTON.BACKPACK)


func set_manually_hidden(is_hidden: bool) -> void:
	_manually_hidden = is_hidden
	if is_hidden:
		hide()
	else:
		var explorer = Global.get_explorer()
		if explorer != null:
			# Check if discover or chat are open before restoring visibility
			if explorer.control_menu.visible and explorer.control_menu.control_discover.visible:
				# If discover is open, keep hidden
				return
			if explorer.chat_container.visible:
				# If chat is open, keep hidden
				return

		# Restore visibility based on window size only if discover and chat are closed
		var window_size: Vector2i = DisplayServer.window_get_size()
		visible = window_size.x > window_size.y
