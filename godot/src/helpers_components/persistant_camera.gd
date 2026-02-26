extends Camera3D

var _fallback_viewport: Viewport = null


func _on_tree_exiting() -> void:
	_fallback_viewport = get_viewport()
	_deferred_safety_reparent.call_deferred()


func _deferred_safety_reparent() -> void:
	# If we're already back in the tree (e.g. after a valid reparent), nothing to do
	if is_inside_tree():
		return
	# Otherwise we're orphaned — reparent to viewport as a safety net
	if is_instance_valid(_fallback_viewport) and _fallback_viewport.is_inside_tree():
		_fallback_viewport.add_child(self)
