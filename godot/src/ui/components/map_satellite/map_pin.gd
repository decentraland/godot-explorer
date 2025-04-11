class_name MapPin

extends Control

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
	POI
}
@export var pin_category: PinCategoryEnum
@export var coord_x: int
@export var coord_y: int
@export var scene_title: String

@onready var label: Label = $Sprite2D/Label
@onready var sprite_2d: Sprite2D = %Sprite2D
@onready var animation_player: AnimationPlayer = $AnimationPlayer

const SCALE_FACTOR := Vector2(1.1, 1.1)
var hovered := false
var tween: Tween

func _ready():
	label.text = scene_title
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


func _on_control_mouse_entered() -> void:
	animation_player.play('show_title')
	if hovered: return
	hovered = true
	smooth_scale(Vector2.ONE * SCALE_FACTOR)


func _on_control_mouse_exited() -> void:
	animation_player.play('hide_title')
	if not hovered: return
	smooth_scale(Vector2.ONE)
	hovered = false

func _process(_delta: float) -> void:
	const FONT_SIZE = 16
	const FONT_OUTLINE_SIZE = 6
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

func smooth_scale(target: Vector2, duration: float = 0.3):
	if scale == target:
		return
			
	tween = get_tree().create_tween()
	tween.tween_property(self, "scale", target, duration).set_trans(Tween.TRANS_SINE).set_ease(Tween.EASE_OUT)
