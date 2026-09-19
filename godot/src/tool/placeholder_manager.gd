class_name PlaceholderManager

## Lazily instantiates a placeholder's scene and frees it when its owner says so.
##
## A plain RefCounted with no coroutines and no timers, on purpose: `placeholder` and
## `instance` belong to the owner's tree, and only the owner - a Node - knows when the
## instance stops being needed (the Menu frees every screen when it closes). Nothing here
## can outlive that tree or resume on a freed node.

var placeholder: Node
var instance: Node


func _init(_placeholder: Node) -> void:
	placeholder = _placeholder


## Returns the instance, creating it from the placeholder when there is none.
func instantiate() -> Node:
	if instance == null:
		instance = placeholder.create_instance()
		instance.set_name(placeholder.get_name() + "_instance")
	return instance


func queue_free_instance() -> void:
	if instance != null:
		instance.queue_free()
		instance = null
