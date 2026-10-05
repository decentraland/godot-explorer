class_name CarrouselGenerator
extends Node

@warning_ignore("unused_signal")
signal report_loading_status(status: LoadingStatus)
signal item_pressed(data)

enum LoadingStatus { LOADING, OK_WITH_RESULTS, OK_WITHOUT_RESULTS, ERROR }

@export var discover: Discover = null

var item_container: Container = null

var search_param: String:
	set(new_value):
		_new_search = true
		search_param = new_value

var _new_search: bool = true


func clean_items():
	printerr("This must be override")


func on_request(_offset: int, _limit: int) -> void:
	printerr("This must be override")


## True while `item_container` is a live node that is STILL INSIDE THE SCENE TREE.
##
## Async builders must re-check this after every `await` - `is_instance_valid()` alone is not
## enough. Leaving a page calls `change_scene_to_file()`, which detaches the page immediately
## while its `queue_free()` only runs later, so `is_instance_valid(item_container)` still
## answers true for a container that has already left the tree.
##
## That gap is what crashes release builds: `add_child()` onto a detached parent never fires
## `_ready` on the child, so every `@onready` in it stays null. The first statically typed call
## on one of those nulls compiles to `OPCODE_CALL_METHOD_BIND`, whose null/freed check is
## `#ifdef DEBUG_ENABLED` - a debug build prints "Cannot call method 'x' on a null value", a
## release build dereferences null and segfaults (observed: SIGSEGV at
## `Control::get_theme_stylebox`, fault addr 0x400, on a cold start where DISCOVER was still
## loading when the user entered a scene).
func can_populate() -> bool:
	return is_instance_valid(item_container) and item_container.is_inside_tree()
