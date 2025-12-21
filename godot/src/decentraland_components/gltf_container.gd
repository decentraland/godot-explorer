extends DclGltfContainer

enum GltfContainerLoadingState {
	UNKNOWN = 0,
	LOADING = 1,
	NOT_FOUND = 2,
	FINISHED_WITH_ERROR = 3,
	FINISHED = 4,
}

const MAX_CONCURRENT_LOADS := 10

var dcl_gltf_hash := ""

@onready var timer = $Timer

# Static variable to track currently loading assets
static var currently_loading_assets := []
# Static queue and flag for throttling
static var pending_load_queue := []


func _ready():
	# Connect to ContentProvider2 signals
	if not Global.content_provider2.gltf_ready.is_connected(_async_on_gltf_ready):
		Global.content_provider2.gltf_ready.connect(_async_on_gltf_ready)
	if not Global.content_provider2.gltf_error.is_connected(_on_gltf_error):
		Global.content_provider2.gltf_error.connect(_on_gltf_error)

	self.async_load_gltf.call_deferred()


func _exit_tree():
	# Remove from pending queue if we're being freed
	if pending_load_queue.has(self):
		pending_load_queue.erase(self)


# Helper to handle finishing a load (success or error)
func _finish_gltf_load(gltf_hash: String):
	currently_loading_assets.erase(gltf_hash)
	_process_next_gltf_load()


static func _process_next_gltf_load():
	while currently_loading_assets.size() < MAX_CONCURRENT_LOADS and pending_load_queue.size() > 0:
		# Prioritize GLTFs from the current scene
		var idx = -1
		for i in range(pending_load_queue.size()):
			var candidate = pending_load_queue[i]
			if candidate != null and is_instance_valid(candidate) and candidate.is_current_scene():
				idx = i
				break
		var next_gltf = null
		if idx != -1:
			next_gltf = pending_load_queue[idx]
			pending_load_queue.remove_at(idx)
		else:
			next_gltf = pending_load_queue.pop_front()
		if next_gltf != null and is_instance_valid(next_gltf):
			next_gltf._start_load()
		else:
			break


func is_current_scene():
	if dcl_scene_id == Global.scene_runner.get_current_parcel_scene_id():
		return true
	return false


func async_load_gltf():
	var content_mapping := Global.scene_runner.get_scene_content_mapping(dcl_scene_id)

	self.dcl_gltf_src = dcl_gltf_src.to_lower()
	var file_hash = content_mapping.get_hash(dcl_gltf_src)
	self.dcl_gltf_hash = file_hash

	if file_hash.is_empty():
		dcl_gltf_loading_state = GltfContainerLoadingState.NOT_FOUND
		timer.stop()
		return

	dcl_gltf_loading_state = GltfContainerLoadingState.LOADING
	timer.start()

	# Throttling: If already loading max, queue and return
	if currently_loading_assets.size() >= MAX_CONCURRENT_LOADS:
		if not pending_load_queue.has(self):
			pending_load_queue.append(self)
		return

	_start_load()


func _start_load():
	var content_mapping := Global.scene_runner.get_scene_content_mapping(dcl_scene_id)

	# Add asset to loading list
	if not currently_loading_assets.has(dcl_gltf_hash):
		currently_loading_assets.append(dcl_gltf_hash)

	# Request load via ContentProvider2
	# Note: Colliders are created with mask=0, we set masks after instantiating
	Global.content_provider2.load_scene_gltf(dcl_gltf_src, content_mapping)


