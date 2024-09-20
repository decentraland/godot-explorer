class_name CarrouselGenerator
extends Node

enum LoadingStatus {
	Loading,
	OkWithResults,
	OkWithoutResults,
	Error
}

signal report_loading_status(status: LoadingStatus)
signal item_pressed(data)

@export var discover: Discover = null

var item_container: Container = null
var new_search: bool = true

var search_param: String:
	set(new_value):
		new_search = true
		search_param = new_value
		if is_instance_valid(item_container):
			for child in item_container.get_children():
				child.queue_free()
				item_container.remove_child(child)

func on_request(_offset: int, _limit: int) -> void:
	printerr("This must be override")
