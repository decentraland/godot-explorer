extends Area3D

signal block_camera_mode(mode: Global.CameraMode)
signal unblock_camera_mode

var overlapping_areas: Array[Area3D] = []


func _on_area_entered(area):
	if area is DclCameraModeArea3D:
		overlapping_areas.push_back(area)
		check_areas()


func _on_area_exited(area):
	if area is DclCameraModeArea3D:
		overlapping_areas.erase(area)
		check_areas()


func get_last_dcl_camera_mode_area_3d() -> DclCameraModeArea3D:
	return overlapping_areas.back()


func check_areas():
	if !overlapping_areas.is_empty():
		var first_area = get_last_dcl_camera_mode_area_3d()  # get first
		if first_area != null:
			block_camera_mode.emit(first_area.forced_camera_mode)
	else:
		unblock_camera_mode.emit()
