class_name MapPin
extends TextureRect

enum PinCategoryEnum {
	ALL,
	FAVORITES,
	ART,
	CRYPTO,
	SOCIAL,
	GAMES,
	SHOP,
	EDUCATION,
	MUSIC,
	FASHION,
	CASINO,
	SPORTS,
	BUSINESS,
	POI,
	LIVE
}

signal touched_pin(pos: Vector2i)
signal play_sound

@export var pin_category: PinCategoryEnum
@export var coord_x: int
@export var coord_y: int
@export var scene_title: String

var pin_x: int
var pin_y: int

@onready var label_scene_title: Label = %Label_SceneTitle
@onready var container_cluster: PanelContainer = %Container_Cluster
@onready var label_cluster: Label = %Label_Cluster


func _ready():
	UiSounds.install_audio_recusirve(self)
	if scene_title.length() > 0:
		label_scene_title.text = scene_title
		label_scene_title.show()
	else:
		label_scene_title.hide()

	set_category(pin_category)


func set_category(category: PinCategoryEnum) -> void:
	var category_string = PinCategoryEnum.keys()[category].to_lower()
	if category_string == null:
		push_error("Category not found: %s" % category_string)
		return

	var image_path := "res://assets/ui/place_categories/%s-pin.svg" % category_string
	var loaded_texture := load(image_path)

	if loaded_texture:
		self.texture = loaded_texture
	else:
		printerr("_update_pin_category_icon texture_path not found ", image_path)


func set_place(place: Place) -> void:
	label_scene_title.text = place.title


func show_cluster(quantity: int = 1):
	if quantity > 1:
		label_cluster.text = str(quantity)
		container_cluster.show()


func _process(_delta: float) -> void:
	var camera_zoom = get_sibling_camera_zoom()
	scale = Vector2.ONE / camera_zoom


func get_sibling_camera_zoom() -> Vector2:
	var parent = get_parent()
	if not parent:
		return Vector2.ONE
	for sibrling in parent.get_children():
		if sibrling is Camera2D:
			return sibrling.zoom
	return Vector2.ONE


func _on_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if event.pressed:
			self.play_sound.emit()
			touched_pin.emit(Vector2i(pin_x, pin_y))
