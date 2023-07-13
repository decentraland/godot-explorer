extends Control

@onready
var h_slider_process_tick_quota = $VBoxContainer_General/VBoxContainer_ProcessTickQuota/HBoxContainer/HSlider_ProcessTickQuota
@onready
var label_process_tick_quota_value = $VBoxContainer_General/VBoxContainer_ProcessTickQuota/HBoxContainer/Label_ProcessTickQuotaValue
@onready
var h_slider_scene_radius = $VBoxContainer_General/VBoxContainer_SceneRadius/HBoxContainer/HSlider_SceneRadius
@onready
var label_scene_radius_value = $VBoxContainer_General/VBoxContainer_SceneRadius/HBoxContainer/Label_SceneRadiusValue
@onready
var line_edit_gravity = $VBoxContainer_General/HBoxContainer/HBoxContainer_Gravity/LineEdit_Gravity
@onready
var line_edit_jump_velocity = $VBoxContainer_General/HBoxContainer/HBoxContainer_JumpVelocity/LineEdit_JumpVelocity
@onready
var line_edit_run_speed = $VBoxContainer_General/HBoxContainer2/HBoxContainer_RunSpeed/LineEdit_RunSpeed
@onready
var line_edit_walk_speed = $VBoxContainer_General/HBoxContainer2/HBoxContainer_WalkSpeed/LineEdit_WalkSpeed

var gravity: float
var walk_velocity: float
var run_velocity: float
var jump_velocity: float
var scene_radius: int
var process_tick_quota: int


func _ready():
	get_config_dictionary()
	refresh_values()


func get_config_dictionary():
	gravity = Global.get_gravity()
	walk_velocity = Global.get_walk_velocity()
	run_velocity = Global.get_run_velocity()
	jump_velocity = Global.get_jump_velocity()
	scene_radius = Global.get_scene_radius()
	process_tick_quota = Global.get_process_tick_quota()


func refresh_values():
	line_edit_gravity.text = str(gravity).pad_decimals(1)
	line_edit_walk_speed.text = str(walk_velocity).pad_decimals(1)
	line_edit_run_speed.text = str(run_velocity).pad_decimals(1)
	line_edit_jump_velocity.text = str(jump_velocity).pad_decimals(1)
	h_slider_process_tick_quota.set_value_no_signal(process_tick_quota)
	h_slider_scene_radius.set_value_no_signal(scene_radius)
	label_process_tick_quota_value.text = str(process_tick_quota)
	label_scene_radius_value.text = str(scene_radius)


func apply_changes():
	pass


func _on_h_slider_process_tick_quota_value_changed(value):
	label_process_tick_quota_value.text = str(value)


func _on_h_slider_scene_radius_value_changed(value):
	label_scene_radius_value.text = str(value)
