extends Control

signal jump_to(parcel: Vector2i)

@onready var map: Control = %Map
@onready var map_viewport: SubViewport = %MapViewport
@onready var search_and_filters: Control = $SearchAndFilters
@onready var jump_in: ColorRect = %JumpIn

var show_poi:= true
var show_live:= true

func _ready():
	jump_in.hide()
	UiSounds.install_audio_recusirve(self)
	get_window().size_changed.connect(self._on_size_changed)
	refresh_viewport_size()
	
func _on_size_changed() -> void:
	refresh_viewport_size()
	
func refresh_viewport_size() -> void:
	map_viewport.size = size

# TO IMPLEMENT (need to add a menu)
func _on_show_poi_toggled(toggled_on: bool) -> void:
	show_poi = toggled_on
	map.show_poi_toggled(toggled_on)

func _on_show_live_toggled(toggled_on: bool) -> void:
	show_live = toggled_on
	map.show_live_toggled(toggled_on)

func _on_map_clicked_parcel(parcel: Vector2i) -> void:
	await jump_in.async_load_place_position(Vector2i(parcel.x, -parcel.y))
