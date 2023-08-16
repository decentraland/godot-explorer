extends Node

signal config_changed

@onready var is_mobile =OS.get_name() == "Android" or OS.get_name() == "iOS"
@onready var is_desktop = OS.get_name() != "Android" and OS.get_name() != "iOS"
@onready var is_vr = false

#@onready var is_mobile = true

## Global classes (singleton pattern)

var scene_runner: SceneManager
var realm: Realm
var content_manager: ContentManager
var comms: CommunicationManager
var avatars: AvatarScene
var config: ConfigData

var raycast_debugger = load("res://src/tool/raycast_debugger/raycast_debugger.gd").new()

var standalone = false

var xr_interface: XRInterface = null

var current_camera: Camera3D 
var xr_main_controller: XRController3D 

func _ready():
	var args := OS.get_cmdline_args()

	if args.size() == 1 and args[0].begins_with("res://"):
		if args[0] != "res://src/main.tscn":
			self.standalone = true

	if args.has("--test"):
		var test_runner = load("res://src/test/test_runner.gd").new()
		add_child(test_runner)
		test_runner.start.call_deferred()
		return

	if not DirAccess.dir_exists_absolute("user://content/"):
		DirAccess.make_dir_absolute("user://content/")
		
		
	if is_mobile:
		xr_interface = XRServer.find_interface("OpenXR")
		if xr_interface and xr_interface.is_initialized():
			print("OpenXR initialised successfully")

			# Turn off v-sync!
			DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)

			# Change our main viewport to output to the HMD
			get_viewport().use_xr = true
			self.is_vr = true
			self.is_mobile = false
		else:
			print("OpenXR not initialized, please check if your headset is connected")

	self.scene_runner = SceneManager.new()
	self.scene_runner.set_name("scene_runner")
	self.scene_runner.process_mode = Node.PROCESS_MODE_DISABLED

	self.realm = Realm.new()
	self.realm.set_name("realm")

	self.content_manager = ContentManager.new()
	self.content_manager.set_name("content_manager")

	self.comms = CommunicationManager.new()
	self.comms.set_name("comms")

	self.avatars = AvatarScene.new()
	self.avatars.set_name("avatars")

	self.config = ConfigData.new()
	config.load_from_settings_file()

	get_tree().root.add_child.call_deferred(self.scene_runner)
	get_tree().root.add_child.call_deferred(self.realm)
	get_tree().root.add_child.call_deferred(self.comms)
	get_tree().root.add_child.call_deferred(self.content_manager)
	get_tree().root.add_child.call_deferred(self.avatars)

	# TODO: enable raycast debugger
	add_child(raycast_debugger)

	DCLMeshRenderer._init_primitive_shapes()
	


func add_raycast(_id: int, _time: float, _from: Vector3, _to: Vector3) -> void:
	# raycast_debugger.add_raycast(id, time, from, to)
	pass


func get_tls_client():
	return TLSOptions.client_unsafe()


func print_node_tree(node: Node, prefix = ""):
	print(prefix + node.name)
	for child in node.get_children():
		if child is Node:
			print_node_tree(child, prefix + node.name + "/")

func get_raycast_params() -> Array[Vector3]:
	if is_vr:
		var raycast_from = xr_main_controller.global_position
		var direction = xr_main_controller.global_transform.rotated(Vector3.RIGHT, PI/2).rotated(Vector3.UP, PI * 49.5 / 180.0)
		var raycast_to = raycast_from + direction.basis.z * -100.0
#		print("raycast params ", raycast_from, " and ", raycast_to)
		return [raycast_from, raycast_to]
	elif is_desktop or is_mobile :
		var mouse_position = get_viewport().get_visible_rect().size  * 0.5
		var raycast_from = current_camera.project_ray_origin(mouse_position)
		var raycast_to = raycast_from + current_camera.project_ray_normal(mouse_position) * 100.0
		return [raycast_from, raycast_to]
	
	return []
