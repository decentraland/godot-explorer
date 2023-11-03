extends PanelContainer


func _ready():
	hide()


func _on_child_order_changed():
	set_visible(get_child_count() > 1)

	if %Counter:
		if get_child_count() > 2:
			%Counter.set_text("Tabs: %d" % (get_child_count() - 1))
			%Counter.show()
		else:
			%Counter.hide()
