class_name Marker
extends Control

const LABEL_POSITION = Vector2(-50, 28)

var marker_x: int = 0
var marker_y: int = 0

@onready var sprite_2d: Sprite2D = %Sprite2D
@onready var coordinates_label: Label = %CoordinatesLabel
@onready var control: Control = %Control


func _ready() -> void:
	update()


func _process(_delta: float) -> void:
	var camera_zoom = get_sibling_camera_zoom()
	control.scale = Vector2.ONE / camera_zoom.x
	coordinates_label.position.y = LABEL_POSITION.y * camera_zoom.y


func update() -> void:
	if coordinates_label:
		coordinates_label.visible = true
		coordinates_label.text = "{0},{1}".format([marker_x, marker_y])


func get_sibling_camera_zoom() -> Vector2:
	var parent = get_parent()
	if not parent:
		return Vector2.ONE
	for sibling in parent.get_children():
		if sibling is Camera2D:
			return sibling.zoom
	return Vector2.ONE
