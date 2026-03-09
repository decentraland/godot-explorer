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
var optimized := false
# Track specific error reason from loading (for better error reporting)
var _last_load_error := ""
# Once entity is known to move, all colliders should spawn as KINEMATIC
var _kinematic_requested := false

@onready var timer = $Timer

# Static variable to track currently loading assets (by hash)
static var currently_loading_assets := []
# Static queue for throttling
static var pending_load_queue := []


func _ready():
	# Check if Rust flagged this entity as kinematic (has active tween)
	if self.has_meta("kinematic_requested"):
		_kinematic_requested = true
	# Connect to switch_to_kinematic signal from Rust
	self.switch_to_kinematic.connect(_on_switch_to_kinematic)
	self.async_load_gltf.call_deferred()


func _exit_tree():
	# Remove from pending queue
	pending_load_queue.erase(self)

	# Free pending orphan node
	if dcl_pending_node != null:
		dcl_pending_node.queue_free()
		dcl_pending_node = null

	# Clean up loading state
	if not dcl_gltf_hash.is_empty():
		currently_loading_assets.erase(dcl_gltf_hash)


#region Loading Flow
# Two loading paths:
# 1. Optimized: Pre-baked scenes from res://glbs/ (loaded via ResourceLoader)
# 2. Runtime: Runtime-processed scenes from user://content/<hash>.scn (promise-based)
#
# Both paths share:
# - Throttling via currently_loading_assets and pending_load_queue
# - ResourceLoader pattern for instantiation
# - Queue prioritization for current scene


func async_load_gltf():
	self.dcl_gltf_src = dcl_gltf_src.to_lower()
	var content_mapping := Global.scene_runner.get_scene_content_mapping(dcl_scene_id)
	var file_hash := content_mapping.get_hash(dcl_gltf_src)
	self.dcl_gltf_hash = file_hash

	if file_hash.is_empty():
		dcl_gltf_loading_state = GltfContainerLoadingState.NOT_FOUND
		return

	dcl_gltf_loading_state = GltfContainerLoadingState.LOADING
	timer.start()

	# Check CLI flags for asset loading mode
	var has_optimized = Global.content_provider.optimized_asset_exists(file_hash)

	# --only-no-optimized: Always use runtime processing, ignore optimized assets
	if Global.cli.only_no_optimized:
		await _async_load_runtime_gltf()
		return

	# --only-optimized: Only use optimized path, skip if not optimized
	if Global.cli.only_optimized:
		if has_optimized:
			await _async_load_optimized_asset(file_hash)
		else:
			# Skip loading - no optimized asset available
			dcl_gltf_loading_state = GltfContainerLoadingState.NOT_FOUND
			timer.stop()
		return

	# Default: Check for optimized asset first (pre-baked in res://glbs/)
	if has_optimized:
		await _async_load_optimized_asset(file_hash)
		return

	# Fall back to runtime processing (saves to user://content/<hash>.scn)
	await _async_load_runtime_gltf()


## Optimized Asset Loading Path
## Pre-baked scenes from res://glbs/<hash>.scn
# gdlint:ignore = async-function-name
func _async_load_optimized_asset(gltf_hash: String):
	self.optimized = true

	# Throttle: queue if at max concurrent loads
	if currently_loading_assets.size() >= MAX_CONCURRENT_LOADS:
		if not pending_load_queue.has(self):
			pending_load_queue.append(self)
		return

	if not currently_loading_assets.has(gltf_hash):
		currently_loading_assets.append(gltf_hash)

	# Download dependencies (textures, etc.)
	var promise = Global.content_provider.fetch_optimized_asset_with_dependencies(gltf_hash)
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		printerr("[GltfContainer] Failed to download optimized asset dependencies: ", gltf_hash)
		_finish_with_error("failed to download optimized asset dependencies")
		return
	# Load from res://glbs/
	var scene_file = "res://glbs/" + gltf_hash + ".scn"
	if not ResourceLoader.exists(scene_file):
		printerr("[GltfContainer] Scene file not found after resource pack load: ", scene_file)
		_finish_with_error("optimized scene not found: " + scene_file)
		return

	var gltf_node := await _async_load_and_instantiate(scene_file)
	if gltf_node == null:
		var reason = (
			_last_load_error
			if not _last_load_error.is_empty()
			else "failed to instantiate optimized scene"
		)
		printerr("[GltfContainer] Failed to instantiate: ", scene_file, " reason: ", reason)
		_finish_with_error(reason)
		return

	# Add to scene tree
	_async_add_gltf_to_tree.call_deferred(gltf_node)


