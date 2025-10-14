class_name EventDetailWrapper
extends Control

signal jump_in(position: Vector2i, realm: String)
signal close

@onready var texture_progress_bar: TextureProgressBar = %TextureProgressBar
@onready var event_details_portrait: PlaceItem = %EventDetailsPortrait
@onready var event_details_landscape: PlaceItem = %EventDetailsLandscape


func _ready():
	event_details_portrait.jump_in.connect(self._emit_jump_in)
	event_details_portrait.close.connect(self._close)
	texture_progress_bar.hide()


func _emit_jump_in(pos: Vector2i, realm: String):
	jump_in.emit(pos, realm)


func _close():
	self.hide()
	UiSounds.play_sound("mainmenu_widget_close")


func async_load_place_position(pos: Vector2i):
	event_details_portrait.hide()
	event_details_landscape.hide()
	show()
	texture_progress_bar.show()
	var url: String = "https://places.decentraland.org/api/places?limit=1"
	url += "&positions=%d,%d" % [pos.x, pos.y]

	var headers = {"Content-Type": "application/json"}
	var promise: Promise = Global.http_requester.request_json(
		url, HTTPClient.METHOD_GET, "", headers
	)
	var result = await PromiseUtils.async_awaiter(promise)

	if result is PromiseError:
		printerr("Error request places jump in", result.get_error())
		return

	var json: Dictionary = result.get_string_response_as_json()

	if json.data.is_empty():
		var unknown_place: Dictionary = {
			"base_position": "%d,%d" % [pos.x, pos.y], "title": "Unknown place"
		}
		set_data(unknown_place)
	else:
		set_data(json.data[0])
	texture_progress_bar.hide()
	show_animation()


func set_data(item_data):
	event_details_landscape.set_data(item_data)
	event_details_portrait.set_data(item_data)


func show_animation() -> void:
	self.show()
	if event_details_portrait != null and event_details_landscape != null:
		if Global.is_orientation_portrait():
			event_details_portrait.show()
			event_details_landscape.hide()
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
			event_details_portrait.hide()
			event_details_landscape.show()
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


func _on_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if !event.pressed:
			_close()
			close.emit()
