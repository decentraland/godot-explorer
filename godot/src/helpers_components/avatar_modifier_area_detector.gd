extends Area3D

signal set_avatar_modifier_area(area: DclAvatarModifierArea3D)
# Logic explanation:
# If the avatar enters/exits an AREA but it is not in the scene
# we don't check if it's inside or not...
# and when the avatar enters/exits the scene, it will check it

signal unset_avatar_modifier_area

@export var avatar: Avatar = null

var overlapping_areas: Array[Area3D] = []


func _ready():
	avatar.change_scene_id.connect(self._on_avatar_change_scene_id)

	# Arbitrary order
	for area in self.get_overlapping_areas():
		_on_area_entered(area)


func _on_tree_exiting():
	avatar.change_scene_id.disconnect(self._on_avatar_change_scene_id)


func _on_avatar_change_scene_id(_new_scene_id: int, _prev_scene_id: int):
	check_areas()


func _on_area_entered(area):
	if area is DclAvatarModifierArea3D:
		overlapping_areas.push_back(area)
		check_areas()


func _on_area_exited(area):
	if area is DclAvatarModifierArea3D:
		overlapping_areas.erase(area)
		check_areas()


func get_last_dcl_avatar_modifier_area_3d(areas: Array[Area3D]) -> DclAvatarModifierArea3D:
	return areas.back()


func check_areas():
	var avatar_scene_id = avatar.current_parcel_scene_id

	# only areas that have the same scene id than the player...
	var areas = overlapping_areas.filter(func(area): return area.scene_id == avatar_scene_id)

	if !areas.is_empty():
		var first_area = get_last_dcl_avatar_modifier_area_3d(areas)
		if first_area != null:
			set_avatar_modifier_area.emit(first_area)
	else:
		unset_avatar_modifier_area.emit()