## Runtime GLTF Loading Path (Promise-based)
## Runtime-processed scenes saved to user://content/<hash>.scn
# gdlint:ignore = async-function-name
func _async_load_runtime_gltf():
	self.optimized = false

	# Throttle: queue if at max concurrent loads
	if currently_loading_assets.size() >= MAX_CONCURRENT_LOADS:
		if not pending_load_queue.has(self):
			pending_load_queue.append(self)
		return

	if not currently_loading_assets.has(dcl_gltf_hash):
		currently_loading_assets.append(dcl_gltf_hash)

	var content_mapping := Global.scene_runner.get_scene_content_mapping(dcl_scene_id)
	var promise = Global.content_provider.load_scene_gltf(dcl_gltf_src, content_mapping)

	if promise == null:
		_finish_with_error("failed to start loading")
		return

	# Wait for the promise to resolve
	await PromiseUtils.async_awaiter(promise)

	# Check if we're still in a valid state (scene might have been unloaded)
	if dcl_gltf_loading_state != GltfContainerLoadingState.LOADING:
		return

	if promise.is_rejected():
		var error = promise.get_data()
		var reason = error.get_error() if error is PromiseError else "promise rejected"
		_finish_with_error(reason)
		return

	# Get scene path from promise data
	var scene_path = promise.get_data()
	if not scene_path is String or scene_path.is_empty():
		_finish_with_error("invalid scene path")
		return

	# Load and instantiate the PackedScene
	var gltf_node := await _async_load_and_instantiate(scene_path)
	if gltf_node == null:
		var reason = (
			_last_load_error if not _last_load_error.is_empty() else "failed to instantiate scene"
		)
		_finish_with_error(reason)
		return

	# Add to scene tree
	_async_add_gltf_to_tree.call_deferred(gltf_node)


#endregion

#region Shared Loading Logic


## Load a PackedScene via ResourceLoader and instantiate it
## Used by both optimized and runtime paths
## Sets _last_load_error on failure for specific error reporting
func _async_load_and_instantiate(scene_path: String) -> Node3D:
	_last_load_error = ""

	# Check file exists
	# For res:// paths: use ResourceLoader.exists() which handles both .remap files
	# (exported builds) and dynamically loaded resource packs
	# For user:// paths: use FileAccess.file_exists()
	if scene_path.begins_with("res://"):
		if not ResourceLoader.exists(scene_path):
			_last_load_error = "file not found: " + scene_path
			printerr("GltfContainer: ", _last_load_error)
			return null
	else:
		if not FileAccess.file_exists(scene_path):
			_last_load_error = "file not found: " + scene_path
			printerr("GltfContainer: ", _last_load_error)
			return null

	# Request threaded load
	var err := ResourceLoader.load_threaded_request(scene_path)
	if err != OK:
		_last_load_error = "ResourceLoader request failed (error " + str(err) + ")"
		printerr("GltfContainer: ", _last_load_error, " for ", scene_path)
		return null

	# Wait for load to complete
	var main_tree := get_tree()
	var status := ResourceLoader.load_threaded_get_status(scene_path)
	while status == ResourceLoader.THREAD_LOAD_IN_PROGRESS:
		if not is_instance_valid(main_tree) or not is_inside_tree():
			return null
		await main_tree.process_frame
		status = ResourceLoader.load_threaded_get_status(scene_path)

	# Check for load failures BEFORE trying to get the resource
	if status != ResourceLoader.THREAD_LOAD_LOADED:
		if status == ResourceLoader.THREAD_LOAD_FAILED:
			_last_load_error = "ResourceLoader THREAD_LOAD_FAILED"
		elif status == ResourceLoader.THREAD_LOAD_INVALID_RESOURCE:
			_last_load_error = "ResourceLoader THREAD_LOAD_INVALID_RESOURCE"
		else:
			_last_load_error = "ResourceLoader unexpected status " + str(status)
		printerr("GltfContainer: ", _last_load_error, " for ", scene_path)
		return null

	# Get the loaded resource
	var resource := ResourceLoader.load_threaded_get(scene_path)
	if resource == null:
		_last_load_error = "loaded resource is null"
		printerr("GltfContainer: ", _last_load_error, " for ", scene_path)
		return null

	var gltf_node: Node3D = resource.instantiate()
	apply_fixes(gltf_node)

	# Set collision masks (colliders created with mask=0 initially)
	set_mask_colliders(
		gltf_node, dcl_visible_cmask, dcl_invisible_cmask, dcl_scene_id, dcl_entity_id
	)

	return gltf_node


