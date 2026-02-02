class_name Modal
extends ColorRect

const MODAL_ALERT_ICON = preload("res://assets/ui/modal-alert-icon.svg")
const MODAL_BLOCK_ICON = preload("res://assets/ui/modal-block-icon.svg")
const MODAL_CONNECTION_ICON = preload("res://assets/ui/modal-connection-icon.svg")

var dismissable: bool = true

@onready var margin_container_content: MarginContainer = %MarginContainer_Content
@onready var label_title: Label = %Label_Title
@onready var label_body: Label = %Label_Body
@onready var h_separator_url: HSeparator = %HSeparator_Url
@onready var label_url: Label = %Label_Url
@onready var icon: TextureRect = %Icon
@onready var button_secondary: Button = %Button_Secondary
@onready var button_primary: Button = %Button_Primary
@onready var panel_container: PanelContainer = $PanelContainer


func _ready() -> void:
	hide()


func _notification(what: int) -> void:
	if what == NOTIFICATION_VISIBILITY_CHANGED:
		if visible:
			# Update size when modal becomes visible
			_async_update_modal_size()


## Sets the modal title
func set_title(title: String) -> void:
	label_title.text = title


## Sets the modal body text
func set_body(body: String) -> void:
	label_body.text = body
	_async_update_modal_size()


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
	if url_text.length() > 106:
		label_url.text = url_text.left(103) + "..."
	else:
		label_url.text = url_text
	h_separator_url.show()
	label_url.show()


## Hides the URL separator and URL text
func hide_url() -> void:
	h_separator_url.hide()
	label_url.hide()
	_async_update_modal_size()


## Updates the modal size to fit its content
func _async_update_modal_size() -> void:
	if not is_inside_tree():
		return

	# Force ResponsiveContainer to recalculate size after content changes
	if panel_container and panel_container.has_method("_request_update"):
		# Wait for multiple frames to ensure:
		# 1. Content has been laid out
		# 2. Viewport size is correct (especially when called from SDK)
		# 3. All @onready nodes are initialized
		await get_tree().process_frame
		await get_tree().process_frame
		panel_container._request_update()


func _on_gui_input(event: InputEvent) -> void:
	if not dismissable:
		return
	if event is InputEventScreenTouch:
		if event.pressed:
			Global.modal_manager.close_current_modal()
