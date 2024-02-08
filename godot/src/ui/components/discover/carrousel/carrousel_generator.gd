class_name CarrouselGenerator
extends Node

signal set_consumer_visible(visible: bool)
signal item_pressed(data)

@export var discover: Discover = null

var item_container: Container = null


func on_request(_offset: int, _limit: int) -> void:
	printerr("This must be override")
