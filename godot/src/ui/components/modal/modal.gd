class_name Modal
extends ColorRect

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
