extends Node3D

@export var dcl_gltf_src: String = ""
@export var dcl_scene_id: int = -1

var scene_runner: SceneRunner
var realm: Realm
var base_url: String
var hash: String

# Called when the node enters the scene tree for the first time.
func _ready():
	realm = get_tree().root.get_node("realm")
	scene_runner = get_tree().root.get_node("scene_runner")
	self.load_gltf.call_deferred()
	
func load_gltf():
	
	base_url = scene_runner.get_scene_base_url(dcl_scene_id)
	hash = scene_runner.get_scene_content_hash(dcl_scene_id, dcl_gltf_src)
	
	if hash.is_empty() or base_url.is_empty():
		return
	
	var file = await realm.requester.do_request_file(base_url + hash, hash, 0)
	var local_gltf_path = "user://content/" + hash
	if not FileAccess.file_exists(local_gltf_path):
		return
	
	var gltf := GLTFDocument.new()
	var gltf_state := GLTFState.new()

	var custom_importer = custom_gltf_importer.new()
	
	gltf.register_gltf_document_extension(custom_importer)
	gltf.append_from_file(local_gltf_path, gltf_state)
	var node = gltf.generate_scene(gltf_state)
	
	add_child(node)

	print("gltf is almost loaded!! ", dcl_gltf_src)

class custom_gltf_importer extends GLTFDocumentExtension:
	func _convert_scene_node(state: GLTFState, gltf_node: GLTFNode, scene_node: Node) -> void:
		pass

	func _import_node(state: GLTFState, gltf_node: GLTFNode, json: Dictionary, node: Node)  -> Error:
		return OK # super._import_node(state,gltf_node, json, node)
		
	func _import_post(state: GLTFState, root: Node) -> Error:
		return OK # super._import_post(state,root)
		
	func _import_post_parse(state: GLTFState) -> Error:
		return OK # super._import_post_parse(state)
		
	func _import_preflight(state: GLTFState, extensions: PackedStringArray) -> Error:
		return OK # super._import_preflight(state,extensions)
		

