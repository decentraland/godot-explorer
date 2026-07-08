## Replaces the per-gltf_container Timer (1419 of them in Genesis Plaza).
## Single Dictionary of deadlines, drained from a dedicated child node's
## `_process` — NOT on Global itself, since adding `_process` to the
## DclGlobal autoload reproducibly triggers a VkThread NULL-deref crash
## inside libgodot_android.so on Android (Samsung A54). Bisected.
class_name GltfLoadTimeoutCoalescer
extends Node

# container InstanceId (int) -> deadline in Time.get_ticks_msec().
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
