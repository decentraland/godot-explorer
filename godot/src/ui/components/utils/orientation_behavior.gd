## Attach directly to a node to apply orientation-driven behaviors:
## visibility toggling, BoxContainer direction flipping, and/or reparenting.
@tool
class_name OrientationBehavior
extends Node

@export_group("Visibility")
@export var hide_on_portrait: bool = false
@export var hide_on_landscape: bool = false

@export_group("Container Direction")
@export var control_direction: bool = false
@export var invert_direction: bool = false

@export_group("Reparent")
@export var portrait_parent: Node
@export var landscape_parent: Node
@export var reparent_to_front: bool = false

var _watcher: OrientationWatcher
var _original_visible: bool = true
var _original_vertical: bool = false


func _ready() -> void:
	var node = self
	if Engine.is_editor_hint():
		if node is CanvasItem:
			_original_visible = node.visible
		if node is BoxContainer:
			_original_vertical = node.vertical
	_watcher = OrientationWatcher.new()
	_watcher.orientation_changed.connect(_on_orientation_changed)
	add_child(_watcher)


func _notification(what: int) -> void:
	if not Engine.is_editor_hint():
		return
	var node = self
	if what == NOTIFICATION_EDITOR_PRE_SAVE:
		if (hide_on_portrait or hide_on_landscape) and node is CanvasItem:
			node.visible = _original_visible
		if control_direction and node is BoxContainer:
			node.vertical = _original_vertical
	elif what == NOTIFICATION_EDITOR_POST_SAVE:
		if _watcher:
			_on_orientation_changed(_watcher.get_is_portrait())


func _do_reparent(target: Node) -> void:
	# OrientationBehavior IS the node being reparented (not its parent).
	# Calling get_parent().reparent() fails with owner errors at runtime.
	reparent(target, false)
	if reparent_to_front:
		get_parent().move_child(self, 0)


func _on_orientation_changed(is_portrait: bool) -> void:
	var node = self

	# Visibility
	if hide_on_portrait or hide_on_landscape:
		if Engine.is_editor_hint() and not OrientationWatcher.is_editor_preview_active():
			if node is CanvasItem:
				node.visible = true
		elif node is CanvasItem:
			node.visible = (
				not (hide_on_portrait and is_portrait)
				and not (hide_on_landscape and not is_portrait)
			)

	# Container direction
	if control_direction and node is BoxContainer:
		node.vertical = is_portrait if not invert_direction else not is_portrait

	# Reparent (runtime only)
	if not Engine.is_editor_hint() and (portrait_parent or landscape_parent):
		var target: Node = portrait_parent if is_portrait else landscape_parent
		if target != null and target != self and get_parent() != target:
			if is_ancestor_of(target):
				push_error(
					(
						"OrientationBehavior: cannot reparent '%s' to '%s' — target is a descendant."
						% [name, target.name]
					)
				)
				return
			_do_reparent.call_deferred(target)
