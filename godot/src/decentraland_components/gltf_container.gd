extends Node3D

@export var dcl_gltf_src: String = ""
@export var dcl_scene_id: int = -1
@export var dcl_visible_cmask: int = 0
@export var dcl_invisible_cmask: int = 3
@export var dcl_entity_id: int = -1

var file_hash: String = ""
var gltf_node = null

const GodotGltfState = {
	Unknown = 0,
	Loading = 1,
	NotFound = 2,
	FinishedWithError = 3,
	Finished = 4,
}
var gltf_state: int = 0


func _ready():
	self.load_gltf.call_deferred()


func load_gltf():
	var content_mapping = Global.scene_runner.get_scene_content_mapping(dcl_scene_id)

	self.dcl_gltf_src = dcl_gltf_src.to_lower()
	self.file_hash = content_mapping.get("content", {}).get(dcl_gltf_src, "")

	if self.file_hash.is_empty():
		gltf_state = GodotGltfState.NotFound
		return

#	# TODO: should we set a timeout?
	gltf_state = GodotGltfState.Loading

	var promise = Global.content_manager.fetch_gltf(dcl_gltf_src, content_mapping)
	if promise != null:
		await promise.awaiter()

	_on_gltf_loaded()


func _on_gltf_loaded():
	var node = Global.content_manager.get_resource_from_hash(file_hash)
	if node == null:
		gltf_state = GodotGltfState.FinishedWithError
		return

	var promise: Promise = Global.content_manager.instance_gltf_colliders(
		node, dcl_visible_cmask, dcl_invisible_cmask, dcl_scene_id, dcl_entity_id
	)
	
	await promise.awaiter()

	gltf_node = promise.get_data()
	gltf_state = GodotGltfState.Finished

	add_child.call_deferred(gltf_node)


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

		self.load_gltf.call_deferred()
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