func _async_add_gltf_to_tree(gltf_node: Node3D):
	# Check if still valid (scene might have been unloaded)
	if not is_inside_tree():
		gltf_node.queue_free()
		_finish_with_error("scene unloaded during load")
		return

	add_child(gltf_node)
	await get_tree().process_frame

	_complete_load()


func _complete_load():
	dcl_gltf_loading_state = GltfContainerLoadingState.FINISHED
	timer.stop()
	_finish_loading_slot()

	self.check_animations()


func _finish_with_error(reason: String = "unknown"):
	printerr("GLTF load error for ", dcl_gltf_src, ": ", reason)
	# Report to resource tracker if we have a valid hash
	if not dcl_gltf_hash.is_empty():
		Global.content_provider.report_resource_failed(dcl_gltf_hash, reason)
	dcl_gltf_loading_state = GltfContainerLoadingState.FINISHED_WITH_ERROR
	timer.stop()
	_finish_loading_slot()


func _finish_loading_slot():
	currently_loading_assets.erase(dcl_gltf_hash)
	_process_next_in_queue()


static func _process_next_in_queue():
	while currently_loading_assets.size() < MAX_CONCURRENT_LOADS and pending_load_queue.size() > 0:
		var next_gltf = _pop_next_from_queue()
		if next_gltf == null:
			break

		# Skip if already finished (received signal early from another container with same hash)
		if next_gltf.dcl_gltf_loading_state != GltfContainerLoadingState.LOADING:
			continue

		# Resume the appropriate loading path
		if next_gltf.optimized:
			next_gltf._async_load_optimized_asset(next_gltf.dcl_gltf_hash)
		else:
			next_gltf._async_load_runtime_gltf()


static func _pop_next_from_queue():
	# Prioritize current scene GLTFs
	for i in range(pending_load_queue.size()):
		var candidate = pending_load_queue[i]
		if is_instance_valid(candidate) and candidate.is_current_scene():
			pending_load_queue.remove_at(i)
			return candidate

	# Fall back to first valid item
	while pending_load_queue.size() > 0:
		var candidate = pending_load_queue.pop_front()
		if is_instance_valid(candidate):
			return candidate

	return null


func is_current_scene() -> bool:
	return dcl_scene_id == Global.scene_runner.get_current_parcel_scene_id()


#endregion

#region Post-processing


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


#endregion

#region Legacy / Unused
# Note: async_deferred_add_child is no longer used in the new loading flow
# but kept for compatibility with old content_provider path


func async_deferred_add_child():
	var new_gltf_node = dcl_pending_node
	dcl_pending_node = null

	# Guard: if pending node was already consumed or never set, skip
	# This can happen with duplicate signal emissions for cached GLTFs
	if new_gltf_node == null:
		return

	# Corner case, when the scene is unloaded before the gltf is loaded
	if not is_inside_tree():
		dcl_gltf_loading_state = GltfContainerLoadingState.FINISHED_WITH_ERROR
		timer.stop()
		# Free orphan node that was never added to tree
		new_gltf_node.queue_free()
		return

	var main_tree = get_tree()
	if not is_instance_valid(main_tree):
		dcl_gltf_loading_state = GltfContainerLoadingState.FINISHED_WITH_ERROR
		timer.stop()
		# Free orphan node that was never added to tree
		new_gltf_node.queue_free()
		return

	add_child(new_gltf_node)

	await main_tree.process_frame

	# Colliders and rendering is ensured to be ready at this point
	dcl_gltf_loading_state = GltfContainerLoadingState.FINISHED
	timer.stop()

	self.check_animations()


