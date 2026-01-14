class_name PlaceholderManager

var placeholder: Node
var instance: Node

var sleep_time := 20000.0 # In milliseconds
var status := STATUS.UNLOADED

enum STATUS {
	UNLOADED,
	LOADING,
	LOADED,
	SLEEPING
}

func _init(_placeholder: Node) -> void:
	placeholder = _placeholder


func _async_instantiate() -> Node:
	return await instantiate()


func instantiate() -> Node:
	if status != STATUS.UNLOADED: return instance
	if not instance:
		status = STATUS.LOADING
		instance = placeholder.create_instance()
		status = STATUS.LOADED
	return instance

## Starts a timer to free the instance
func put_to_sleep() -> void:
	if status != STATUS.LOADED: return
	status = STATUS.SLEEPING
	var start_time = Time.get_ticks_msec()
	while 1:
		await Engine.get_main_loop().process_frame
		if Time.get_ticks_msec() - start_time > sleep_time:
			break
		if status != STATUS.SLEEPING:
			return
	_remove_instance()

func _remove_instance() -> void:
	if status != STATUS.SLEEPING: return
	if instance:
		instance.queue_free()
		instance = null
