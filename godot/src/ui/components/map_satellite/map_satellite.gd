extends Control

signal jump_to(parcel: Vector2i)

var show_poi := true
var show_live := true

@onready var map: Control = %Map
@onready var map_viewport: SubViewport = %MapViewport
@onready var search_and_filters: Control = $SearchAndFilters
@onready var jump_in: ColorRect = %JumpIn


func _ready():
	jump_in.hide()
	UiSounds.install_audio_recusirve(self)


# TO IMPLEMENT (need to add a menu)
func _on_show_poi_toggled(toggled_on: bool) -> void:
	show_poi = toggled_on
	map.show_poi_toggled(toggled_on)


func _on_show_live_toggled(toggled_on: bool) -> void:
	show_live = toggled_on
	map.show_live_toggled(toggled_on)


func _async_on_map_clicked_parcel(parcel: Vector2i) -> void:
	await jump_in.async_load_place_position(Vector2i(parcel.x, -parcel.y))


func _on_jump_in_jump_in(parcel_position: Vector2i, realm: String) -> void:
	jump_in.hide()
	Global.teleport_to(parcel_position, realm)
