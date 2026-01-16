class_name PlaceholderManager

enum STATUS { UNLOADED, LOADING, LOADED, SLEEPING }

var placeholder: Node
var instance: Node

var sleep_time := 20000.0  # In milliseconds
var status := STATUS.UNLOADED


func _init(_placeholder: Node) -> void:
	placeholder = _placeholder


## If state is LOADING waits for the instance to load.
## If state is LOADED returns the instance.
## If state is SLEEPING cancel sleep and returns the instance.
## If status is UNLOADED creates a new instance.
func _async_instantiate() -> Node:
	if status == STATUS.LOADED:
		return instance
	if status == STATUS.SLEEPING:
		status = STATUS.LOADED
		return instance
	if status == STATUS.LOADING:
		while 1:
			await Engine.get_main_loop().process_frame
			if status == STATUS.LOADED or instance:
				return instance

	status = STATUS.LOADING
	return await _instantiate()


## Creates and instance from the placeholder
func _instantiate() -> Node:
	if not instance:
		instance = placeholder.create_instance()
		status = STATUS.LOADED
	return instance


## Starts a timer to free the instance
func async_put_to_sleep() -> void:
	if status != STATUS.LOADED:
		return
	status = STATUS.SLEEPING
	var start_time = Time.get_ticks_msec()
	while 1:
		await Engine.get_main_loop().process_frame
		if status != STATUS.SLEEPING:
			return
		if Time.get_ticks_msec() - start_time > sleep_time:
			break
	_remove_instance()


## Frees the instance
func _remove_instance() -> void:
	if status != STATUS.SLEEPING:
		return
	if instance:
		instance.queue_free()
		instance = null
	status = STATUS.UNLOADED
