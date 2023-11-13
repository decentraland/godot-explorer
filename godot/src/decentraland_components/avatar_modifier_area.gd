extends DclAvatarModifierArea3D

@onready var collision_shape_3d = $CollisionShape3D

var scene_id: int = 0


# Called when the node enters the scene tree for the first time.
func _ready():
	scene_id = SceneHelper.search_scene_node(self).get_scene_id()
	var shape = BoxShape3D.new()
	shape.set_size(area)
	collision_shape_3d.set_shape(shape)
