class_name Modal
extends ColorRect

## Generic UI component for displaying modals.
## All business logic should be handled by ModalManager.

const V_MARGIN_RATIO_LANDSCAPE = 0.1  # Modal max height = 80%
const V_MARGIN_RATIO_PORTRAIT = 0.2  # Modal max height = 60%
const H_MARGIN_RATIO_LANDSCAPE = 0.275  # Modal max width = 45%
const H_MARGIN_RATIO_PORTRAIT = 0.14  # Modal max width = 72%

const H_MARGIN_CONTENT_LANDSCAPE = 80 * 2 / 3
const TOP_MARGIN_CONTENT_LANDSCAPE = 80 * 2 / 3
const BOTTOM_MARGIN_CONTENT_LANDSCAPE = 70 * 2 / 3
const H_MARGIN_CONTENT_PORTRAIT = 80 * 2 / 3
const TOP_MARGIN_CONTENT_PORTRAIT = 100 * 2 / 3
const BOTTOM_MARGIN_CONTENT_PORTRAIT = 90 * 2 / 3

const MODAL_ALERT_ICON = preload("res://assets/ui/modal-alert-icon.svg")
const MODAL_BLOCK_ICON = preload("res://assets/ui/modal-block-icon.svg")
const MODAL_CONNECTION_ICON = preload("res://assets/ui/modal-connection-icon.svg")

@onready var margin_container_modal: MarginContainer = %MarginContainer_Modal
@onready var margin_container_content: MarginContainer = %MarginContainer_Content
@onready var label_title: Label = %Label_Title
@onready var label_body: Label = %Label_Body
@onready var h_separator_url: HSeparator = %HSeparator_Url
@onready var label_url: Label = %Label_Url
@onready var icon: TextureRect = %Icon
@onready var button_secondary: Button = %Button_Secondary
@onready var button_primary: Button = %Button_Primary


func _ready() -> void:
	hide()


## Sets the modal title
func set_title(title: String) -> void:
	label_title.text = title


## Sets the modal body text
func set_body(body: String) -> void:
	label_body.text = body


## Sets the primary button text
func set_primary_button_text(text: String) -> void:
	button_primary.text = text


## Sets the secondary button text
func set_secondary_button_text(text: String) -> void:
	button_secondary.text = text


## Shows an icon in the modal
## @param texture: The texture to display (optional, uses alert icon by default)
func show_icon(texture: Texture2D = null) -> void:
	if texture:
		icon.texture = texture
	else:
		icon.texture = MODAL_ALERT_ICON
	icon.show()


## Hides the icon
func hide_icon() -> void:
	icon.hide()


## Shows the URL separator and URL text
## @param url_text: The URL text to display
func show_url(url_text: String) -> void:
	label_url.text = url_text
	h_separator_url.show()
	label_url.show()


## Hides the URL separator and URL text
func hide_url() -> void:
	h_separator_url.hide()
	label_url.hide()


## Resizes the modal based on window size and orientation
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
		margin_container_content.add_theme_constant_override(
			"margin_top", TOP_MARGIN_CONTENT_LANDSCAPE
		)
		margin_container_content.add_theme_constant_override(
			"margin_bottom", BOTTOM_MARGIN_CONTENT_LANDSCAPE
		)
		margin_container_content.add_theme_constant_override(
			"margin_left", H_MARGIN_CONTENT_LANDSCAPE
		)
		margin_container_content.add_theme_constant_override(
			"margin_right", H_MARGIN_CONTENT_LANDSCAPE
		)
	else:
		var v_margin = window_size.y * V_MARGIN_RATIO_PORTRAIT
		var h_margin = window_size.x * H_MARGIN_RATIO_PORTRAIT
		margin_container_modal.add_theme_constant_override("margin_top", v_margin)
		margin_container_modal.add_theme_constant_override("margin_bottom", v_margin)
		margin_container_modal.add_theme_constant_override("margin_left", h_margin)
		margin_container_modal.add_theme_constant_override("margin_right", h_margin)
		margin_container_content.add_theme_constant_override(
			"margin_top", TOP_MARGIN_CONTENT_PORTRAIT
		)
		margin_container_content.add_theme_constant_override(
			"margin_bottom", BOTTOM_MARGIN_CONTENT_PORTRAIT
		)
		margin_container_content.add_theme_constant_override(
			"margin_left", H_MARGIN_CONTENT_PORTRAIT
		)
		margin_container_content.add_theme_constant_override(
			"margin_right", H_MARGIN_CONTENT_PORTRAIT
		)
