class_name EventDetailWrapper
extends Control

signal jump_in(position: Vector2i, realm: String)
signal close

var event_id: String
var event_status: String
var event_tags: String
var orientation: String

@onready var texture_progress_bar: TextureProgressBar = %TextureProgressBar
@onready var event_details_portrait: PlaceItem = %EventDetailsPortrait
@onready var event_details_landscape: PlaceItem = %EventDetailsLandscape


func _ready():
	event_details_portrait.jump_in.connect(self._emit_jump_in)
	event_details_portrait.close.connect(self._close)
	event_details_landscape.jump_in.connect(self._emit_jump_in)
	event_details_landscape.close.connect(self._close)
	texture_progress_bar.hide()


func _emit_jump_in(pos: Vector2i, realm: String):
	jump_in.emit(pos, realm)


func _close():
	self.hide()
	UiSounds.play_sound("mainmenu_widget_close")


func set_data(item_data):
	event_details_landscape.set_data(item_data)
	event_details_portrait.set_data(item_data)
	event_id = item_data.get("id", "unknown-id")
	event_status = "live" if item_data.get("live", false) else "upcoming"
	event_tags = "trending" if item_data.get("trending", false) else "none"


func show_animation() -> void:
	self.show()
	if event_details_portrait != null and event_details_landscape != null:
		if Global.is_orientation_portrait():
			orientation = "portrait"
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
			orientation = "landscape"
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
