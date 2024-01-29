extends ColorRect

@onready var margin_container_content = $PanelContainer/MarginContainer_Content
@onready
var button_back = $PanelContainer/MarginContainer_Navigation/HBoxContainer/VBoxContainer_Left/Button_Back
@onready
var button_close = $PanelContainer/MarginContainer_Navigation/HBoxContainer/VBoxContainer_Right/Button_Close


func _ready():
	if margin_container_content.get_child_count() == 0:
		button_back.hide()


func _on_gui_input(event):
	if event is InputEventScreenTouch:
		if event.pressed:
			close()


func close():
	for child in margin_container_content.get_children():
		child.queue_free()

	var tween_m = create_tween()
	tween_m.tween_property(self, "modulate", Color(1, 1, 1, 0), 0.3)
	var tween_h = create_tween()
	tween_h.tween_callback(self.hide).set_delay(0.3)


func open(childNode):
	push(childNode)
	show()
	var tween_m = create_tween()
	tween_m.tween_property(self, "modulate", Color(1, 1, 1), 0.3)


func push(scene: PackedScene):
	if !visible:
		show()
		var tween_m = create_tween()
		tween_m.tween_property(self, "modulate", Color(1, 1, 1), 0.3)

	for child in margin_container_content.get_children():
		fade_out(child)
	var instantiatedScene = scene.instantiate()
	instantiatedScene.set_parent(self)
	margin_container_content.add_child(instantiatedScene)

	if margin_container_content.get_child_count() <= 1:
		button_back.hide()
	else:
		button_back.show()


func pop():
	var last_index: int = margin_container_content.get_child_count() - 1
	var last_child: Node = margin_container_content.get_child(last_index)
	fade_out(last_child)
	margin_container_content.remove_child(last_child)

	var new_last_child: Node = margin_container_content.get_child(last_index - 1)
	fade_in(new_last_child)

	if margin_container_content.get_child_count() <= 1:
		button_back.hide()
	else:
		button_back.show()


func fade_out(node: Node):
	var tween_m = create_tween()
	tween_m.tween_property(node, "modulate", Color(1, 1, 1, 0), 0.3)
	var tween_h = create_tween()
	tween_h.tween_callback(node.hide).set_delay(0.3)


func fade_in(node: Node):
	node.modulate = Color(1, 1, 1, 0)
	node.show()
	var tween_m = create_tween()
	tween_m.tween_property(node, "modulate", Color(1, 1, 1, 1), 0.3)


func _on_button_back_pressed():
	pop()


func _on_button_close_pressed():
	close()
