extends DclGltfContainer

enum GltfContainerLoadingState {
	UNKNOWN = 0,
	LOADING = 1,
	NOT_FOUND = 2,
	FINISHED_WITH_ERROR = 3,
	FINISHED = 4,
}

var dcl_gltf_hash := ""
var optimized := false
# Once entity is known to move, all colliders should spawn as KINEMATIC
var _kinematic_requested := false
# Content hash this container is registered under in GltfLoadingCoordinator
var _requested_hash := ""


func _ready():
	# Check if Rust flagged this entity as kinematic (has active tween)
	if self.has_meta("kinematic_requested"):
		_kinematic_requested = true
	# Connect to switch_to_kinematic signal from Rust
	self.switch_to_kinematic.connect(_on_switch_to_kinematic)
	self.async_load_gltf.call_deferred()


func _exit_tree():
	# Detach from the shared load group
	if not _requested_hash.is_empty():
		GltfLoadingCoordinator.unregister(self, _requested_hash)

	# Free pending orphan node (legacy path)
	if dcl_pending_node != null:
		dcl_pending_node.queue_free()
		dcl_pending_node = null


#region Loading Flow
# Each container resolves its content hash and decides optimized-vs-runtime
# once, then hands off to GltfLoadingCoordinator. The coordinator shares one
# download + one main-thread ResourceLoader.load per hash across all waiters,
# instantiates every waiter when that shared load finishes (so instances never
# re-enter the queue), and paces add_child to one source-group per frame.
#
# Two loading paths (chosen per hash, shared by the coordinator):
# 1. Optimized: Pre-baked scenes from res://glbs/ (loaded via ResourceLoader)
# 2. Runtime: Runtime-processed scenes from user://content/<hash>.scn


func async_load_gltf():
	self.dcl_gltf_src = dcl_gltf_src.to_lower()
	var content_mapping := Global.scene_runner.get_scene_content_mapping(dcl_scene_id)
	var file_hash := content_mapping.get_hash(dcl_gltf_src)
	self.dcl_gltf_hash = file_hash

	if file_hash.is_empty():
		dcl_gltf_loading_state = GltfContainerLoadingState.NOT_FOUND
		return

	dcl_gltf_loading_state = GltfContainerLoadingState.LOADING
	Global.get_gltf_load_timeout_coalescer().schedule(self, 120_000)

	# Decide optimized-vs-runtime once; the coordinator shares the actual work.
	var has_optimized = Global.content_provider.optimized_asset_exists(file_hash)
	var use_optimized := false
	if Global.cli.only_no_optimized:
		# --only-no-optimized: always runtime-process, ignore optimized assets
		use_optimized = false
	elif Global.cli.only_optimized:
		# --only-optimized: skip entirely if no optimized asset exists
		if not has_optimized:
			dcl_gltf_loading_state = GltfContainerLoadingState.NOT_FOUND
			Global.get_gltf_load_timeout_coalescer().cancel(self)
			return
		use_optimized = true
	else:
		# Default: prefer the pre-baked optimized asset when present
		use_optimized = has_optimized

	self.optimized = use_optimized
	_requested_hash = file_hash
	GltfLoadingCoordinator.request(
		self, file_hash, dcl_gltf_src, dcl_scene_id, use_optimized, content_mapping
	)


#endregion

#region Coordinator callbacks
# Driven by GltfLoadingCoordinator once the shared load for this hash resolves.


## True while this container still needs its instance built (load in flight,
## nothing added yet) — guards a group from being drained twice for late waiters.
func _needs_realize() -> bool:
	return dcl_gltf_loading_state == GltfContainerLoadingState.LOADING and get_child_count() == 0


## Instantiate a fresh copy from the shared PackedScene and add it to the tree.
## Called for every waiter of one source within a SINGLE frame — no await here,
## the coordinator awaits one frame for the whole batch, then completes it.
func _instantiate_and_add(packed_scene: PackedScene) -> void:
	# Scene may have been unloaded between the shared load and this realize pass
	if not is_inside_tree():
		_finish_with_error("scene unloaded during load")
		return

	var gltf_node: Node3D = packed_scene.instantiate()
	# instantiate() clones the node tree but references the already-loaded,
	# already-GPU-resident meshes/textures — CPU-only and cheap.
	apply_fixes(gltf_node)

	# Set collision masks (colliders created with mask=0 initially)
	set_mask_colliders(
		gltf_node, dcl_visible_cmask, dcl_invisible_cmask, dcl_scene_id, dcl_entity_id
	)

	# Add to tree so global_transform of every MeshInstance3D inside the GLB is
	# valid (the manager bakes per-mesh local poses against the container's
	# current world). Without this step every mesh would land at the world origin.
	add_child(gltf_node)


## Mark FINISHED after the batch's render frame. gpu_ms is the whole source
## batch's first-frame stall, shared across its instances.
func _complete_shared_load(gpu_ms: float = -1.0) -> void:
	if dcl_gltf_loading_state != GltfContainerLoadingState.LOADING:
		return
	_complete_load(gpu_ms)


## Called by the coordinator when the shared load fails for this hash.
func _on_shared_load_error(reason: String) -> void:
	_finish_with_error(reason)


#endregion

#region Completion


func _complete_load(_gpu_ms: float = -1.0):
	dcl_gltf_loading_state = GltfContainerLoadingState.FINISHED
	Global.get_gltf_load_timeout_coalescer().cancel(self)

	self.check_animations()


func _finish_with_error(reason: String = "unknown"):
	printerr("GLTF load error for ", dcl_gltf_src, ": ", reason)
	# Report to resource tracker if we have a valid hash
	if not dcl_gltf_hash.is_empty():
		Global.content_provider.report_resource_failed(dcl_gltf_hash, reason)
	dcl_gltf_loading_state = GltfContainerLoadingState.FINISHED_WITH_ERROR
	Global.get_gltf_load_timeout_coalescer().cancel(self)


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
	# Camera proximity fade (issue #1814): dissolve geometry the camera clips into
	ProximityFade.apply_material(mat)

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
		Global.get_gltf_load_timeout_coalescer().cancel(self)
		# Free orphan node that was never added to tree
		new_gltf_node.queue_free()
		return

	var main_tree = get_tree()
	if not is_instance_valid(main_tree):
		dcl_gltf_loading_state = GltfContainerLoadingState.FINISHED_WITH_ERROR
		Global.get_gltf_load_timeout_coalescer().cancel(self)
		# Free orphan node that was never added to tree
		new_gltf_node.queue_free()
		return

	add_child(new_gltf_node)

	await main_tree.process_frame

	# Colliders and rendering is ensured to be ready at this point
	dcl_gltf_loading_state = GltfContainerLoadingState.FINISHED
	Global.get_gltf_load_timeout_coalescer().cancel(self)

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

		# Detach from the old shared load group before requesting the new one
		if not _requested_hash.is_empty():
			GltfLoadingCoordinator.unregister(self, _requested_hash)
			_requested_hash = ""

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


## Invoked from GltfLoadTimeoutCoalescer when the load-timeout deadline
## elapses (replacement for the per-container Timer node's `timeout` signal).
func _on_load_timeout():
	_finish_with_error("timeout")

#endregion