# Called when a GLTF is ready to be loaded from disk
func _async_on_gltf_ready(file_hash: String, scene_path: String):
	if file_hash != self.dcl_gltf_hash:
		return  # Not for us

	# Check if we're still valid
	if not is_instance_valid(self) or not is_inside_tree():
		_finish_gltf_load(file_hash)
		return

	var main_tree = get_tree()
	if not is_instance_valid(main_tree):
		dcl_gltf_loading_state = GltfContainerLoadingState.FINISHED_WITH_ERROR
		timer.stop()
		_finish_gltf_load(file_hash)
		return

	# Load the scene file using ResourceLoader
	var err = ResourceLoader.load_threaded_request(scene_path)
	if err != OK:
		printerr("Failed to request load for: ", scene_path)
		dcl_gltf_loading_state = GltfContainerLoadingState.FINISHED_WITH_ERROR
		timer.stop()
		_finish_gltf_load(file_hash)
		return

	# Wait for load to complete
	await _async_wait_for_resource_load(scene_path)

	# Check again if we're still valid after await
	if not is_instance_valid(self) or not is_inside_tree():
		_finish_gltf_load(file_hash)
		return

	# Instantiate the loaded resource
	var resource = ResourceLoader.load_threaded_get(scene_path)
	if resource == null:
		printerr("Failed to load resource: ", scene_path)
		dcl_gltf_loading_state = GltfContainerLoadingState.FINISHED_WITH_ERROR
		timer.stop()
		_finish_gltf_load(file_hash)
		return

	var gltf_node = resource.instantiate()
	apply_fixes(gltf_node)
	# Set collision masks and metadata (colliders are created with mask=0 initially)
	set_mask_colliders(
		gltf_node, dcl_visible_cmask, dcl_invisible_cmask, dcl_scene_id, dcl_entity_id
	)
	dcl_pending_node = gltf_node
	timer.stop()
	_finish_gltf_load(file_hash)


func _async_wait_for_resource_load(path: String):
	var main_tree = get_tree()
	if not is_instance_valid(main_tree):
		return

	var status = ResourceLoader.load_threaded_get_status(path)
	while status == ResourceLoader.THREAD_LOAD_IN_PROGRESS:
		await main_tree.process_frame
		if not is_instance_valid(self) or not is_inside_tree():
			return
		status = ResourceLoader.load_threaded_get_status(path)


# Called when a GLTF fails to load
func _on_gltf_error(file_hash: String, error: String):
	if file_hash != self.dcl_gltf_hash:
		return  # Not for us

	printerr("GLTF load error for ", dcl_gltf_src, ": ", error)
	dcl_gltf_loading_state = GltfContainerLoadingState.FINISHED_WITH_ERROR
	timer.stop()
	_finish_gltf_load(file_hash)


func apply_fixes(gltf_instance: Node3D):
	var meshes = []
	var children = gltf_instance.get_children()
	while children.size():
		var child = children.pop_back()
		if child is MeshInstance3D:
			meshes.push_back(child)
		var grandchildren = child.get_children()
		for grandchild in grandchildren:
			children.push_back(grandchild)

	for instance in meshes:
		var mesh = instance.mesh
		if mesh == null:
			continue
		for idx in range(mesh.get_surface_count()):
			var material = mesh.surface_get_material(idx)
			if material is BaseMaterial3D:
				fix_material(material, instance.name)


func fix_material(mat: BaseMaterial3D, _mesh_name: String = ""):
	# Induced rules for metallic specular roughness
	# - If material has metallic texture then metallic value should be
	# multiplied by .5
	if mat.metallic_texture:
		mat.metallic *= .5

	# To replicate foundation
	mat.vertex_color_use_as_albedo = false


func async_deferred_add_child():
	var new_gltf_node = dcl_pending_node
	dcl_pending_node = null

	# Corner case, when the scene is unloaded before the gltf is loaded
	if not is_inside_tree():
		dcl_gltf_loading_state = GltfContainerLoadingState.FINISHED_WITH_ERROR
		timer.stop()
		return

	var main_tree = get_tree()
	if not is_instance_valid(main_tree):
		dcl_gltf_loading_state = GltfContainerLoadingState.FINISHED_WITH_ERROR
		timer.stop()
		return

	add_child(new_gltf_node)

	await main_tree.process_frame

	# Colliders and rendering is ensured to be ready at this point
	dcl_gltf_loading_state = GltfContainerLoadingState.FINISHED
	timer.stop()

	self.check_animations()


