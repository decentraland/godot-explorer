extends DclGltfContainer

enum GltfContainerLoadingState {
	UNKNOWN = 0,
	LOADING = 1,
	NOT_FOUND = 2,
	FINISHED_WITH_ERROR = 3,
	FINISHED = 4,
}

var file_hash: String = ""
var gltf_node = null


func _ready():
	self.async_load_gltf.call_deferred()


func async_load_gltf():
	var content_mapping = Global.scene_runner.get_scene_content_mapping(dcl_scene_id)

	self.dcl_gltf_src = dcl_gltf_src.to_lower()
	self.file_hash = content_mapping.get("content", {}).get(dcl_gltf_src, "")

	if self.file_hash.is_empty():
		dcl_gltf_loading_state = GltfContainerLoadingState.NOT_FOUND
		return

	# TODO: should we set a timeout?
	dcl_gltf_loading_state = GltfContainerLoadingState.LOADING

	var promise = Global.content_manager.fetch_gltf(dcl_gltf_src, content_mapping)
	if promise != null:
		await PromiseUtils.async_awaiter(promise)

	_async_on_gltf_loaded()


func _async_on_gltf_loaded():
	var node = Global.content_manager.get_resource_from_hash(file_hash)
	if node == null:
		dcl_gltf_loading_state = GltfContainerLoadingState.FINISHED_WITH_ERROR
		return

	var promise: Promise = Global.content_manager.instance_gltf_colliders(
		node, dcl_visible_cmask, dcl_invisible_cmask, dcl_scene_id, dcl_entity_id
	)

	await PromiseUtils.async_awaiter(promise)

	gltf_node = promise.get_data()
	self.async_deferred_add_child.call_deferred(gltf_node)


func async_deferred_add_child(new_gltf_node):
	# Corner case, when the scene is unloaded before the gltf is loaded
	var main_tree = get_tree()
	if not is_instance_valid(main_tree):
		dcl_gltf_loading_state = GltfContainerLoadingState.FINISHED_WITH_ERROR
		return

	add_child(new_gltf_node)
	self.check_animations()

	await main_tree.process_frame

	# Colliders and rendering is ensured to be ready at this point
	dcl_gltf_loading_state = GltfContainerLoadingState.FINISHED


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
		self.dcl_gltf_src = new_gltf
		dcl_visible_cmask = visible_meshes_collision_mask
		dcl_invisible_cmask = invisible_meshes_collision_mask

		if gltf_node != null:
			remove_child(gltf_node)
			gltf_node.queue_free()
			gltf_node = null

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
