## Coalesces per-GltfContainer load-timeout Timers (1419 on Genesis Plaza).
## Must NOT live on the DclGlobal autoload directly — adding `_process` to
## DclGlobal reproducibly crashes libgodot_android.so (Samsung A54).
class_name GltfLoadTimeoutCoalescer
extends Node

var _deadlines: Dictionary = {}


func schedule(container: Node, timeout_ms: int) -> void:
	_deadlines[container.get_instance_id()] = Time.get_ticks_msec() + timeout_ms


func cancel(container: Node) -> void:
	_deadlines.erase(container.get_instance_id())


func _process(_delta: float) -> void:
	if _deadlines.is_empty():
		return
	var now: int = Time.get_ticks_msec()
	var fired: Array = []
	for iid in _deadlines:
		if _deadlines[iid] <= now:
			fired.append(iid)
	for iid in fired:
		_deadlines.erase(iid)
		var node = instance_from_id(iid)
		if is_instance_valid(node) and node.has_method("_on_load_timeout"):
			node._on_load_timeout()
