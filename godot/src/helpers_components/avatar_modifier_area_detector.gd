extends Area3D

signal set_avatar_modifier_area(area: DclAvatarModifierArea3D)
signal unset_avatar_modifier_area()

var overlapping_areas: Array[Area3D] = []


func _on_area_entered(area):
	if area is DclAvatarModifierArea3D:
		overlapping_areas.push_back(area)
		check_areas()


func _on_area_exited(area):
	if area is DclAvatarModifierArea3D:
		overlapping_areas.erase(area)
		check_areas()


func get_last_dcl_avatar_modifier_area_3d() -> DclAvatarModifierArea3D:
	return overlapping_areas.back()


func check_areas():
	if !overlapping_areas.is_empty():
		var first_area = get_last_dcl_avatar_modifier_area_3d()
		if first_area != null:
			set_avatar_modifier_area.emit(first_area)
	else:
		unset_avatar_modifier_area.emit()
