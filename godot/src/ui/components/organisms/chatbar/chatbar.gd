extends Control

signal share_place
signal load_scenes_pressed

var _tooltip_tween: Tween = null
var _tooltip_shown: bool = false

@onready var hbox: HBoxContainer = $HBoxContainer
@onready var button_chat: HudButton = %Button_Chat
@onready var button_flip: HudButton = %Button_Flip
@onready var tooltip: HBoxContainer = %HBoxContainer_Tooltip
@onready var panel_load_scenes: PanelContainer = %Panel_LoadScenes


func _ready() -> void:
	Global.close_chat.connect(_on_chat_closed)
	Global.orientation_changed.connect(_on_orientation_changed)
	# The orb buttons scale with orientation: landscape → SMALL (66 disc), portrait → LARGE (96).
	_apply_button_sizes()


func _on_orientation_changed(_is_portrait: bool) -> void:
	_apply_button_sizes()


func _apply_button_sizes() -> void:
	var preset: HudButton.Size = (
		HudButton.Size.LARGE if Global.is_orientation_portrait() else HudButton.Size.SMALL
	)
	button_chat.size_preset = preset
	button_flip.size_preset = preset


func _on_hud_button_share_pressed() -> void:
	share_place.emit()
	Global.send_haptic_feedback()


func _on_button_chat_pressed() -> void:
	Global.send_haptic_feedback()
	# HudButton skins itself from its toggle state (default → pressed → selected); we only drive
	# the chat open/close side effects here.
	if button_chat.button_pressed:
		_enter_chat_mode()
		Global.open_chat.emit()
	else:
		_exit_chat_mode()
		Global.close_chat.emit()


func _on_chat_closed() -> void:
	# Clear the toggle via button_pressed (not set_pressed_no_signal) so the HudButton's `toggled`
	# handler repaints back to the default orb; `pressed` is not re-emitted by a programmatic set.
	if button_chat.button_pressed:
		button_chat.button_pressed = false
	_exit_chat_mode()


func _enter_chat_mode() -> void:
	button_flip.show()
	if not _tooltip_shown:
		_tooltip_shown = true
		_show_tooltip()


func _on_button_flip_pressed() -> void:
	Global.send_haptic_feedback()
	if Global.is_orientation_portrait():
		Global.set_orientation_landscape()
	else:
		Global.set_orientation_portrait()


func _exit_chat_mode() -> void:
	button_flip.hide()
	_kill_tooltip()


func _show_tooltip() -> void:
	_kill_tooltip()
	tooltip.modulate = Color.WHITE
	tooltip.show()
	_tooltip_tween = create_tween()
	_tooltip_tween.tween_interval(10.0)
	_tooltip_tween.tween_property(tooltip, "modulate:a", 0.0, 1.0)
	_tooltip_tween.tween_callback(tooltip.hide)


func _kill_tooltip() -> void:
	if _tooltip_tween and _tooltip_tween.is_valid():
		_tooltip_tween.kill()
		_tooltip_tween = null
	tooltip.hide()


func _on_button_load_scenes_pressed() -> void:
	load_scenes_pressed.emit()


func show_load_scenes_button() -> void:
	panel_load_scenes.show()


func hide_load_scenes_button() -> void:
	panel_load_scenes.hide()
