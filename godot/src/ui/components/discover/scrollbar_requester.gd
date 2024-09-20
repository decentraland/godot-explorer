extends CarrouselRequester

var current_offset = 0
var threshold_limit = 10

func start():
	var node = self
	if not node is ScrollContainer:
		printerr("Not a scroll container!")
		return

	self.scroll_ended.connect(self._on_scroll_container_scroll_ended)
	emit_request()

func restart():
	current_offset = 0
	emit_request()

func emit_request():
	request.emit(current_offset, threshold_limit)

func _on_scroll_container_scroll_ended():
	var child_number = 0
	var min_child_number = 0
	var max_child_number = 0
	var scroll_container_width = self.size.x
	var scroll_horizontal = self.scroll_horizontal
	for child in item_container.get_children():
		child_number += 1
		var begin = child.position.x - scroll_horizontal
		var end = child.position.x + child.size.x - scroll_horizontal
		if end >= 0 and begin <= scroll_container_width:
			max_child_number = max(max_child_number, child_number)
			min_child_number = (
				min(min_child_number, child_number) if min_child_number > 0 else child_number
			)

	if max_child_number >= threshold_limit:
		current_offset = (max_child_number / threshold_limit) * threshold_limit
		emit_request()
