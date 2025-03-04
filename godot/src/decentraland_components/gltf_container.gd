extends DclGltfContainer

enum GltfContainerLoadingState {
	UNKNOWN = 0,
	LOADING = 1,
	NOT_FOUND = 2,
	FINISHED_WITH_ERROR = 3,
	FINISHED = 4,
}

@onready var timer = $Timer


func _ready():
	self.async_load_gltf.call_deferred()


func async_try_load_gltf_from_local_file(gltf_hash: String) -> void:
	var promise = Global.content_provider.fetch_optimized_asset_with_dependencies(gltf_hash)
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		printerr(
			(
				"Failed to download optimized asset (sceneId=%s gltf_hash=%s)"
				% [dcl_scene_id, gltf_hash]
			)
		)
		return

	var main_tree = get_tree()
	if not is_instance_valid(main_tree):
		return

	var scene_file = "res://glbs/" + gltf_hash + ".tscn"
	if not FileAccess.file_exists(scene_file + ".remap"):
		printerr("File %s doesn't exists" % scene_file)
		dcl_gltf_loading_state = GltfContainerLoadingState.FINISHED_WITH_ERROR
		timer.stop()
		return
	var err = ResourceLoader.load_threaded_request(scene_file)
	if err != OK:
		dcl_gltf_loading_state = GltfContainerLoadingState.FINISHED_WITH_ERROR
		timer.stop()
		return

	var status = ResourceLoader.load_threaded_get_status(scene_file)
	while status == 1:
		await main_tree.process_frame
		status = ResourceLoader.load_threaded_get_status(scene_file)

	var resource = ResourceLoader.load_threaded_get(scene_file)
	if resource == null:
		dcl_gltf_loading_state = GltfContainerLoadingState.FINISHED_WITH_ERROR
		timer.stop()
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
		return

	dcl_pending_node = res_instance
	timer.stop()


func async_load_gltf():
	var content_mapping := Global.scene_runner.get_scene_content_mapping(dcl_scene_id)

	self.dcl_gltf_src = dcl_gltf_src.to_lower()
	var file_hash = content_mapping.get_hash(dcl_gltf_src)
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

	remove_emission_texture(res_instance)

	dcl_pending_node = res_instance


func remove_emission_texture(gltf_instance: Node3D):
	# HACK: Workaround to fix an import error that sets the emisison texture as the albedo texture.
	for child in gltf_instance.get_children():
		if !(child is MeshInstance3D):
			continue
		var material = child.mesh.surface_get_material(0)
		material.emission_texture = null


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
