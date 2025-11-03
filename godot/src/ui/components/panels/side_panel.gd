class_name SidePanelWrapper
extends Control

signal jump_in(position: Vector2i, realm: String)
signal close

const JUMP_IN_PORTRAIT = preload(
	"res://src/ui/components/discover/jump_in/panel_jump_in_portrait.tscn"
)
const JUMP_IN_LANDSCAPE = preload(
	"res://src/ui/components/discover/jump_in/panel_jump_in_landscape.tscn"
)

@export var portrait_panel_resource: PackedScene
@export var landscape_panel_resource: PackedScene
@export var tracking_handler: ScreenTrackingHandler

var portrait_panel: PlaceItem
var landscape_panel: PlaceItem
var item_data: Dictionary
var orientation: String

@onready var texture_progress_bar: TextureProgressBar = %TextureProgressBar


func _ready():
	texture_progress_bar.hide()


func _emit_jump_in(pos: Vector2i, realm: String):
	jump_in.emit(pos, realm)


func _close():
	self.hide()
	for child in get_children():
		if child is PlaceItem:
			child.queue_free()
	UiSounds.play_sound("mainmenu_widget_close")


func instantiate_portrait_panel():
	portrait_panel = portrait_panel_resource.instantiate()
	self.add_child(portrait_panel)
	portrait_panel.set_anchors_preset(Control.PRESET_BOTTOM_WIDE)
	portrait_panel.set_data(item_data)
	portrait_panel.jump_in.connect(self._emit_jump_in)
	set_data(item_data)


func instantiate_landscape_panel():
	landscape_panel = landscape_panel_resource.instantiate()
	self.add_child(landscape_panel)
	landscape_panel.set_anchors_preset(Control.PRESET_RIGHT_WIDE)
	landscape_panel.set_data(item_data)
	landscape_panel.jump_in.connect(self._emit_jump_in)
	set_data(item_data)


func async_load_place_position(pos: Vector2i):
	_close()
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


func set_data(data):
	item_data = data


func show_animation() -> void:
	_close()
	self.show()
	if Global.is_orientation_portrait():
		instantiate_portrait_panel()
		orientation = "portrait"
		var animation_target_y = portrait_panel.position.y
		# Place the menu off-screen above (its height above the target position)
		portrait_panel.position.y = (portrait_panel.position.y + portrait_panel.size.y)

		(
			create_tween()
			. tween_property(portrait_panel, "position:y", animation_target_y, 0.5)
			. set_trans(Tween.TRANS_SINE)
			. set_ease(Tween.EASE_OUT)
		)
	else:
		instantiate_landscape_panel()
		orientation = "landscape"
		var animation_target_x = landscape_panel.position.x
		# Place the menu off-screen above (its height above the target position)
		landscape_panel.position.x = (landscape_panel.position.x + landscape_panel.size.x)

		(
			create_tween()
			. tween_property(landscape_panel, "position:x", animation_target_x, 0.5)
			. set_trans(Tween.TRANS_SINE)
			. set_ease(Tween.EASE_OUT)
		)

	if tracking_handler:
		if item_data.is_empty():
			printerr("SidePanel: WARNING - item_data is empty!")
		tracking_handler.track_screen_viewed(item_data)
	else:
		printerr("SidePanel: tracking_handler is null")


func _on_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if !event.pressed:
			_close()
			close.emit()
