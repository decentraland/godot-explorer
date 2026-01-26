class_name SingleInstanceManager
extends RefCounted

var _avatar_preview_instance: AvatarPreview = null
var _avatar_preview_scene = load("res://src/ui/components/backpack/avatar_preview.tscn")
var _current_container: Node = null


func get_avatar_preview() -> AvatarPreview:
	return _get_or_create_avatar_preview()


func handle_visibility_change(container: Node, is_visible: bool) -> void:
	if not is_visible:
		# Do nothing when hiding - tree_exiting will handle reparenting to root when container is deleted
		return
	
	var preview = _get_or_create_avatar_preview()
	if not is_instance_valid(preview):
		return
	
	_ensure_tree_exiting_connected(preview)
	_reparent_to_container_safe(preview, container)




func _reparent_to_container_safe(preview: AvatarPreview, container: Node) -> void:
	if not is_instance_valid(container) or not _can_reparent_to_container(container):
		call_deferred("_deferred_reparent_to_container", preview, container)
		return
	
	_reparent_to_container(preview, container)




# Legacy function for backward compatibility
func reparent_avatar_preview(container: Node) -> AvatarPreview:
	var preview = _get_or_create_avatar_preview()
	if not is_instance_valid(preview):
		return null
	
	_ensure_tree_exiting_connected(preview)
	_reparent_to_container_safe(preview, container)
	return preview


func _get_or_create_avatar_preview() -> AvatarPreview:
	# Check if we have a valid saved reference
	if is_instance_valid(_avatar_preview_instance) and _avatar_preview_instance.is_inside_tree():
		var parent = _avatar_preview_instance.get_parent()
		var parent_name: String = parent.name if is_instance_valid(parent) else "null"
		print("Using existing avatar_preview from reference, parent: '", parent_name, "'")
		return _avatar_preview_instance
	
	# Clear invalid reference
	if not is_instance_valid(_avatar_preview_instance) or not _avatar_preview_instance.is_inside_tree():
		_avatar_preview_instance = null
	
	# Search in tree
	var found_preview = _find_avatar_preview_in_tree(Global.get_tree().root)
	if is_instance_valid(found_preview):
		var parent = found_preview.get_parent()
		var parent_name: String = parent.name if is_instance_valid(parent) else "null"
		print("Found avatar_preview in tree, parent: '", parent_name, "'")
		_avatar_preview_instance = found_preview
		return found_preview
	
	# Create new instance
	print("AvatarPreview does not exist, creating new instance")
	_avatar_preview_instance = _avatar_preview_scene.instantiate()
	var viewport = Global.get_viewport()
	if is_instance_valid(viewport):
		viewport.add_child(_avatar_preview_instance)
	
	return _avatar_preview_instance


func _ensure_tree_exiting_connected(preview: AvatarPreview) -> void:
	if not is_instance_valid(preview):
		return
	
	if preview.tree_exiting.is_connected(_on_avatar_preview_tree_exiting):
		preview.tree_exiting.disconnect(_on_avatar_preview_tree_exiting)
	
	preview.tree_exiting.connect(_on_avatar_preview_tree_exiting)


func _can_reparent_to_container(container: Node) -> bool:
	return container.is_inside_tree() and not _is_node_or_ancestor_queued_for_deletion(container)


func _reparent_to_container(preview: AvatarPreview, container: Node) -> void:
	var current_parent = preview.get_parent()
	
	if current_parent == container:
		print("AvatarPreview already in correct container: '", container.name, "'")
		_ensure_visible(preview)
		return
	
	var current_parent_name = current_parent.name if is_instance_valid(current_parent) else "null"
	print("Reparenting avatar_preview from '", current_parent_name, "' to '", container.name, "'")
	
	_disconnect_previous_container(preview)
	
	# Reparent immediately if current parent is valid, otherwise defer
	if is_instance_valid(current_parent) and not current_parent.is_queued_for_deletion():
		preview.reparent(container)
		preview._apply_layout()
		print("AvatarPreview reparented immediately")
	else:
		print("Current parent invalid or being deleted, using call_deferred")
		call_deferred("_deferred_reparent_to_container", preview, container)
		return
	
	_ensure_visible(preview)
	_current_container = container
	print("Current container updated to: '", container.name, "'")
	_monitor_container_for_deletion(container, preview)


func _disconnect_previous_container(preview: AvatarPreview) -> void:
	if is_instance_valid(_current_container) and _current_container.tree_exiting.is_connected(_on_container_tree_exiting.bind(preview)):
		_current_container.tree_exiting.disconnect(_on_container_tree_exiting.bind(preview))


