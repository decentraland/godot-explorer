class_name JumpInWrapper
extends Control

signal jump_in(position: Vector2i, realm: String)

@onready var panel_jump_in_portrait: JumpIn = %PanelJumpInPortrait
@onready var panel_jump_in_landscape: JumpIn = %PanelJumpInLandscape
@onready var texture_progress_bar: TextureProgressBar = %TextureProgressBar


func _ready():
	panel_jump_in_portrait.jump_in.connect(self._emit_jump_in)
	panel_jump_in_landscape.jump_in.connect(self._emit_jump_in)
	panel_jump_in_portrait.close.connect(self._close)
	panel_jump_in_landscape.close.connect(self._close)
	texture_progress_bar.hide()


func _emit_jump_in(position: Vector2i, realm: String):
	jump_in.emit(position, realm)


func _close():
	self.hide()
	UiSounds.play_sound("mainmenu_widget_close")


func async_load_place_position(position: Vector2i):
	panel_jump_in_portrait.hide()
	panel_jump_in_landscape.hide()
	show()
	texture_progress_bar.show()
	var url: String = "https://places.decentraland.org/api/places?limit=1"
	url += "&positions=%d,%d" % [position.x, position.y]

	var headers = {"Content-Type": "application/json"}
	var promise: Promise = Global.http_requester.request_json(
		url, HTTPClient.METHOD_GET, "", headers
	)
	var result = await PromiseUtils.async_awaiter(promise)

	if result is PromiseError:
		printerr("Error request places", result.get_error())
		return

	var json: Dictionary = result.get_string_response_as_json()

	if json.data.is_empty():
		var unknown_place: Dictionary = {
			"base_position": "%d,%d" % [position.x, position.y], "title": "Unknown place"
		}
		set_data(unknown_place)
	else:
		set_data(json.data[0])
	texture_progress_bar.hide()
	show_animation()


func set_data(item_data):
	panel_jump_in_landscape.set_data(item_data)
	panel_jump_in_portrait.set_data(item_data)


func show_animation() -> void:
	self.show()
	if panel_jump_in_portrait != null and panel_jump_in_landscape != null:
		if Global.is_orientation_portrait():
			panel_jump_in_portrait.show()
			panel_jump_in_landscape.hide()
			var animation_target_y = panel_jump_in_portrait.position.y
			# Place the menu off-screen above (its height above the target position)
			panel_jump_in_portrait.position.y = (
				panel_jump_in_portrait.position.y + panel_jump_in_portrait.size.y
			)

			(
				create_tween()
				. tween_property(panel_jump_in_portrait, "position:y", animation_target_y, 0.5)
				. set_trans(Tween.TRANS_SINE)
				. set_ease(Tween.EASE_OUT)
			)
		else:
			panel_jump_in_portrait.hide()
			panel_jump_in_landscape.show()
			var animation_target_x = panel_jump_in_landscape.position.x
			# Place the menu off-screen above (its height above the target position)
			panel_jump_in_landscape.position.x = (
				panel_jump_in_landscape.position.x + panel_jump_in_landscape.size.x
			)

			(
				create_tween()
				. tween_property(panel_jump_in_landscape, "position:x", animation_target_x, 0.5)
				. set_trans(Tween.TRANS_SINE)
				. set_ease(Tween.EASE_OUT)
			)


func _on_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if !event.pressed:
			_close()
