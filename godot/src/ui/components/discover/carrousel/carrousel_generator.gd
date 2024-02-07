class_name CarrouselGenerator
extends Node

@export var discover: Discover = null

signal set_consumer_visible(visible: bool)
signal item_pressed(data)

var item_container: Container = null

func on_request(_offset: int, _limit: int) -> void:
	printerr("This must be override")