func _ensure_visible(preview: AvatarPreview) -> void:
	if preview is CanvasItem and not preview.visible:
		preview.show()


func _monitor_container_for_deletion(container: Node, preview: AvatarPreview) -> void:
	if is_instance_valid(container) and not container.tree_exiting.is_connected(_on_container_tree_exiting.bind(preview)):
		container.tree_exiting.connect(_on_container_tree_exiting.bind(preview))


func _on_container_tree_exiting(preview: AvatarPreview) -> void:
	if not is_instance_valid(preview):
		return
	
	var current_parent = preview.get_parent()
	if is_instance_valid(current_parent) and current_parent == _current_container:
		print("Current container being deleted, reparenting avatar_preview to root")
		var root = Global.get_tree().root
		if is_instance_valid(root):
			call_deferred("_deferred_reparent_to_root_from_container", preview)
			preview.hide()
	else:
		var current_parent_name: String = ""
		if is_instance_valid(current_parent):
			current_parent_name = current_parent.name
		else:
			current_parent_name = "null"
		
		var saved_container_name: String = ""
		if is_instance_valid(_current_container):
			saved_container_name = _current_container.name
		else:
			saved_container_name = "null"
		
		print("Container being deleted but not current container (current parent: '", current_parent_name, "', saved container: '", saved_container_name, "')")


func _on_avatar_preview_tree_exiting() -> void:
	if not is_instance_valid(_avatar_preview_instance):
		return
	
	var parent = _avatar_preview_instance.get_parent()
	if not is_instance_valid(parent) or parent.is_queued_for_deletion():
		print("AvatarPreview tree_exiting detected, reparenting to root")
		detach_node(_avatar_preview_instance)


func detach_node(node: Node) -> void:
	if not is_instance_valid(node):
		return
	
	var root = Global.get_tree().root
	if not is_instance_valid(root):
		return
	
	var current_parent = node.get_parent()
	if current_parent != root:
		var current_parent_name: String = current_parent.name if is_instance_valid(current_parent) else "null"
		node.reparent(root)
		print("AvatarPreview reparented to root from: '", current_parent_name, "'")
		if node is AvatarPreview:
			_avatar_preview_instance = node as AvatarPreview
		_current_container = null
	else:
		print("AvatarPreview already in root, no reparenting needed")
	
	if node is CanvasItem:
		var canvas_item = node as CanvasItem
		canvas_item.hide()
		print("AvatarPreview hidden")


func _deferred_reparent_to_container(preview: AvatarPreview, container: Node) -> void:
	if not is_instance_valid(preview) or not is_instance_valid(container):
		print("_deferred_reparent_to_container: preview or container invalid")
		return
	
	if not container.is_inside_tree():
		print("Container '", container.name, "' still not in tree, retrying next frame")
		call_deferred("_deferred_reparent_to_container", preview, container)
		return
	
	if _is_node_or_ancestor_queued_for_deletion(container):
		print("Container '", container.name, "' still being deleted, retrying next frame")
		call_deferred("_deferred_reparent_to_container", preview, container)
		return
	
	if preview.get_parent() != container:
		preview.reparent(container)
		preview._apply_layout()
		print("AvatarPreview reparented to container (deferred): '", container.name, "'")
		_ensure_visible(preview)
		_current_container = container
		if preview is AvatarPreview:
			_avatar_preview_instance = preview as AvatarPreview
		_monitor_container_for_deletion(container, preview)
	else:
		print("AvatarPreview already in correct container (deferred check)")
		_ensure_visible(preview)


func _deferred_reparent_to_root_from_container(preview: AvatarPreview) -> void:
	if not is_instance_valid(preview):
		return
	
	var root = Global.get_tree().root
	if not is_instance_valid(root):
		return
	
	var current_parent = preview.get_parent()
	if current_parent != root:
		preview.reparent(root)
		print("AvatarPreview reparented to root from container (deferred)")
		_current_container = null
		if preview is AvatarPreview:
			_avatar_preview_instance = preview as AvatarPreview


func _is_node_or_ancestor_queued_for_deletion(node: Node) -> bool:
	var current = node
	while is_instance_valid(current):
		if current.is_queued_for_deletion():
			return true
		current = current.get_parent()
	return false


func _find_avatar_preview_in_tree(node: Node) -> AvatarPreview:
	if node is AvatarPreview:
		return node as AvatarPreview

	for child in node.get_children():
		var result = _find_avatar_preview_in_tree(child)
		if is_instance_valid(result):
			return result

	return null
