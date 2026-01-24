class_name SingleInstanceManager
extends RefCounted

var _avatar_preview_instance: AvatarPreview = null
var _avatar_preview_scene = load("res://src/ui/components/backpack/avatar_preview.tscn")
var _current_container: Node = null  # Guardar referencia al contenedor actual


func reparent_avatar_preview(container: Node) -> AvatarPreview:
	# Primero verificar si tenemos una referencia válida guardada
	var found_preview: AvatarPreview = null
	
	if is_instance_valid(_avatar_preview_instance):
		# Verificar que todavía esté en el árbol
		if _avatar_preview_instance.is_inside_tree():
			found_preview = _avatar_preview_instance
			print("Usando avatar_preview existente de referencia: ", found_preview.get_path())
		else:
			# La referencia ya no es válida, limpiarla
			_avatar_preview_instance = null
	
	# Si no encontramos uno válido, buscar en el árbol
	if not is_instance_valid(found_preview):
		found_preview = _find_avatar_preview_in_tree(Global.get_tree().root)
		if is_instance_valid(found_preview):
			print("Encontrado avatar_preview en árbol: ", found_preview.get_path())
			_avatar_preview_instance = found_preview
	
	# Si todavía no encontramos uno, crear uno nuevo
	if not is_instance_valid(found_preview):
		print("no existía, creando nuevo")
		_avatar_preview_instance = _avatar_preview_scene.instantiate()
		found_preview = _avatar_preview_instance
		var viewport = Global.get_viewport()
		if is_instance_valid(viewport):
			viewport.add_child(found_preview)
	
	# Conectar la señal tree_exiting para reemparentar al root antes de eliminarse
	if is_instance_valid(found_preview):
		# Desconectar si ya estaba conectada para evitar duplicados
		if found_preview.tree_exiting.is_connected(_on_avatar_preview_tree_exiting):
			found_preview.tree_exiting.disconnect(_on_avatar_preview_tree_exiting)
		# Conectar la señal
		found_preview.tree_exiting.connect(_on_avatar_preview_tree_exiting)
		

	if is_instance_valid(container) and found_preview.get_parent() != container:
		# Desconectar del contenedor anterior si existe
		if is_instance_valid(_current_container) and _current_container != container:
			if _current_container.tree_exiting.is_connected(_on_container_tree_exiting.bind(found_preview)):
				_current_container.tree_exiting.disconnect(_on_container_tree_exiting.bind(found_preview))
		
		found_preview.reparent(container)
		found_preview._apply_layout()
		
		# Mostrar el avatar_preview si estaba oculto
		if found_preview is CanvasItem and not found_preview.visible:
			found_preview.show()
		
		# Actualizar referencia al contenedor actual
		_current_container = container
		
		# Monitorear el nuevo contenedor
		_monitor_container_for_deletion(container, found_preview)

	return found_preview


func _monitor_container_for_deletion(container: Node, preview: AvatarPreview) -> void:
	# Conectar a la señal tree_exiting del contenedor para detectar cuando se elimina
	if is_instance_valid(container) and not container.tree_exiting.is_connected(_on_container_tree_exiting.bind(preview)):
		container.tree_exiting.connect(_on_container_tree_exiting.bind(preview))


func _on_container_tree_exiting(preview: AvatarPreview) -> void:
	# Cuando el contenedor se elimina, verificar que sea realmente el contenedor actual
	if not is_instance_valid(preview):
		return
	
	var current_parent = preview.get_parent()
	
	# Solo reemparentar si:
	# 1. El preview todavía tiene un padre válido
	# 2. El padre es realmente el contenedor que se está eliminando (no otro contenedor)
	if is_instance_valid(current_parent) and current_parent == _current_container:
		print("Contenedor actual eliminándose, reemparentando avatar_preview al root")
		var root = Global.get_tree().root
		if is_instance_valid(root):
			# Usar call_deferred para asegurar que el reemparentado ocurra después
			# de que el árbol se actualice, especialmente cuando cambia la escena completa
			call_deferred("_deferred_reparent_to_root_from_container", preview)
			preview.hide()
	else:
		print("Contenedor eliminándose pero no es el contenedor actual (padre actual: ", current_parent.get_path() if is_instance_valid(current_parent) else "null", ", contenedor guardado: ", _current_container.get_path() if is_instance_valid(_current_container) else "null", ")")


func _on_avatar_preview_tree_exiting() -> void:
	if is_instance_valid(_avatar_preview_instance):
		# Verificar que el nodo realmente esté a punto de salir del árbol
		var parent = _avatar_preview_instance.get_parent()
		if not is_instance_valid(parent) or parent.is_queued_for_deletion():
			print("AvatarPreview tree_exiting detectado, reemparentando al root")
			detach_node(_avatar_preview_instance)


func detach_node(node: Node) -> void:
	if not is_instance_valid(node):
		print("detach_node: nodo no válido")
		return
	
	print("detach_node llamado para: ", node.name if is_instance_valid(node) else "null")
	
	# Reemparentar inmediatamente al root (no usar call_deferred porque puede ser demasiado tarde)
	var root = Global.get_tree().root
	if not is_instance_valid(root):
		print("detach_node: root no válido")
		return
	
	if not is_instance_valid(node):
		print("detach_node: nodo no válido después de verificar root")
		return
	
	var current_parent = node.get_parent()
	print("detach_node: padre actual = ", current_parent.get_path() if is_instance_valid(current_parent) else "null")
	print("detach_node: root = ", root.get_path())
	
	# Solo reemparentar si no está ya en el root
	if current_parent != root:
		# Reemparentar inmediatamente - esto debe ocurrir ANTES de que el nodo se elimine
		node.reparent(root)
		print("✓ AvatarPreview reemparentado al root desde: ", current_parent.get_path() if is_instance_valid(current_parent) else "null")
		# Actualizar la referencia y limpiar contenedor
		if node is AvatarPreview:
			_avatar_preview_instance = node as AvatarPreview
		_current_container = null
	else:
		print("detach_node: ya está en el root, no necesita reemparentar")
	
	# Ocultar el nodo
	if node is CanvasItem:
		node.hide()
		print("AvatarPreview ocultado")


func _deferred_reparent_to_root_from_container(preview: AvatarPreview) -> void:
	if not is_instance_valid(preview):
		return
	
	var root = Global.get_tree().root
	if is_instance_valid(root) and is_instance_valid(preview):
		var current_parent = preview.get_parent()
		if current_parent != root:
			preview.reparent(root)
			print("✓ AvatarPreview reemparentado al root desde contenedor (deferred)")
			# Limpiar referencia al contenedor
			_current_container = null
			# Actualizar referencia
			if preview is AvatarPreview:
				_avatar_preview_instance = preview as AvatarPreview


func _find_avatar_preview_in_tree(node: Node) -> AvatarPreview:
	if node is AvatarPreview:
		return node as AvatarPreview

	for child in node.get_children():
		var result = _find_avatar_preview_in_tree(child)
		if is_instance_valid(result):
			return result

	return null
