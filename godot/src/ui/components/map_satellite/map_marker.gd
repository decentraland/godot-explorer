extends Control
class_name Marker

@onready var label: Label = $Label
@onready var sprite_2d: Sprite2D = %Sprite2D

var marker_x: int = 0
var marker_y: int = 0

const std_scale = Vector2(0.207, 0.207)

func _ready() -> void:
	update()

func _process(_delta: float) -> void:
	# var camera_zoom = get_sibling_camera_zoom()
	# sprite_2d.scale = std_scale
	pass
	

func update()->void:
	print(marker_x,",", marker_y)
	if label:
		label.visible = true
		label.text = "{0},{1}".format([marker_x, marker_y])

func get_sibling_camera_zoom() -> Vector2:
	var parent = get_parent()
	if not parent:
		return Vector2.ONE
	for sibrling in parent.get_children():
		if sibrling is Camera2D:
			return sibrling.zoom
	return Vector2.ONE
