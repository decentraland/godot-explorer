extends Control

const BACKPACK_OFF = preload("res://assets/ui/nav-bar-icons/backpack-off.svg")
const BACKPACK_ON = preload("res://assets/ui/nav-bar-icons/backpack-on.svg")
const EXPLORER_OFF = preload("res://assets/ui/nav-bar-icons/explorer-off.svg")
const EXPLORER_ON = preload("res://assets/ui/nav-bar-icons/explorer-on.svg")
const MAP_OFF = preload("res://assets/ui/nav-bar-icons/map-off.svg")
const MAP_ON = preload("res://assets/ui/nav-bar-icons/map-on.svg")
const SETTINGS_OFF = preload("res://assets/ui/nav-bar-icons/settings-off.svg")
const SETTINGS_ON = preload("res://assets/ui/nav-bar-icons/settings-on.svg")

@onready var button_discover: Button = %Button_Discover
@onready var button_map: Button = %Button_Map
@onready var button_backpack: Button = %Button_Backpack
@onready var button_settings: Button = %Button_Settings

@onready var label_menu: Label = %LabelMenu

# Called when the node enters the scene tree for the first time.
func _ready() -> void:
	# default DISCOVER
	button_discover.button_pressed = true
	_on_button_discover_pressed()


# Called every frame. 'delta' is the elapsed time since the previous frame.
func _process(delta: float) -> void:
	pass


func _on_button_discover_pressed() -> void:
	label_menu.text = "DISCOVER"

func _on_button_map_pressed() -> void:
	label_menu.text = "MAP"

func _on_button_backpack_pressed() -> void:
	label_menu.text = "BACKPACK"

func _on_button_settings_pressed() -> void:
	label_menu.text = "SETTINGS"


func _on_button_discover_toggled(toggled_on):
	button_discover.icon = EXPLORER_ON if toggled_on else EXPLORER_OFF
	button_discover.get_child(0).set_visible(toggled_on)


func _on_button_map_toggled(toggled_on):
	button_map.icon = MAP_ON if toggled_on else MAP_OFF
	button_map.get_child(0).set_visible(toggled_on)


func _on_button_backpack_toggled(toggled_on):
	button_backpack.icon = BACKPACK_ON if toggled_on else BACKPACK_OFF
	button_backpack.get_child(0).set_visible(toggled_on)
	#if !toggled_on:
	#	_async_deploy_if_has_changes()


func _on_button_settings_toggled(toggled_on):
	button_settings.icon = SETTINGS_ON if toggled_on else SETTINGS_OFF
	button_settings.get_child(0).set_visible(toggled_on)
