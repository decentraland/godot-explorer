extends DclCameraModeArea3D

var _my_scene_id: int = -1

@onready var collision_shape_3d = $CollisionShape3D
@onready var on_active_scene = $OnActiveScene


func _ready():
	var shape = BoxShape3D.new()
	shape.set_size(area)
	collision_shape_3d.set_shape(shape)
	collision_shape_3d.set_disabled(true)

	var scene_node: DclSceneNode = SceneHelper.search_scene_node(self)
	if scene_node.is_global():
		# It's always active
		collision_shape_3d.set_disabled(false)
		return

	_my_scene_id = scene_node.get_scene_id()

	_deferred_start.call_deferred()
	Global.scene_runner.on_change_scene_id.connect(self._on_change_scene_id)


func _on_change_scene_id(scene_id: int):
	_on_scene_active(_my_scene_id == scene_id)


func _deferred_start():
	# First emiting to setup the initial value
	_on_scene_active(_my_scene_id == Global.scene_runner.get_current_parcel_scene_id())


func _on_scene_active(active: bool):
	collision_shape_3d.set_disabled(!active)
