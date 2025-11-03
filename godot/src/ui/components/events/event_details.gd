class_name EventDetailWrapper
extends Control

signal jump_in(position: Vector2i, realm: String)
signal close

const EVENT_DETAILS_PORTRAIT = preload("res://src/ui/components/events/event_details_portrait.tscn")
const EVENT_DETAILS_LANDSCAPE = preload("res://src/ui/components/events/event_details_landscape.tscn")

var event_id: String
var event_status: String
var event_tags: String
var orientation: String
var event_details_portrait: PlaceItem
var event_details_landscape: PlaceItem


func _ready():
	pass


func instantiate_portrait_panel(item_data):
	event_details_portrait = EVENT_DETAILS_PORTRAIT.instantiate()
	self.add_child(event_details_portrait)
	event_details_portrait.set_anchors_preset(Control.PRESET_BOTTOM_WIDE)
	event_details_portrait.set_data(item_data)
	event_details_portrait.jump_in.connect(self._emit_jump_in)
	set_data(item_data)


func instantiate_landscape_panel(item_data):
	event_details_landscape = EVENT_DETAILS_LANDSCAPE.instantiate()
	self.add_child(event_details_landscape)
	event_details_landscape.set_anchors_preset(Control.PRESET_RIGHT_WIDE)
	event_details_landscape.set_data(item_data)
	event_details_landscape.jump_in.connect(self._emit_jump_in)
	set_data(item_data)


func _emit_jump_in(pos: Vector2i, realm: String):
	jump_in.emit(pos, realm)


func _close():
	self.hide()
	for child in get_children():
		child.queue_free()


func set_data(item_data):
	event_id = item_data.get("id", "unknown-id")
	event_status = "live" if item_data.get("live", false) else "upcoming"
	event_tags = "trending" if item_data.get("trending", false) else "none"


func show_animation(item_data) -> void:
	_close()
	self.show()
	if Global.is_orientation_portrait():
		instantiate_portrait_panel(item_data)
		orientation = "portrait"
		var animation_target_y = event_details_portrait.position.y
		# Place the menu off-screen above (its height above the target position)
		event_details_portrait.position.y = (
			event_details_portrait.position.y + event_details_portrait.size.y
		)

		(
			create_tween()
			. tween_property(event_details_portrait, "position:y", animation_target_y, 0.5)
			. set_trans(Tween.TRANS_SINE)
			. set_ease(Tween.EASE_OUT)
		)
	else:
		instantiate_landscape_panel(item_data)
		orientation = "landscape"
		var animation_target_x = event_details_landscape.position.x
		# Place the menu off-screen above (its height above the target position)
		event_details_landscape.position.x = (
			event_details_landscape.position.x + event_details_landscape.size.x
		)

		(
			create_tween()
			. tween_property(event_details_landscape, "position:x", animation_target_x, 0.5)
			. set_trans(Tween.TRANS_SINE)
			. set_ease(Tween.EASE_OUT)
		)
	Global.metrics.track_screen_viewed(
		"EVENT_DETAILS",
		JSON.stringify(
			{
				"event_id": event_id,
				"event_status": event_status,
				"event_tags": event_tags,
				"orientation": orientation
			}
		)
	)


func _on_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if !event.pressed:
			_close()
			close.emit()
