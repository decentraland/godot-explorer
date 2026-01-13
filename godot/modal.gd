extends ColorRect

const V_MARGIN_RATIO_LANDSCAPE = 0.1 #Modal max height = 80%
const V_MARGIN_RATIO_PORTRAIT = 0.2 #Modal max height = 60%
const H_MARGIN_RATIO_LANDSCAPE = 0.275 #Modal max width = 45%
const H_MARGIN_RATIO_PORTRAIT = 0.14 #Modal max width = 72%

const H_MARGIN_CONTENT_LANDSCAPE = 80 * 2/3
const TOP_MARGIN_CONTENT_LANDSCAPE = 80 * 2/3
const BOTTOM_MARGIN_CONTENT_LANDSCAPE = 70 * 2/3
const H_MARGIN_CONTENT_PORTRAIT = 80 * 2/3
const TOP_MARGIN_CONTENT_PORTRAIT = 100 * 2/3
const BOTTOM_MARGIN_CONTENT_PORTRAIT = 90 * 2/3

enum ModalType {
	EXTERNAL_LINK,
	JUMP_TO,
	TAKING_TOO_LONG
}

var url: String
var modal_type: ModalType

@onready var margin_container_modal: MarginContainer = %MarginContainer_Modal
@onready var margin_container_content: MarginContainer = %MarginContainer_Content
@onready var label_title: Label = %Label_Title
@onready var label_body: Label = %Label_Body
@onready var h_separator_url: HSeparator = %HSeparator_Url
@onready var label_url: Label = %Label_Url

func _ready() -> void:
	resize_modal()

func resize_modal() -> void:
	var window_size: Vector2i = DisplayServer.window_get_size()
	var is_landscape: bool = window_size.x > window_size.y
	if is_landscape:
		var v_margin = window_size.y * V_MARGIN_RATIO_LANDSCAPE
		var h_margin = window_size.x * H_MARGIN_RATIO_LANDSCAPE
		margin_container_modal.add_theme_constant_override("margin_top", v_margin)
		margin_container_modal.add_theme_constant_override("margin_bottom", v_margin)
		margin_container_modal.add_theme_constant_override("margin_left", h_margin)
		margin_container_modal.add_theme_constant_override("margin_right", h_margin)
		margin_container_content.add_theme_constant_override("margin_top", TOP_MARGIN_CONTENT_LANDSCAPE)
		margin_container_content.add_theme_constant_override("margin_bottom", BOTTOM_MARGIN_CONTENT_LANDSCAPE)
		margin_container_content.add_theme_constant_override("margin_left", H_MARGIN_CONTENT_LANDSCAPE)
		margin_container_content.add_theme_constant_override("margin_right", H_MARGIN_CONTENT_LANDSCAPE)
		
	else:
		var v_margin = window_size.y * V_MARGIN_RATIO_PORTRAIT	
		var h_margin = window_size.x * H_MARGIN_RATIO_PORTRAIT	
		margin_container_modal.add_theme_constant_override("margin_top", v_margin)
		margin_container_modal.add_theme_constant_override("margin_bottom", v_margin)
		margin_container_modal.add_theme_constant_override("margin_left", h_margin)
		margin_container_modal.add_theme_constant_override("margin_right", h_margin)
		margin_container_content.add_theme_constant_override("margin_top", TOP_MARGIN_CONTENT_PORTRAIT)
		margin_container_content.add_theme_constant_override("margin_bottom", BOTTOM_MARGIN_CONTENT_PORTRAIT)
		margin_container_content.add_theme_constant_override("margin_left", H_MARGIN_CONTENT_PORTRAIT)
		margin_container_content.add_theme_constant_override("margin_right", H_MARGIN_CONTENT_PORTRAIT)

func _set_title(title: String) -> void:
	label_title.text = title
	
	
func _set_body(body: String) -> void:
	label_body.text = body
	
	
func _on_button_secondary_pressed() -> void:
	open_external_link("www.google.com")
	resize_modal()
	
	
func _set_modal_type(type: ModalType) -> void:
	modal_type = type
	_update_object_visibility()
	
	
func open_external_link(external_url: String) -> void:
	url = external_url
	_set_title("Open external link?")
	_set_body("You’re about to visit an external website. Make sure you trust this site before continuing.")
	_set_modal_type(ModalType.EXTERNAL_LINK)
	
func _update_object_visibility() -> void:
	h_separator_url.hide()
	label_url.hide()
	match modal_type:
		ModalType.EXTERNAL_LINK:
			h_separator_url.show()
			label_url.text = url
			label_url.show()
		_:
			return
