class_name PlaceholderManager

## Lazily instantiates a placeholder's scene and frees it again once it has been out of use
## for a while.
##
## A plain RefCounted with no coroutines, on purpose: `placeholder` and `instance` belong to
## the owner's tree, and a RefCounted that awaits outlives that tree (its suspended call keeps
## it alive) and resumes on freed nodes. The owner - a Node - drives the sleep timer instead,
## so every callback dies with the tree that holds the instance.

enum STATUS { UNLOADED, LOADED, SLEEPING }

var placeholder: Node
var instance: Node
var status := STATUS.UNLOADED


func _init(_placeholder: Node) -> void:
	placeholder = _placeholder


## Returns the instance, creating it from the placeholder when there is none. Cancels a
## pending sleep.
func instantiate() -> Node:
	if instance == null:
		instance = placeholder.create_instance()
		instance.set_name(placeholder.get_name() + "_instance")
	status = STATUS.LOADED
	return instance


## Marks the instance as out of use. The owner calls free_if_sleeping() later; showing the
## instance again in between (instantiate / wake) cancels that.
func put_to_sleep() -> void:
	if status == STATUS.LOADED:
		status = STATUS.SLEEPING


func wake() -> void:
	if status == STATUS.SLEEPING:
		status = STATUS.LOADED


func free_if_sleeping() -> void:
	if status == STATUS.SLEEPING:
		queue_free_instance()


func queue_free_instance() -> void:
	if instance != null:
		instance.queue_free()
		instance = null
	status = STATUS.UNLOADED
