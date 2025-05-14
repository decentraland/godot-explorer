extends Control

signal clicked_parcel(parcel: Vector2i)

@onready var map: Control = %Map
@onready var map_viewport: SubViewport = %MapViewport
@onready var search_and_filters: Control = $SearchAndFilters

var show_poi:= true
var show_live:= true

func _ready():
	UiSounds.install_audio_recusirve(self)
	get_viewport().size_changed.connect(_on_viewport_resized)
	_on_viewport_resized()
	
func _on_viewport_resized()->void:
	map_viewport.size = size

# TO IMPLEMENT (need to add a menu)
func _on_show_poi_toggled(toggled_on: bool) -> void:
	show_poi = toggled_on
	map.show_poi_toggled(toggled_on)

func _on_show_live_toggled(toggled_on: bool) -> void:
	show_live = toggled_on
	map.show_live_toggled(toggled_on)
