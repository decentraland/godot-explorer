extends Control

var items: Array[Control] = []
var current_index = -1


func _ready():
	var i = 1
	while true:
		if !has_node("Item%d" % i):
			break

		var item: Control = get_node("Item%d" % i)

		item.hide()
		items.push_back(item)
		i += 1


func item_count():
	return items.size()


# gdlint:ignore = async-function-name
func set_item(index: int, direction_right: bool = true):
	if current_index == -1:
		current_index = index
		items[index].show()
	elif index != current_index:
		var new_item = items[index]
		var old_item = items[current_index]

		var direction = 1.0 if direction_right else -1.0
		var item_width = old_item.size.x * direction

		new_item.set_position(Vector2(item_width, 0.0))
		old_item.set_position(Vector2(0.0, 0.0))

		var old_tween = get_tree().create_tween()
		old_tween.tween_property(old_item, "position:x", -item_width, 0.25)

		var new_tween = get_tree().create_tween()
		new_tween.tween_property(new_item, "position:x", 0.0, 0.25)

		new_item.show()
		current_index = index
		await old_tween.finished
		old_item.hide()


func _on_timer_timeout():
	set_item(randi_range(0, items.size() - 1))
