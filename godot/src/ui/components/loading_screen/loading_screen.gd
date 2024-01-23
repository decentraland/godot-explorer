extends VBoxContainer

var bg_colors: Array[Color] = [
	Color(0.5, 0.25, 0.0, 1.0),
	Color(0.0, 0.0, 0.5, 1.0),
	Color(0.5, 0.5, 0.0, 1.0),
	Color(0.5, 0.0, 0.25, 1.0),
	Color(0.25, 0.0, 0.5, 1.0),
	Color(0.34, 0.22, 0.15, 1.0),
	Color(0.0, 0.5, 0.5, 1.0),
]

var item_index = 0
var item_count = 0
var progress: float = 0.0

@onready var loading_progress = %ColorRect_LoadingProgress
@onready var loading_progress_label = %Label_LoadingProgress

@onready
var carousel = $ColorRect_Background/Control_Discover/VBoxContainer/HBoxContainer_Content/CarouselViewport/SubViewport/Carousel
@onready var background: ColorRect = $ColorRect_Background

@onready var timer_auto_move_carousel = $Timer_AutoMoveCarousel
@onready var timer_hide_loading_screen = $Timer_HideLoadingScreen


func _ready():
	item_count = carousel.item_count()
	set_item(randi_range(0, item_count - 1))


# Forward
func enable_loading_screen():
	$LoadingScreenProgressLogic.enable_loading_screen()


func _on_texture_rect_right_arrow_gui_input(event):
	if event is InputEventScreenTouch:
		if event.pressed:
			next_page()


func _on_texture_rect_left_arrow_gui_input(event):
	if event is InputEventScreenTouch:
		if event.pressed:
			prev_page()


func prev_page():
	var next_index = item_index - 1
	if next_index < 0:
		next_index = item_count - 1

	set_item(next_index, false)


func next_page():
	var next_index = item_index + 1
	if next_index >= item_count:
		next_index = 0

	set_item(next_index, true)


func set_item(index: int, right_direction: bool = true):
	timer_auto_move_carousel.stop()
	timer_auto_move_carousel.start()

	var current_color = Color(bg_colors[item_index])
	item_index = index
	carousel.set_item(index, right_direction)

	var tween = get_tree().create_tween()
	var new_color = Color(bg_colors[index])
	tween.tween_method(set_shader_background_color, current_color, new_color, 0.25)


func set_shader_background_color(color: Color):
	var shader_material: ShaderMaterial = background.get_material()
	shader_material.set_shader_parameter("lineColor", color)


func set_progress(new_progress: float):
	progress = new_progress

	loading_progress_label.text = "LOADING %d%%" % floor(progress)
	var tween = get_tree().create_tween()
	var new_width = loading_progress.get_parent().size.x * (progress / 100.0)
	tween.tween_property(loading_progress, "size:x", new_width, 0.1)


func _on_timer_auto_move_carousel_timeout():
	next_page()
