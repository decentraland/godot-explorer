class_name TravelModal
extends ColorRect

signal jump_in_pressed
signal closed

const LANDSCAPE_IMAGE_MIN_SIZE := Vector2(600, 460)
const LANDSCAPE_CONTENT_MIN_SIZE := Vector2(460, 460)

@onready var panel_container: PanelContainer = $PanelContainer
@onready var image_container: Control = %Control
@onready var panel_skeleton: Panel = %Panel_Skeleton
@onready var texture_rect_image: TextureRect = %TextureRect_Image
@onready var margin_container_content: MarginContainer = %MarginContainer_Content
@onready var panel_skeleton_title: Panel = %Panel_SkeletonTitle
@onready var panel_skeleton_creator: Panel = %Panel_SkeletonCreator
@onready var label_title: Label = %Label_Title
@onready var hbox_creator: HBoxContainer = %HBoxContainer_Creator
@onready var label_creator: Label = %Label_Creator
@onready var button_close: Button = %Button_Close
@onready var button_jump_in: Button = %Button_JumpIn


func _ready() -> void:
	hide()
	button_close.pressed.connect(func(): closed.emit())
	button_jump_in.pressed.connect(func(): jump_in_pressed.emit())
	_show_loading_state()


func _notification(what: int) -> void:
	if what == NOTIFICATION_VISIBILITY_CHANGED and visible:
		_update_min_sizes()


func _show_loading_state() -> void:
	# Image: skeleton visible, image hidden
	texture_rect_image.hide()
	panel_skeleton.show()
	# Content: skeletons visible, labels hidden, button disabled
	panel_skeleton_title.show()
	panel_skeleton_creator.show()
	label_title.hide()
	hbox_creator.hide()
	button_jump_in.disabled = true


func _show_content() -> void:
	panel_skeleton_title.hide()
	panel_skeleton_creator.hide()
	label_title.show()
	button_jump_in.disabled = false


func _update_min_sizes() -> void:
	if not is_instance_valid(image_container) or not is_instance_valid(margin_container_content):
		return
	if Global.is_orientation_portrait():
		image_container.custom_minimum_size = Vector2.ZERO
		margin_container_content.custom_minimum_size = Vector2.ZERO
	else:
		image_container.custom_minimum_size = LANDSCAPE_IMAGE_MIN_SIZE
		margin_container_content.custom_minimum_size = LANDSCAPE_CONTENT_MIN_SIZE


func set_place_name(place_name: String) -> void:
	label_title.text = place_name
	_show_content()


func set_creator(creator: String) -> void:
	if creator.is_empty():
		hbox_creator.hide()
	else:
		label_creator.text = creator
		hbox_creator.show()


func set_image(texture: Texture2D) -> void:
	if texture:
		texture_rect_image.texture = texture
		panel_skeleton.hide()
		texture_rect_image.show()


func _async_update_modal_size() -> void:
	if not is_inside_tree():
		return
	if panel_container and panel_container.has_method("_request_update"):
		await get_tree().process_frame
		await get_tree().process_frame
		panel_container._request_update()


func _on_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch and event.pressed:
		closed.emit()


func _on_button_close_pressed() -> void:
	closed.emit()