func get_static_body_3d(mesh_instance: MeshInstance3D):
	for maybe_static_body in mesh_instance.get_children():
		if maybe_static_body is StaticBody3D:
			return maybe_static_body

	return null


# Set collision masks and metadata on all colliders after instantiating
func set_mask_colliders(
	node_to_inspect: Node, visible_cmask: int, invisible_cmask: int, scene_id: int, entity_id: int
):
	for node in node_to_inspect.get_children():
		if node is MeshInstance3D:
			var static_body_3d = get_static_body_3d(node)
			if static_body_3d != null:
				# Check if this is an invisible collider mesh (metadata set during GLTF processing)
				var invisible_mesh = (
					static_body_3d.has_meta("invisible_mesh")
					and static_body_3d.get_meta("invisible_mesh") == true
				)

				var mask: int = 0
				if invisible_mesh:
					mask = invisible_cmask
				else:
					mask = visible_cmask

				static_body_3d.set_meta("dcl_col", mask)
				static_body_3d.set_meta("dcl_scene_id", scene_id)
				static_body_3d.set_meta("dcl_entity_id", entity_id)
				static_body_3d.collision_layer = mask
				static_body_3d.collision_mask = 0
				if mask == 0:
					static_body_3d.process_mode = Node.PROCESS_MODE_DISABLED
				else:
					static_body_3d.process_mode = Node.PROCESS_MODE_INHERIT

		set_mask_colliders(node, visible_cmask, invisible_cmask, scene_id, entity_id)


func update_mask_colliders(node_to_inspect: Node):
	for node in node_to_inspect.get_children():
		if node is MeshInstance3D:
			var static_body_3d = get_static_body_3d(node)
			if static_body_3d != null:
				# Check if this is an invisible collider mesh
				var invisible_mesh = (
					static_body_3d.has_meta("invisible_mesh")
					and static_body_3d.get_meta("invisible_mesh") == true
				)

				var mask: int = 0
				if invisible_mesh:
					mask = dcl_invisible_cmask
				else:
					mask = dcl_visible_cmask

				static_body_3d.collision_layer = mask
				static_body_3d.collision_mask = 0
				static_body_3d.set_meta("dcl_col", mask)
				if mask == 0:
					static_body_3d.process_mode = Node.PROCESS_MODE_DISABLED
				else:
					static_body_3d.process_mode = Node.PROCESS_MODE_INHERIT

		update_mask_colliders(node)


func change_gltf(new_gltf, visible_meshes_collision_mask, invisible_meshes_collision_mask):
	var gltf_node = self.get_gltf_resource()
	if self.dcl_gltf_src != new_gltf:
		dcl_gltf_loading_state = GltfContainerLoadingState.LOADING
		timer.start()

		self.dcl_gltf_src = new_gltf
		dcl_visible_cmask = visible_meshes_collision_mask
		dcl_invisible_cmask = invisible_meshes_collision_mask

		if gltf_node != null:
			remove_child(gltf_node)
			gltf_node.queue_free()

		dcl_pending_node = null

		self.async_load_gltf.call_deferred()
	else:
		if (
			(
				visible_meshes_collision_mask != dcl_visible_cmask
				or invisible_meshes_collision_mask != dcl_invisible_cmask
			)
			and gltf_node != null
		):
			dcl_visible_cmask = visible_meshes_collision_mask
			dcl_invisible_cmask = invisible_meshes_collision_mask
			update_mask_colliders(gltf_node)


func _on_timer_timeout():
	printerr("gltf loading timeout ", dcl_gltf_src)
	dcl_gltf_loading_state = GltfContainerLoadingState.FINISHED_WITH_ERROR
