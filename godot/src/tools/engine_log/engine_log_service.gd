extends Node

## Owns the EngineLogTap logger and the floating engine log overlay.
## Installing starts capturing logs and opens the overlay.
## Uninstalling removes the OS logger and closes the overlay.

const ENGINE_LOG_OVERLAY_SCRIPT = preload("res://src/tools/engine_log/engine_log_overlay.gd")
const ENGINE_LOG_OVERLAY_NODE_NAME := "EngineLogOverlay"

var _logger: EngineLogTap = null
var _overlay: CanvasLayer = null
var _installed := false


func _ready() -> void:
	process_mode = Node.PROCESS_MODE_ALWAYS


func _exit_tree() -> void:
	uninstall()


func set_installed(value: bool) -> void:
	if value:
		install()
	else:
		uninstall()


func is_installed() -> bool:
	return _installed


func install() -> void:
	if _installed:
		_ensure_overlay()
		return

	_logger = EngineLogTap.new()
	OS.add_logger(_logger)
	_installed = true

	_ensure_overlay()


func uninstall() -> void:
	if _installed and _logger != null:
		OS.remove_logger(_logger)

	_logger = null
	_installed = false

	_remove_overlay()


func get_lines() -> Array[String]:
	if _logger == null:
		return []

	return _logger.get_lines()


func clear() -> void:
	if _logger != null:
		_logger.clear()


func get_dropped_count() -> int:
	if _logger == null:
		return 0

	return _logger.get_dropped_count()


func _ensure_overlay() -> void:
	if get_tree() == null:
		return

	if is_instance_valid(_overlay):
		_overlay.show()

		if _overlay.has_method("setup"):
			_overlay.call("setup", self)

		return

	var existing := get_tree().root.get_node_or_null(ENGINE_LOG_OVERLAY_NODE_NAME)

	if existing is CanvasLayer:
		_overlay = existing
		_overlay.show()

		if _overlay.has_method("setup"):
			_overlay.call("setup", self)

		return

	var overlay := ENGINE_LOG_OVERLAY_SCRIPT.new() as CanvasLayer

	if overlay == null:
		push_error("engine_log_overlay.gd must extend CanvasLayer.")
		return

	overlay.name = ENGINE_LOG_OVERLAY_NODE_NAME
	get_tree().root.add_child(overlay)

	_overlay = overlay

	if overlay.has_method("setup"):
		overlay.call("setup", self)


func _remove_overlay() -> void:
	var overlay := _overlay
	_overlay = null

	if is_instance_valid(overlay):
		overlay.queue_free()
