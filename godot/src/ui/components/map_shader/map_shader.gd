extends Control

signal on_move
signal parcel_click(parcel_position: Vector2i)

@export var drag_enabled: bool = true
@export var zoom_value: int = 20

# Draging variables
var is_dragging = false
var dirty_is_dragging = false
var drag_position: Vector2
var start_dragging_position: Vector2

# The size of the map in parcels
var map_parcel_size: Vector2
# The top left parcel
var map_topleft_parcel_position: Vector2

@onready var color_rect_map = %ColorRect_Map


func _ready():
	map_parcel_size = Vector2(512, 512)
	map_topleft_parcel_position = Vector2(-256, -256)

	# TODO: use "https://api.decentraland.org/v1/minimap.png"
	color_rect_map.material = color_rect_map.material.duplicate()

	set_zoom(zoom_value)
	set_center_position(Vector2(0, 0))


func set_zoom(new_zoom_value: int) -> void:
	var center_parcel_position: Vector2 = get_center_position()
	zoom_value = new_zoom_value
	color_rect_map.size = map_parcel_size * new_zoom_value
	color_rect_map.material.set_shader_parameter("size", float(zoom_value))
	color_rect_map.material.set_shader_parameter(
		"line_width_px", floor(1.0 + float(zoom_value) / 16.0)
	)

	set_center_position(center_parcel_position)
	_update_used_parcels()


func set_center_position(parcel_position: Vector2):
	var absolute_position = (
		Vector2(parcel_position.x, -parcel_position.y) - map_topleft_parcel_position
	)
	var position_in_color_rect = Vector2(absolute_position) * zoom_value
	var new_position = -position_in_color_rect + self.size / 2.0 - (Vector2.ONE * zoom_value / 2.0)
	color_rect_map.position = new_position


func get_center_position() -> Vector2:
	var position_in_color_rect: Vector2 = -(
		color_rect_map.position - self.size / 2.0 + (Vector2.ONE * zoom_value / 2.0)
	)
	var absolute_position: Vector2 = Vector2(position_in_color_rect / zoom_value)
	var inverted_parcel_position: Vector2 = absolute_position + map_topleft_parcel_position
	return Vector2(inverted_parcel_position.x, -inverted_parcel_position.y)


func _on_color_rect_map_gui_input(event):
	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_LEFT:
			if event.pressed:
				dirty_is_dragging = true
				drag_position = get_global_mouse_position() - color_rect_map.position
				start_dragging_position = get_global_mouse_position()
				self.reflect_dragging.call_deferred()
			else:
				dirty_is_dragging = false
				self.reflect_dragging.call_deferred()
				var diff: Vector2 = (
					(get_global_mouse_position() - start_dragging_position) / zoom_value
				)
				if diff.length() < 1:
					parcel_click.emit(get_parcel_from_mouse())

	if event is InputEventMouseMotion:
		if drag_enabled and dirty_is_dragging:
			var new_pos = get_global_mouse_position() - drag_position
			color_rect_map.position = new_pos
			emit_signal("on_move")


func reflect_dragging():
	is_dragging = dirty_is_dragging


func get_parcel_from_mouse_real() -> Vector2:
	var absolute_position: Vector2 = Vector2(color_rect_map.get_local_mouse_position() / zoom_value)
	var position_without_offset: Vector2 = absolute_position + map_topleft_parcel_position
	return Vector2(position_without_offset.x, -position_without_offset.y)


func get_parcel_from_mouse() -> Vector2i:
	var real_position = get_parcel_from_mouse_real()
	return Vector2i(floor(real_position.x), floor(real_position.y))


func set_selected_parcel(parcel_position: Vector2):
	var color_rect_position = (
		Vector2(parcel_position.x, -parcel_position.y) - map_topleft_parcel_position
	)
	color_rect_map.material.set_shader_parameter("selected_tile", color_rect_position)


func set_used_parcels(used_parcel, emtpy_parcels):
	var total = used_parcel.size() + emtpy_parcels.size()
	var to_delete = color_rect_map.get_child_count() - total

	if to_delete > 0:
		for i in range(to_delete):
			# TODO: warn: check if get_child(0) is properly set
			var child = color_rect_map.get_child(0)
			color_rect_map.remove_child(child)
			child.queue_free()
	elif to_delete < 0:
		for i in range(-to_delete):
			var new_child: ColorRect = ColorRect.new()
			new_child.mouse_filter = Control.MOUSE_FILTER_PASS
			color_rect_map.add_child(new_child)

	var index: int = 0
	for i in range(used_parcel.size()):
		var color_rect: ColorRect = color_rect_map.get_child(index)
		color_rect.set_meta("parcel", Vector2(used_parcel[i].x, used_parcel[i].y))
		color_rect.color = Color.DARK_GREEN
		color_rect.color.a = 0.5
		index += 1

	for i in range(emtpy_parcels.size()):
		var color_rect: ColorRect = color_rect_map.get_child(index)
		color_rect.set_meta("parcel", Vector2(emtpy_parcels[i].x, emtpy_parcels[i].y))
		color_rect.color = Color.ORANGE_RED
		color_rect.color.a = 0.5
		index += 1

	_update_used_parcels()


func _update_used_parcels():
	for child in color_rect_map.get_children():
		var color_rect: ColorRect = child

		var parcel_position = Vector2(color_rect.get_meta("parcel"))
		var inverted_position = Vector2(parcel_position.x, -parcel_position.y)
		var parcel_in_control = inverted_position - map_topleft_parcel_position

		color_rect.size = Vector2(zoom_value, zoom_value)
		color_rect.position = zoom_value * (parcel_in_control + Vector2.UP)
