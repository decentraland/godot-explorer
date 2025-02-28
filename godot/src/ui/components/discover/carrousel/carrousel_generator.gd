class_name CarrouselGenerator
extends Node

@warning_ignore("unused_signal")
signal report_loading_status(status: LoadingStatus)
@warning_ignore("unused_signal")
signal item_pressed(data)

enum LoadingStatus { LOADING, OK_WITH_RESULTS, OK_WITHOUT_RESULTS, ERROR }

@export var discover: Discover = null

var item_container: Container = null
var new_search: bool = true

var search_param: String:
	set(new_value):
		new_search = true
		search_param = new_value


func on_request(_offset: int, _limit: int) -> void:
	printerr("This must be override")
