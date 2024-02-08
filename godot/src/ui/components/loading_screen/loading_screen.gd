extends Control

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
var last_progress_change := Time.get_ticks_msec()
var popup_warning_pos_y: int = 0

var last_hide_click := 0.0

@onready var loading_progress = %ColorRect_LoadingProgress
@onready var loading_progress_label = %Label_LoadingProgress

@onready
var carousel = $VBox_Loading/ColorRect_Background/Control_Discover/VBoxContainer/HBoxContainer_Content/CarouselViewport/SubViewport/Carousel
@onready var background: ColorRect = $VBox_Loading/ColorRect_Background

@onready var timer_auto_move_carousel = $Timer_AutoMoveCarousel
@onready var popup_warning = $PopupWarning

@onready var loading_screen_progress_logic = $LoadingScreenProgressLogic
@onready var timer_check_progress_timeout = $Timer_CheckProgressTimeout


func _ready():
	last_progress_change = Time.get_ticks_msec()
	popup_warning.hide()
	popup_warning_pos_y = popup_warning.position.y
	item_count = carousel.item_count()
	set_item(randi_range(0, item_count - 1))


# Forward
func enable_loading_screen():
	Global.release_mouse()
	loading_screen_progress_logic.enable_loading_screen()


func async_hide_loading_screen_effect():
	Global.loading_finished.emit()
	timer_check_progress_timeout.stop()
	var tween = get_tree().create_tween()
	background.use_parent_material = true  # disable material
	modulate = Color.WHITE
	tween.tween_property(self, "modulate", Color.TRANSPARENT, 1.0)
	await tween.finished
	hide()
	modulate = Color.WHITE
	background.use_parent_material = false  # enable material
	self.position.y = 0


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

	var new_color = Color(bg_colors[index])
	set_bg_shader_color(current_color, new_color)


func set_bg_shader_color(from: Color, to: Color):
	var tween = get_tree().create_tween()
	tween.tween_method(set_shader_background_color, from, to, 0.25)


func set_shader_background_color(color: Color):
	var shader_material: ShaderMaterial = background.get_material()
	shader_material.set_shader_parameter("lineColor", color)


func set_progress(new_progress: float):
	new_progress = clampf(new_progress, 0.0, 100.0)
	if progress != new_progress:
		last_progress_change = Time.get_ticks_msec()
	progress = new_progress

	loading_progress_label.text = "LOADING %d%%" % floor(progress)
	var tween = get_tree().create_tween()
	var new_width = loading_progress.get_parent().size.x * (progress / 100.0)
	tween.tween_property(loading_progress, "position:x", new_width, 0.1)


func _on_timer_auto_move_carousel_timeout():
	next_page()


func _on_timer_check_progress_timeout_timeout():
	if Global.scene_runner.is_paused():
		last_progress_change = Time.get_ticks_msec()
		return

	var inactive_seconds: int = int(floor((Time.get_ticks_msec() - last_progress_change) / 1000.0))
	if inactive_seconds > 20:
		var tween = get_tree().create_tween()
		popup_warning.position.y = -popup_warning.size.y
		tween.tween_property(popup_warning, "position:y", popup_warning_pos_y, 1.0).set_trans(
			Tween.TRANS_ELASTIC
		)
		popup_warning.show()
		timer_check_progress_timeout.stop()


func async_hide_popup_warning():
	var tween = get_tree().create_tween()
	popup_warning.position.y = popup_warning_pos_y
	tween.tween_property(popup_warning, "position:y", -popup_warning.size.y, 1.0).set_trans(
		Tween.TRANS_ELASTIC
	)
	await tween.finished
	popup_warning.hide()


func _on_button_continue_pressed():
	loading_screen_progress_logic.hide_loading_screen()
	async_hide_popup_warning()


func _on_button_reload_pressed():
	Global.realm.async_set_realm(Global.realm.get_realm_string())
	async_hide_popup_warning()


func _on_loading_screen_progress_logic_loading_show_requested():
	last_progress_change = Time.get_ticks_msec()
	popup_warning.hide()
	timer_check_progress_timeout.start()


# For dev purposes
func _on_color_rect_header_gui_input(event):
	if event is InputEventScreenTouch:
		if event.pressed:
			var elapsed_time = Time.get_ticks_msec() - last_hide_click
			if elapsed_time <= 500:
				loading_screen_progress_logic.hide_loading_screen()
			last_hide_click = Time.get_ticks_msec()
