extends Control

signal camera_params_updated(type, fov, ortho_size, param_position, target)

var fov: float = 60.0
var ortho_size: float = 10.0

@onready var option_button_projection = $Panel/OptionButton_Projection
@onready var line_edit_fov_size = $Panel/LineEdit_FOV_Size
@onready var line_edit_target_x = $Panel/HBoxContainer_Target/LineEdit_X
@onready var line_edit_target_y = $Panel/HBoxContainer_Target/LineEdit_Y
@onready var line_edit_target_z = $Panel/HBoxContainer_Target/LineEdit_Z
@onready var line_edit_position_x = $Panel/HBoxContainer_Position/LineEdit_X
@onready var line_edit_position_y = $Panel/HBoxContainer_Position/LineEdit_Y
@onready var line_edit_position_z = $Panel/HBoxContainer_Position/LineEdit_Z


func _on_option_button_projection_item_selected(index):
	if index == 0:  # perspective
		line_edit_fov_size.text = str(fov)
	else:
		line_edit_fov_size.text = str(ortho_size)

	emit_updated()


func _on_line_edit_text_changed(_new_text):
	emit_updated()


func _on_line_edit_fov_size_text_changed(_new_text):
	if option_button_projection.selected == 0:
		fov = float(line_edit_fov_size.text)
	else:
		ortho_size = float(line_edit_fov_size.text)

	emit_updated()


func emit_updated():
	var camera_option: Camera3D.ProjectionType
	if option_button_projection.selected == 0:
		camera_option = Camera3D.PROJECTION_PERSPECTIVE
	else:
		camera_option = Camera3D.PROJECTION_ORTHOGONAL

	var param_position = Vector3(
		float(line_edit_position_x.text),
		float(line_edit_position_y.text),
		float(line_edit_position_z.text)
	)
	var param_target = Vector3(
		float(line_edit_target_x.text),
		float(line_edit_target_y.text),
		float(line_edit_target_z.text)
	)
	camera_params_updated.emit(camera_option, fov, ortho_size, param_position, param_target)