#endregion

#region Collider Management


func get_static_body_3d(mesh_instance: MeshInstance3D):
	for maybe_body in mesh_instance.get_children():
		if maybe_body is StaticBody3D:
			return maybe_body

	return null


# Set collision masks and metadata on all colliders after instantiating
# StaticBody3D is STATIC by default - will switch to KINEMATIC if entity moves
# Returns true if any colliders have active masks (need kinematic tracking)
func set_mask_colliders(
	node_to_inspect: Node, visible_cmask: int, invisible_cmask: int, scene_id: int, entity_id: int
) -> bool:
	var has_active_colliders := false
	for node in node_to_inspect.get_children():
		if node is MeshInstance3D:
			var body_3d = get_static_body_3d(node)
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
					if _kinematic_requested:
						# Entity already known to move - set KINEMATIC immediately
						var rid = body_3d.get_rid()
						if rid.is_valid():
							PhysicsServer3D.body_set_mode(rid, PhysicsServer3D.BODY_MODE_KINEMATIC)
					else:
						# Mark for deferred tracking - will switch to KINEMATIC if entity moves
						body_3d.set_meta("dcl_static_mode", true)
					has_active_colliders = true

		if set_mask_colliders(node, visible_cmask, invisible_cmask, scene_id, entity_id):
			has_active_colliders = true
	return has_active_colliders


func update_mask_colliders(node_to_inspect: Node):
	for node in node_to_inspect.get_children():
		if node is MeshInstance3D:
			var body_3d = get_static_body_3d(node)
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


# Signal handler: called by Rust when entity has moved enough to require kinematic mode
func _on_switch_to_kinematic():
	# Guard: Don't process if we're being freed or not in tree
	if not is_inside_tree():
		return
	if is_queued_for_deletion():
		return

	# Remember that this entity moves, so future colliders spawn as KINEMATIC
	_kinematic_requested = true

	var gltf_node = get_gltf_resource()
	if gltf_node != null and is_instance_valid(gltf_node):
		_switch_colliders_to_kinematic(gltf_node)


# Switch all colliders from STATIC to KINEMATIC mode
# Called when the entity moves (after the 2nd transform change)
# Searches ALL descendants for StaticBody3D, not just direct children of MeshInstance3D
func _switch_colliders_to_kinematic(node_to_inspect: Node):
	# Guard: Skip if node is being freed
	if not is_instance_valid(node_to_inspect):
		return

	for node in node_to_inspect.get_children():
		if not is_instance_valid(node):
			continue

		# Switch any StaticBody3D with dcl_static_mode to KINEMATIC
		if node is StaticBody3D:
			var body_3d: StaticBody3D = node
			if body_3d.has_meta("dcl_static_mode"):
				var rid = body_3d.get_rid()
				if rid.is_valid():
					PhysicsServer3D.body_set_mode(rid, PhysicsServer3D.BODY_MODE_KINEMATIC)
				body_3d.remove_meta("dcl_static_mode")

		_switch_colliders_to_kinematic(node)


#endregion

#region GLTF Changes


func change_gltf(
	new_gltf: String, visible_meshes_collision_mask: int, invisible_meshes_collision_mask: int
):
	var gltf_node := get_gltf_resource()
	var gltf_changed := dcl_gltf_src != new_gltf
	var masks_changed := (
		visible_meshes_collision_mask != dcl_visible_cmask
		or invisible_meshes_collision_mask != dcl_invisible_cmask
	)

	dcl_visible_cmask = visible_meshes_collision_mask
	dcl_invisible_cmask = invisible_meshes_collision_mask

	if gltf_changed:
		# New GLTF source - reload everything
		dcl_gltf_src = new_gltf
		optimized = false

		if gltf_node != null:
			remove_child(gltf_node)
			gltf_node.queue_free()

		if dcl_pending_node != null:
			dcl_pending_node.queue_free()
			dcl_pending_node = null

		async_load_gltf.call_deferred()

	elif masks_changed and gltf_node != null:
		# Same GLTF but masks changed - just update colliders
		update_mask_colliders(gltf_node)


func _on_timer_timeout():
	_finish_with_error("timeout")

#endregion
