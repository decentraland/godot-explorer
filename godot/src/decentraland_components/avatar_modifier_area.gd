extends DclAvatarModifierArea3D

# Initial value SceneId::INVALID
var scene_id: int = -1

@onready var collision_shape_3d = $CollisionShape3D


# Called when the node enters the scene tree for the first time.
func _ready():
	scene_id = SceneHelper.search_scene_node(self).get_scene_id()
	var shape = BoxShape3D.new()
	shape.set_size(area)
	collision_shape_3d.set_shape(shape)
