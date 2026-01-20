extends Node

## Singleton que gestiona una única instancia de AvatarPreview globalmente.
## Solo se muestra una vez a la vez, usando posición visual en lugar de reparenting.

var avatar_preview: AvatarPreview = null
var current_target: Control = null  # El contenedor objetivo donde se debe mostrar
var root_canvas_layer: CanvasLayer = null  # Capa de canvas para mostrar sobre todo

func _ready():
	# Crear una capa de canvas para mostrar el preview sobre otros elementos
	root_canvas_layer = CanvasLayer.new()
	root_canvas_layer.name = "AvatarPreviewCanvasLayer"
	root_canvas_layer.layer = 100  # Capa alta para aparecer sobre todo
	get_tree().root.add_child(root_canvas_layer)
	
	# Cargar la escena del avatar preview
	var AvatarPreviewScene = preload("res://src/ui/components/backpack/avatar_preview.tscn")
	avatar_preview = AvatarPreviewScene.instantiate()
	avatar_preview.name = "AvatarPreview"
	# Inicialmente oculto
	avatar_preview.hide()
	# Configurar para que use posición absoluta
	avatar_preview.layout_mode = 0  # LAYOUT_MODE_ANCHORS
	avatar_preview.set_anchors_preset(Control.PRESET_TOP_LEFT)
	root_canvas_layer.add_child(avatar_preview)


## Muestra el avatar preview posicionándolo visualmente sobre el contenedor objetivo
## No reparenta el nodo, solo lo mueve visualmente
func show_preview(target_container: Control, config: Dictionary = {}):
	# Asegurarse de que el preview esté inicializado
	if not avatar_preview:
		var AvatarPreviewScene = preload("res://src/ui/components/backpack/avatar_preview.tscn")
		avatar_preview = AvatarPreviewScene.instantiate()
		avatar_preview.name = "AvatarPreview"
		avatar_preview.layout_mode = 0  # LAYOUT_MODE_ANCHORS
		avatar_preview.set_anchors_preset(Control.PRESET_TOP_LEFT)
		if root_canvas_layer:
			root_canvas_layer.add_child(avatar_preview)
		else:
			# Fallback: agregar al root si no hay canvas layer
			get_tree().root.add_child(avatar_preview)
	
	if not target_container:
		push_error("AvatarPreviewManager: Target container no puede ser null")
		return
	
	if not target_container.is_inside_tree():
		push_error("AvatarPreviewManager: Target container debe estar en el árbol de escena: ", target_container.name)
		return
	
	current_target = target_container
	
	# Aplicar configuración usando el método del preview
	if not config.is_empty():
		avatar_preview.update_configuration(config)
	
	# Posicionar visualmente el preview sobre el contenedor objetivo
	call_deferred("_position_preview_over_target", target_container)
	
	# Asegurarse de que el preview esté visible y completamente opaco
	avatar_preview.show()
	avatar_preview.visible = true
	avatar_preview.modulate.a = 1.0


func _position_preview_over_target(target_container: Control):
	if not avatar_preview or not target_container or not target_container.is_inside_tree():
		return
	
	if not avatar_preview.is_inside_tree():
		return
	
	# Obtener la posición global del contenedor objetivo
	var target_global_pos = target_container.get_global_position()
	var target_size = target_container.size
	
	# Convertir la posición global a posición relativa al canvas layer
	var canvas_layer_pos = root_canvas_layer.get_global_position()
	var relative_pos = target_global_pos - canvas_layer_pos
	
	# Configurar el preview para que cubra el área del contenedor objetivo
	avatar_preview.layout_mode = 0  # LAYOUT_MODE_ANCHORS
	avatar_preview.set_anchors_preset(Control.PRESET_TOP_LEFT)
	avatar_preview.offset_left = relative_pos.x
	avatar_preview.offset_top = relative_pos.y
	avatar_preview.offset_right = relative_pos.x + target_size.x
	avatar_preview.offset_bottom = relative_pos.y + target_size.y
	
	# Asegurarse de que el SubViewportContainer tenga stretch habilitado
	if avatar_preview is SubViewportContainer:
		avatar_preview.stretch = true
	
	# Asegurarse de que esté visible y opaco
	avatar_preview.visible = true
	avatar_preview.modulate.a = 1.0
	
	# Forzar actualización del layout
	avatar_preview.queue_sort()
	avatar_preview.queue_redraw()
	
	# Actualizar posición si el contenedor se mueve
	if not target_container.resized.is_connected(_on_target_resized):
		target_container.resized.connect(_on_target_resized)
	if not target_container.visibility_changed.is_connected(_on_target_visibility_changed):
		target_container.visibility_changed.connect(_on_target_visibility_changed)


func _on_target_resized():
	if current_target and current_target.is_inside_tree():
		_position_preview_over_target(current_target)


func _on_target_visibility_changed():
	if current_target:
		if current_target.visible:
			avatar_preview.show()
		else:
			avatar_preview.hide()


## Oculta el avatar preview sin reparentar
func hide_preview():
	if not avatar_preview:
		return
	
	# Desconectar señales del target anterior
	if current_target:
		if current_target.resized.is_connected(_on_target_resized):
			current_target.resized.disconnect(_on_target_resized)
		if current_target.visibility_changed.is_connected(_on_target_visibility_changed):
			current_target.visibility_changed.disconnect(_on_target_visibility_changed)
	
	# Restaurar opacidad antes de ocultar (por si se usó para snapshots)
	avatar_preview.modulate.a = 1.0
	avatar_preview.hide()
	current_target = null


## Obtiene la instancia del avatar preview
func get_preview() -> AvatarPreview:
	return avatar_preview


## Verifica si el preview está visible
func is_visible() -> bool:
	if not avatar_preview:
		return false
	return avatar_preview.visible


## Reposiciona el preview sobre un nuevo contenedor sin cambiar su visibilidad
func reparent_to(new_target: Control):
	if not avatar_preview:
		return
	
	if new_target and new_target.is_inside_tree():
		show_preview(new_target, {})
