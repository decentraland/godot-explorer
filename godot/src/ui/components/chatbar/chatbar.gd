extends Control

signal share_place
signal load_scenes_pressed

var _tooltip_tween: Tween = null
var _tooltip_shown: bool = false

@onready var hbox: HBoxContainer = $HBoxContainer
@onready var button_chat: TextureButton = %Button_Chat
@onready var button_flip: TextureButton = %Button_Flip
@onready var tooltip: HBoxContainer = %HBoxContainer_Tooltip
@onready var discover_panel: PanelContainer = %DiscoverPanel
@onready var scene_panel: PanelContainer = %ScenePanel
@onready var panel_load_scenes: PanelContainer = %Panel_LoadScenes


func _ready() -> void:
	Global.change_parcel.connect(_on_change_parcel)
	Global.close_chat.connect(_on_chat_closed)


func _on_change_parcel(coordinates: Vector2i) -> void:
	%Label_Coordinates.text = "%d,%d" % [coordinates.x, coordinates.y]


func _on_hud_button_discover_pressed() -> void:
	Global.open_discover.emit()
	Global.send_haptic_feedback()


func _on_hud_button_share_pressed() -> void:
	share_place.emit()
	Global.send_haptic_feedback()


func _on_button_chat_pressed() -> void:
	Global.send_haptic_feedback()
	if button_chat.button_pressed:
		_enter_chat_mode()
		Global.open_chat.emit()
	else:
		_exit_chat_mode()
		Global.close_chat.emit()


func _on_chat_closed() -> void:
	if button_chat.button_pressed:
		button_chat.set_pressed_no_signal(false)
	_exit_chat_mode()


func _enter_chat_mode() -> void:
	button_flip.show()
	discover_panel.hide()
	scene_panel.hide()
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
	discover_panel.show()
	scene_panel.show()
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


func is_point_inside(position: Vector2) -> bool:
	return hbox.get_global_rect().has_point(position)
