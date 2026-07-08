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


# Called from Rust (avatar_modifier_area.rs) after exclude_ids / modifiers /
# area are updated on an existing node. Forces overlapping AvatarModifierArea
# detectors to re-evaluate so freshly-added excludeIds take effect on avatars
# that already entered the area. See issue #2166.
func refresh_overlapping_detectors() -> void:
	# Issue #2166: do NOT rely on Area3D.get_overlapping_areas() — it returns
	# stale data when invoked from a CRDT update tick (physics has not yet
	# settled). Instead, ask every known avatar to re-evaluate; each one's
	# detector decides whether this AMA applies to it.
	var avatars_root = Global.avatars
	if avatars_root != null:
		for a in avatars_root.get_avatars():
			if a.has_method("try_show"):
				a.try_show()
	var player_avatar = Global.scene_runner.player_avatar_node
	if player_avatar != null and player_avatar.has_method("try_show"):
		player_avatar.try_show()
