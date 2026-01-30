class_name SidePanelWrapper
extends Control

signal jump_in(position: Vector2i, realm: String)
signal close

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
	portrait_panel.set_anchors_preset(Control.PRESET_FULL_RECT)
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

	var result = await PlacesHelper.async_get_by_position(pos)

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
	open_panel()


func set_data(data):
	item_data = data


func open_panel() -> void:
	_close()
	self.show()
	if Global.is_orientation_portrait():
		instantiate_portrait_panel()
		orientation = "portrait"
	else:
		instantiate_landscape_panel()
		orientation = "landscape"
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
