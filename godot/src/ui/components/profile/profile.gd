extends Control

var selected_node: Control

@onready var button_my_profile = $ColorRect_Background/MarginContainer/HBoxContainer/Button_MyProfile
@onready
var button_email_notifications = $ColorRect_Background/MarginContainer/HBoxContainer/Button_EmailNotifications
@onready
var v_box_container_my_profile = $VBoxContainer/ColorRect_MyProfile/MarginContainer/ScrollContainer/VBoxContainer/VBoxContainer_MyProfile
@onready
var v_box_container_email = $VBoxContainer/ColorRect_MyProfile/MarginContainer/ScrollContainer/VBoxContainer/VBoxContainer_Email



func _ready():
	self.modulate = Color(1, 1, 1, 0)
	button_my_profile.set_pressed(true)
	selected_node = v_box_container_my_profile
	fade_in(v_box_container_my_profile)


func _on_button_my_profile_pressed():
	if selected_node != v_box_container_my_profile:
		fade_out(selected_node)
		fade_in(v_box_container_my_profile)


func _on_button_email_notifications_pressed():
	if selected_node != v_box_container_email:
		fade_out(selected_node)
		fade_in(v_box_container_email)


func fade_in(node: Control):
	selected_node = node
	node.show()
	var tween_m = create_tween()
	tween_m.tween_property(node, "modulate", Color(1, 1, 1), 0.3)


func fade_out(node: Control):
	var tween_m = create_tween()
	tween_m.tween_property(node, "modulate", Color(1, 1, 1, 0), 0.3)
	var tween_h = create_tween()
	tween_h.tween_callback(node.hide).set_delay(0.3)
