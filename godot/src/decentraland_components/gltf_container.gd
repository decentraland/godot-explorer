extends DclGltfContainer

enum GltfContainerLoadingState {
	UNKNOWN = 0,
	LOADING = 1,
	NOT_FOUND = 2,
	FINISHED_WITH_ERROR = 3,
	FINISHED = 4,
}

const MAX_CONCURRENT_LOADS := 10

# Debug: Set to true to paint meshes red when switching from STATIC to KINEMATIC
const DEBUG_PAINT_KINEMATIC_BODIES := true

var dcl_gltf_hash := ""

# Transform change tracking for STATIC -> KINEMATIC mode switching
var _last_global_transform: Transform3D
var _transform_change_count := 0
var _colliders_switched_to_kinematic := false
var _has_static_colliders := false  # True if GLTF has colliders that need tracking

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

	# Initialize transform tracking (disabled until GLTF is loaded)
	_last_global_transform = global_transform
	set_process(false)

	self.async_load_gltf.call_deferred()


func _process(_delta: float):
	# Skip if already switched to kinematic or no GLTF loaded yet
	if (
		_colliders_switched_to_kinematic
		or dcl_gltf_loading_state != GltfContainerLoadingState.FINISHED
	):
		return

	# Check if transform has changed
	var current_transform = global_transform
	if current_transform != _last_global_transform:
		_transform_change_count += 1
		_last_global_transform = current_transform

		# First transform change is tolerated (initial positioning)
		# Switch to KINEMATIC after the second change
		if _transform_change_count >= 2:
			_colliders_switched_to_kinematic = true
			var gltf_node = get_gltf_resource()
			if gltf_node != null:
				_switch_colliders_to_kinematic(gltf_node)
			# Stop processing since we've switched
			set_process(false)


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
	# Track if we have any static colliders that need transform monitoring
	_has_static_colliders = set_mask_colliders(
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

	# Only enable transform tracking if we have static colliders that might need
	# to switch to kinematic mode. This skips per-frame overhead for GLTFs without colliders.
	if _has_static_colliders:
		_last_global_transform = global_transform
		_transform_change_count = 0
		_colliders_switched_to_kinematic = false
		set_process(true)

	self.check_animations()


func get_animatable_body_3d(mesh_instance: MeshInstance3D):
	for maybe_body in mesh_instance.get_children():
		if maybe_body is AnimatableBody3D:
			return maybe_body

	return null


# Set collision masks and metadata on all colliders after instantiating
# Also sets STATIC mode for better performance (will switch to KINEMATIC if entity moves)
# Returns true if any colliders were set to STATIC mode
func set_mask_colliders(
	node_to_inspect: Node, visible_cmask: int, invisible_cmask: int, scene_id: int, entity_id: int
) -> bool:
	var has_static := false
	for node in node_to_inspect.get_children():
		if node is MeshInstance3D:
			var body_3d = get_animatable_body_3d(node)
			if body_3d != null:
				# Check if this is an invisible collider mesh (metadata set during GLTF processing)
				var invisible_mesh = (
					body_3d.has_meta("invisible_mesh")
					and body_3d.get_meta("invisible_mesh") == true
				)

				var mask: int = 0
				if invisible_mesh:
					mask = invisible_cmask
				else:
					mask = visible_cmask

				body_3d.set_meta("dcl_col", mask)
				body_3d.set_meta("dcl_scene_id", scene_id)
				body_3d.set_meta("dcl_entity_id", entity_id)
				body_3d.collision_layer = mask
				body_3d.collision_mask = 0
				if mask == 0:
					body_3d.process_mode = Node.PROCESS_MODE_DISABLED
				else:
					body_3d.process_mode = Node.PROCESS_MODE_INHERIT
					# Set STATIC mode for better performance
					# Will switch to KINEMATIC if entity moves (detected in _process)
					PhysicsServer3D.body_set_mode(
						body_3d.get_rid(), PhysicsServer3D.BODY_MODE_STATIC
					)
					body_3d.set_meta("dcl_static_mode", true)
					has_static = true

		if set_mask_colliders(node, visible_cmask, invisible_cmask, scene_id, entity_id):
			has_static = true
	return has_static


func update_mask_colliders(node_to_inspect: Node):
	for node in node_to_inspect.get_children():
		if node is MeshInstance3D:
			var body_3d = get_animatable_body_3d(node)
			if body_3d != null:
				# Check if this is an invisible collider mesh
				var invisible_mesh = (
					body_3d.has_meta("invisible_mesh")
					and body_3d.get_meta("invisible_mesh") == true
				)

				var mask: int = 0
				if invisible_mesh:
					mask = dcl_invisible_cmask
				else:
					mask = dcl_visible_cmask

				body_3d.collision_layer = mask
				body_3d.collision_mask = 0
				body_3d.set_meta("dcl_col", mask)
				if mask == 0:
					body_3d.process_mode = Node.PROCESS_MODE_DISABLED
				else:
					body_3d.process_mode = Node.PROCESS_MODE_INHERIT

		update_mask_colliders(node)


# Shared debug material for kinematic bodies visualization
static var _debug_kinematic_material: StandardMaterial3D = null


static func _get_debug_kinematic_material() -> StandardMaterial3D:
	if _debug_kinematic_material == null:
		_debug_kinematic_material = StandardMaterial3D.new()
		_debug_kinematic_material.albedo_color = Color.RED
		_debug_kinematic_material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
		_debug_kinematic_material.transparency = BaseMaterial3D.TRANSPARENCY_DISABLED
		_debug_kinematic_material.cull_mode = BaseMaterial3D.CULL_DISABLED
		# Add emission to make it really visible
		_debug_kinematic_material.emission_enabled = true
		_debug_kinematic_material.emission = Color.RED
		_debug_kinematic_material.emission_energy_multiplier = 2.0
	return _debug_kinematic_material


# Switch all colliders from STATIC to KINEMATIC mode
# Called when the entity moves (after the 2nd transform change)
func _switch_colliders_to_kinematic(node_to_inspect: Node):
	for node in node_to_inspect.get_children():
		if node is MeshInstance3D:
			var mesh_instance: MeshInstance3D = node
			var body_3d = get_animatable_body_3d(mesh_instance)
			if body_3d != null and body_3d.has_meta("dcl_static_mode"):
				# Switch to KINEMATIC mode via PhysicsServer3D
				PhysicsServer3D.body_set_mode(
					body_3d.get_rid(), PhysicsServer3D.BODY_MODE_KINEMATIC
				)
				body_3d.remove_meta("dcl_static_mode")

			# Debug: Paint visible meshes red (skip collider meshes)
			if DEBUG_PAINT_KINEMATIC_BODIES:
				var is_collider_mesh = mesh_instance.name.to_lower().contains("collider")
				if not is_collider_mesh and mesh_instance.visible:
					var mat = _get_debug_kinematic_material()
					mesh_instance.material_override = mat
					print("[DEBUG] KINEMATIC painted: ", mesh_instance.name)
		_switch_colliders_to_kinematic(node)


func change_gltf(new_gltf, visible_meshes_collision_mask, invisible_meshes_collision_mask):
	var gltf_node = self.get_gltf_resource()
	if self.dcl_gltf_src != new_gltf:
		dcl_gltf_loading_state = GltfContainerLoadingState.LOADING
		timer.start()

		self.dcl_gltf_src = new_gltf
		dcl_visible_cmask = visible_meshes_collision_mask
		dcl_invisible_cmask = invisible_meshes_collision_mask

		# Reset transform tracking for new GLTF
		_transform_change_count = 0
		_colliders_switched_to_kinematic = false
		_has_static_colliders = false
		_last_global_transform = global_transform
		set_process(false)  # Will be re-enabled when GLTF finishes loading (if has colliders)

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
