extends Area3D

signal block_camera_mode(mode: Global.CameraMode)
signal unblock_camera_mode


func _on_area_entered(area):
	if area is DclCameraModeArea3D:
		check_areas()


func _on_area_exited(area):
	if area is DclCameraModeArea3D:
		check_areas()


func get_last_dcl_camera_mode_area_3d() -> DclCameraModeArea3D:
	return get_overlapping_areas().back()


func check_areas():
	if has_overlapping_areas():
		var first_area = get_last_dcl_camera_mode_area_3d()  # get first
		if first_area != null:
			block_camera_mode.emit(first_area.forced_camera_mode)
	else:
		unblock_camera_mode.emit()
