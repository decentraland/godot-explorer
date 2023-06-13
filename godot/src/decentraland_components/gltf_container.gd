extends Node3D

@export var dcl_gltf_src: String = ""
@export var dcl_scene_id: int = -1

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
	self.file_hash = content_mapping.get_content_hash(dcl_gltf_src)

	if self.file_hash.is_empty():
		gltf_state = GodotGltfState.NotFound
		return

	var fetching_resource = content_manager.fetch_resource(dcl_gltf_src, ContentManager.ContentType.CT_GLTF_GLB, content_mapping)

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
	if node != null:
		gltf_state = GodotGltfState.Finished
		gltf_node = node.duplicate()
		add_child(gltf_node)
	else:
		gltf_state = GodotGltfState.FinishedWithError
		
func change_gltf(new_gltf: String):
	if self.dcl_gltf_src == new_gltf:
		# TODO: maybe some property changed
		return

	if gltf_node != null:
		remove_child(gltf_node)
		gltf_node.queue_free()
		gltf_node = null
	
	self.dcl_gltf_src = new_gltf
	self.load_gltf.call_deferred()
