extends Control

@onready var counter = %Counter

@onready var stack = %Stack


func _ready():
	hide()


func _on_dialog_stack_child_order_changed():
	var stack_childs = stack.get_child_count()
	self.set_visible(stack_childs > 0)

	if is_instance_valid(counter):
		if stack_childs > 1:
			counter.set_text("Tabs: %d" % stack_childs)
			counter.show()
		else:
			counter.hide()


func _on_gui_input(event):
	if event is InputEventScreenTouch:
		if !event.pressed:
			for child in stack.get_children():
				child.queue_free()
