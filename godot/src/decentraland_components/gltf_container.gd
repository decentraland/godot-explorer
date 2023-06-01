extends Node3D

@export var dcl_gltf_src: String = ""
@export var dcl_scene_id: int = -1

var file_hash: String = ""

func _ready():
	self.load_gltf.call_deferred()

func load_gltf():
	var scene_runner: SceneManager = get_tree().root.get_node("scene_runner")
	var content_mapping = scene_runner.get_scene_content_mapping(dcl_scene_id)
	var content_manager: ContentManager = get_tree().root.get_node("content_manager")
	
	self.dcl_gltf_src = dcl_gltf_src.to_lower()
	self.file_hash = content_mapping.get_content_hash(dcl_gltf_src)
	var fetching_resource = content_manager.fetch_resource(dcl_gltf_src, ContentManager.ContentType.CT_GLTF_GLB, content_mapping)
	
	if not fetching_resource:
		_on_gltf_loaded(self.file_hash)
	else:
		content_manager.content_loading_finished.connect(self._on_gltf_loaded)


func _on_gltf_loaded(resource_hash: String):
	if resource_hash != file_hash:
		return
	var node = get_tree().root.get_node("content_manager").get_resource_from_hash(file_hash)
	if node != null:
		add_child(node.duplicate())
