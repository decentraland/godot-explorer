extends Camera3D

signal need_reparent()

func _notification(what: int) -> void:
	if what == NOTIFICATION_PREDELETE:
		self.cancel_free()
		self.need_reparent.emit.call_deferred()

func _on_tree_exiting() -> void:
	self.cancel_free()
	self.need_reparent.emit.call_deferred()
