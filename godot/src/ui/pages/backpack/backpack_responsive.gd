@tool
extends Control

var _is_switching: bool = false

@onready var backpack_portrait := PlaceholderManager.new($BackpackPortrait)
@onready var backpack_landscape := PlaceholderManager.new($BackpackLandscape)


func _ready() -> void:
	backpack_portrait.placeholder.visible = false
	backpack_landscape.placeholder.visible = false
	handle_screen_resize()


func _notification(what: int) -> void:
	if what == NOTIFICATION_RESIZED:
		handle_screen_resize()


func _get_active_manager() -> PlaceholderManager:
	var rect_size := get_viewport_rect().size
	if rect_size.x < rect_size.y:
		return backpack_portrait
	return backpack_landscape


func show_emotes() -> void:
	var manager := _get_active_manager()
	manager.instantiate()
	manager.instance.show_emotes()
	manager.instance.press_button_emotes()


func handle_screen_resize() -> void:
	if not is_node_ready():
		return
	if _is_switching:
		return
	_is_switching = true
	var rect_size := get_viewport_rect().size
	if rect_size.x < rect_size.y:
		if Engine.is_editor_hint():
			$BackpackPortrait.show()
			$BackpackLandscape.hide()
		else:
			backpack_landscape.queue_free_instance()
			backpack_portrait.instantiate()
			backpack_portrait.instance.show()
	else:
		if Engine.is_editor_hint():
			$BackpackPortrait.hide()
			$BackpackLandscape.show()
		else:
			backpack_portrait.queue_free_instance()
			backpack_landscape.instantiate()
			backpack_landscape.instance.show()
	_is_switching = false
