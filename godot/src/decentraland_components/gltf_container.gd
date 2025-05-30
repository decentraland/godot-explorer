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

@onready var timer = $Timer

# Static variable to track currently loading assets
static var currently_loading_assets := []
# Static queue and flag for throttling
static var pending_load_queue := []


func _ready():
	self.async_load_gltf.call_deferred()


# Helper to handle finishing a load (success or error)
func _finish_gltf_load(gltf_hash: String):
	currently_loading_assets.erase(gltf_hash)
	_process_next_gltf_load()


func async_try_load_gltf_from_local_file(gltf_hash: String) -> void:
	self.optimized = true

	# Throttling: If already loading max, queue and return
	if currently_loading_assets.size() >= MAX_CONCURRENT_LOADS:
		if not pending_load_queue.has(self):
			pending_load_queue.append(self)
		return

	# Add asset to loading list and print
	if not currently_loading_assets.has(gltf_hash):
		currently_loading_assets.append(gltf_hash)

	var promise = Global.content_provider.fetch_optimized_asset_with_dependencies(gltf_hash)
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		printerr(
			(
				"Failed to download optimized asset (sceneId=%s gltf_hash=%s)"
				% [dcl_scene_id, gltf_hash]
			)
		)
		_finish_gltf_load(gltf_hash)
		return

	var main_tree = get_tree()
	if not is_instance_valid(main_tree):
		dcl_gltf_loading_state = GltfContainerLoadingState.FINISHED_WITH_ERROR
		timer.stop()
		_finish_gltf_load(gltf_hash)
		return

	var scene_file = "res://glbs/" + gltf_hash + ".tscn"
	if not FileAccess.file_exists(scene_file + ".remap"):
		printerr("File %s doesn't exists" % scene_file)
		dcl_gltf_loading_state = GltfContainerLoadingState.FINISHED_WITH_ERROR
		timer.stop()
		_finish_gltf_load(gltf_hash)
		return
	var err = ResourceLoader.load_threaded_request(scene_file)
	if err != OK:
		dcl_gltf_loading_state = GltfContainerLoadingState.FINISHED_WITH_ERROR
		timer.stop()
		_finish_gltf_load(gltf_hash)
		return

	var status = ResourceLoader.load_threaded_get_status(scene_file)
	while status == 1:
		await main_tree.process_frame
		status = ResourceLoader.load_threaded_get_status(scene_file)

	var resource = ResourceLoader.load_threaded_get(scene_file)
	if resource == null:
		dcl_gltf_loading_state = GltfContainerLoadingState.FINISHED_WITH_ERROR
		timer.stop()
		_finish_gltf_load(gltf_hash)
		return

	var gltf_node = resource.instantiate()
	var instance_promise: Promise = Global.content_provider.instance_gltf_colliders(
		gltf_node, dcl_visible_cmask, dcl_invisible_cmask, dcl_scene_id, dcl_entity_id
	)
	var res_instance = await PromiseUtils.async_awaiter(instance_promise)
	if res_instance is PromiseError:
		printerr("Error on fetch gltf: ", res_instance.get_error())
		dcl_gltf_loading_state = GltfContainerLoadingState.FINISHED_WITH_ERROR
		timer.stop()
		_finish_gltf_load(gltf_hash)
		return

	if res_instance == null:
		printerr("instance_gltf_colliders returned null for hash: ", gltf_hash)
		dcl_gltf_loading_state = GltfContainerLoadingState.FINISHED_WITH_ERROR
		timer.stop()
		_finish_gltf_load(gltf_hash)
		return

	apply_fixes(res_instance)
	dcl_pending_node = res_instance
	timer.stop()
	_finish_gltf_load(gltf_hash)


static func _process_next_gltf_load():
	while currently_loading_assets.size() < MAX_CONCURRENT_LOADS and pending_load_queue.size() > 0:
		# Prioritize GLTFs from the current scene
		var idx = -1
		for i in range(pending_load_queue.size()):
			var candidate = pending_load_queue[i]
			if candidate != null and candidate.is_current_scene():
				idx = i
				break
		var next_gltf = null
		if idx != -1:
			next_gltf = pending_load_queue[idx]
			pending_load_queue.remove_at(idx)
		else:
			next_gltf = pending_load_queue.pop_front()
		if next_gltf != null:
			next_gltf.async_try_load_gltf_from_local_file(next_gltf.dcl_gltf_hash)
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

	# TODO: should we set a timeout?
	dcl_gltf_loading_state = GltfContainerLoadingState.LOADING
	timer.start()

	if Global.content_provider.optimized_asset_exists(file_hash):
		await async_try_load_gltf_from_local_file(file_hash)
		return

	var promise = Global.content_provider.fetch_scene_gltf(dcl_gltf_src, content_mapping)
	if promise == null:
		printerr("Fatal error on fetch gltf: promise == null")
		dcl_gltf_loading_state = GltfContainerLoadingState.FINISHED_WITH_ERROR
		timer.stop()
		return

	if not promise.is_resolved():
		await PromiseUtils.async_awaiter(promise)

	var res = promise.get_data()
	if res is PromiseError:
		printerr("Error on fetch gltf: ", res.get_error())
		dcl_gltf_loading_state = GltfContainerLoadingState.FINISHED_WITH_ERROR
		timer.stop()
		return

	var resource_locker = res.get_node("ResourceLocker")
	if is_instance_valid(resource_locker):
		self.add_child(resource_locker.duplicate())

	var instance_promise: Promise = Global.content_provider.instance_gltf_colliders(
		res, dcl_visible_cmask, dcl_invisible_cmask, dcl_scene_id, dcl_entity_id
	)
	var res_instance = await PromiseUtils.async_awaiter(instance_promise)
	if res_instance is PromiseError:
		printerr("Error on fetch gltf: ", res_instance.get_error())
		dcl_gltf_loading_state = GltfContainerLoadingState.FINISHED_WITH_ERROR
		timer.stop()
		return

	apply_fixes(res_instance)
	dcl_pending_node = res_instance


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
		for idx in mesh.get_surface_count():
			var material = mesh.surface_get_material(idx)
			fix_material(material)


func fix_material(mat: BaseMaterial3D):
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


func get_animatable_body_3d(mesh_instance: MeshInstance3D):
	for maybe_static_body in mesh_instance.get_children():
		if maybe_static_body is AnimatableBody3D:
			return maybe_static_body

	return null


func update_mask_colliders(node_to_inspect: Node):
	for node in node_to_inspect.get_children():
		if node is MeshInstance3D:
			var mask: int = 0
			if node.visible:
				mask = dcl_visible_cmask
			else:
				mask = dcl_invisible_cmask

			var animatable_body_3d = get_animatable_body_3d(node)
			if animatable_body_3d != null:
				animatable_body_3d.collision_layer = mask
				animatable_body_3d.collision_mask = 0
				animatable_body_3d.set_meta("dcl_col", mask)
				if mask == 0:
					animatable_body_3d.process_mode = Node.PROCESS_MODE_DISABLED
				else:
					animatable_body_3d.process_mode = Node.PROCESS_MODE_INHERIT

		if node is Node:
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
