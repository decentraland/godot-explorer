extends PanelContainer

@onready var counter = %Counter

func _ready():
	hide()


func _on_child_order_changed():
	set_visible(get_child_count() > 1)

	if is_instance_valid(counter):
		if get_child_count() > 2:
			counter.set_text("Tabs: %d" % (get_child_count() - 1))
			counter.show()
		else:
			counter.hide()
