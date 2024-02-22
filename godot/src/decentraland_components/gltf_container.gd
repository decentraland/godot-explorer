extends DclGltfContainer

enum GltfContainerLoadingState {
	UNKNOWN = 0,
	LOADING = 1,
	NOT_FOUND = 2,
	FINISHED_WITH_ERROR = 3,
	FINISHED = 4,
}


func _ready():
	self.async_load_gltf.call_deferred()


func async_load_gltf():
	var content_mapping := Global.scene_runner.get_scene_content_mapping(dcl_scene_id)

	self.dcl_gltf_src = dcl_gltf_src.to_lower()
	if content_mapping.get_hash(dcl_gltf_src).is_empty():
		dcl_gltf_loading_state = GltfContainerLoadingState.NOT_FOUND
		return

	# TODO: should we set a timeout?
	dcl_gltf_loading_state = GltfContainerLoadingState.LOADING

	var promise = Global.content_provider.fetch_gltf(dcl_gltf_src, content_mapping, 0)
	if promise == null:
		printerr("Fatal error on fetch gltf: promise == null")
		dcl_gltf_loading_state = GltfContainerLoadingState.FINISHED_WITH_ERROR
		return

	if not promise.is_resolved():
		await PromiseUtils.async_awaiter(promise)

	var res = promise.get_data()
	if res is PromiseError:
		printerr("Error on fetch gltf: ", res.get_error())
		dcl_gltf_loading_state = GltfContainerLoadingState.FINISHED_WITH_ERROR
		return

	var instance_promise: Promise = Global.content_provider.instance_gltf_colliders(
		res, dcl_visible_cmask, dcl_invisible_cmask, dcl_scene_id, dcl_entity_id
	)
	var res_instance = await PromiseUtils.async_awaiter(instance_promise)
	if res_instance is PromiseError:
		printerr("Error on fetch gltf: ", res_instance.get_error())
		dcl_gltf_loading_state = GltfContainerLoadingState.FINISHED_WITH_ERROR
		return

	dcl_pending_node = res_instance


func async_deferred_add_child():
	var new_gltf_node = dcl_pending_node
	dcl_pending_node = null

	# Corner case, when the scene is unloaded before the gltf is loaded
	if not is_inside_tree():
		dcl_gltf_loading_state = GltfContainerLoadingState.FINISHED_WITH_ERROR
		return

	var main_tree = get_tree()
	if not is_instance_valid(main_tree):
		dcl_gltf_loading_state = GltfContainerLoadingState.FINISHED_WITH_ERROR
		return

	add_child(new_gltf_node)

	await main_tree.process_frame

	# Colliders and rendering is ensured to be ready at this point
	dcl_gltf_loading_state = GltfContainerLoadingState.FINISHED

	self.check_animations()
	
	var anim_player: AnimationPlayer = get_child(0).get_node_or_null("AnimationPlayer")
	if anim_player != null:
		prints("gltf has ",anim_player.get_animation_list().size(), " anims")
		


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
	if self.dcl_gltf_src != new_gltf:
		dcl_gltf_loading_state = GltfContainerLoadingState.LOADING

		self.dcl_gltf_src = new_gltf
		dcl_visible_cmask = visible_meshes_collision_mask
		dcl_invisible_cmask = invisible_meshes_collision_mask

		if get_child_count() > 0:
			var gltf_node = get_child(0)
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
			and get_child_count() > 0
		):
			dcl_visible_cmask = visible_meshes_collision_mask
			dcl_invisible_cmask = invisible_meshes_collision_mask
			update_mask_colliders(get_child(0))
