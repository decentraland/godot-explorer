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
	var scene_runner: SceneManager = get_tree().root.get_node("scene_runner")
	var content_mapping = scene_runner.get_scene_content_mapping(dcl_scene_id)
	var content_manager: ContentManager = get_tree().root.get_node("content_manager")

	self.dcl_gltf_src = dcl_gltf_src.to_lower()
	self.file_hash = content_mapping.get("content", {}).get(dcl_gltf_src, "")

	if self.file_hash.is_empty():
		gltf_state = GodotGltfState.NotFound
		return

	var fetching_resource = content_manager.fetch_gltf(dcl_gltf_src, content_mapping)

	# TODO: should we set a timeout?
	gltf_state = GodotGltfState.Loading

	if not fetching_resource:
		self._on_gltf_loaded.call_deferred(self.file_hash)
	else:
		content_manager.content_loading_finished.connect(self._on_gltf_loaded)


func _content_manager_resource_loaded(resource_hash: String):
	var content_manager: ContentManager = get_tree().root.get_node("content_manager")
	content_manager.content_loading_finished.disconnect(self._on_gltf_loaded)

	_on_gltf_loaded(resource_hash)


func _on_gltf_loaded(resource_hash: String):
	if resource_hash != file_hash:
		return

	var node = get_tree().root.get_node("content_manager").get_resource_from_hash(file_hash)
	if node == null:
		gltf_state = GodotGltfState.FinishedWithError
		return

	gltf_state = GodotGltfState.Finished
	gltf_node = node.duplicate()

	create_and_set_mask_colliders(gltf_node)
	add_child.call_deferred(gltf_node)


func get_collider(mesh_instance: MeshInstance3D):
	for maybe_static_body in mesh_instance.get_children():
		if maybe_static_body is StaticBody3D:
			return maybe_static_body

	return null


func create_and_set_mask_colliders(node_to_inspect: Node):
	for node in node_to_inspect.get_children():
		if node is MeshInstance3D:
			var mask: int = 0
			if node.visible:
				mask = dcl_visible_cmask
			else:
				mask = dcl_invisible_cmask

			var static_body_3d: StaticBody3D = get_collider(node)
			if static_body_3d == null and mask > 0:
				node.create_trimesh_collision()
				static_body_3d = get_collider(node)

			if static_body_3d != null:
				var parent = static_body_3d.get_parent()
				var new_animatable = AnimatableBody3D.new()
				parent.add_child(new_animatable)
				parent.remove_child(static_body_3d)

				for child in static_body_3d.get_children(true):
					static_body_3d.remove_child(child)
					new_animatable.add_child(child)
					if child is CollisionShape3D and child.shape is ConcavePolygonShape3D:
						# TODO: workaround, the face's normals probably need to be inverted in some meshes
						child.shape.backface_collision = true

				new_animatable.collision_layer = mask
				new_animatable.sync_to_physics = false
				new_animatable.set_meta("dcl_scene_id", dcl_scene_id)
				new_animatable.set_meta("dcl_entity_id", dcl_entity_id)

		if node is Node:
			create_and_set_mask_colliders(node)


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
			visible_meshes_collision_mask != dcl_visible_cmask
			or invisible_meshes_collision_mask != dcl_invisible_cmask
		):
			dcl_visible_cmask = visible_meshes_collision_mask
			dcl_invisible_cmask = invisible_meshes_collision_mask
			create_and_set_mask_colliders(gltf_node)
