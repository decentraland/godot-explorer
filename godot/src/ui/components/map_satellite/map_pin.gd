class_name MapPin

extends Control
signal touched_pin(pos:Vector2i)
var pin_x:int
var pin_y:int

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
@export var pin_category: PinCategoryEnum
@export var coord_x: int
@export var coord_y: int

@export var scene_title: String
@onready var label: Label = $Sprite2D/Label
@onready var sprite_2d: Sprite2D = %Sprite2D
@onready var cluster: PanelContainer = $Sprite2D/PanelContainer
@onready var cluster_label: Label = $Sprite2D/PanelContainer/Label

const cluster_radius:int = 60

func _ready():
	if scene_title.length() > 0:
		label.text = scene_title
		label.show()
	else:
		label.hide()
	
	set_category(pin_category)
	
func set_category(category: PinCategoryEnum) -> void:
	var category_string = PinCategoryEnum.keys()[category].to_lower()
	if category_string == null:
		push_error("Category not found: %s" % category_string)
		return
	
	var image_path := "res://assets/ui/place_categories/%s-pin.svg" % category_string
	var texture := load(image_path)
	
	if texture:
		sprite_2d.texture = texture
	else:
		printerr("_update_pin_category_icon texture_path not found ", image_path)

func set_place(place:Place) -> void:
	label.text = place.title
	
func show_cluster(quantity: int = 1):
	if quantity > 1:
		cluster_label.text = str(quantity)
		cluster.show()
	
func _process(_delta: float) -> void:
	var camera_zoom = get_sibling_camera_zoom()
	sprite_2d.scale = Vector2.ONE / camera_zoom

func get_sibling_camera_zoom() -> Vector2:
	var parent = get_parent()
	if not parent:
		return Vector2.ONE
	for sibrling in parent.get_children():
		if sibrling is Camera2D:
			return sibrling.zoom
	return Vector2.ONE


func _on_control_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if event.pressed:
			emit_signal("touched_pin", Vector2i(pin_x, pin_y))
