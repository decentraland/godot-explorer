class_name Modal
extends ColorRect

const MODAL_ALERT_ICON = preload("res://assets/ui/modal-alert-icon.svg")
const MODAL_BLOCK_ICON = preload("res://assets/ui/modal-block-icon.svg")
const MODAL_BAN_ICON = preload("res://assets/ui/modal-ban-icon.svg")
const MODAL_CONNECTION_ICON = preload("res://assets/ui/modal-connection-icon.svg")
const MODAL_SUCCESS_ICON = preload("res://assets/ui/modal-success-icon.svg")

# When true, the modal cannot be dismissed and consumes all input behind it.
var blocker: bool = false

@onready var margin_container_content: MarginContainer = %MarginContainer_Content
@onready var label_title: Label = %Label_Title
@onready var label_body: Label = %Label_Body
@onready var h_separator_url: HSeparator = %HSeparator_Url
@onready var label_url: Label = %Label_Url
@onready var icon: TextureRect = %Icon
@onready var button_secondary: Button = %Button_Secondary
@onready var button_primary: Button = %Button_Primary
@onready var panel_container: PanelContainer = $PanelContainer
@onready var buttons_separator: HSeparator = %HSeparator_ButtonsSeparator
@onready var buttons_container: HBoxContainer = %HBoxContainer_Buttons
@onready var checkbox_container: HBoxContainer = %CheckBox_Container
@onready var checkbox: CheckBox = %CheckBox
@onready var checkbox_text: RichTextLabel = %CheckBox_Text


func _ready() -> void:
	hide()
	checkbox_container.hide()


func _notification(what: int) -> void:
	if what == NOTIFICATION_VISIBILITY_CHANGED:
		if visible:
			# Update size when modal becomes visible
			_async_update_modal_size()


## Sets the modal title. Takes a key, not text: Label_Title auto-translates, so the
## raw key is what keeps it correct across a language change.
func set_title(title: TranslationKey) -> void:
	label_title.auto_translate_mode = Node.AUTO_TRANSLATE_MODE_INHERIT
	label_title.text = title.raw()


## Sets the modal title from text already composed by the caller — a key filled with a
## place name, a server string. Use [method set_title] whenever a plain key will do; this
## exists because a formatted result is no longer a key and must not be looked up.
func set_title_text(title: String) -> void:
	label_title.auto_translate_mode = Node.AUTO_TRANSLATE_MODE_DISABLED
	label_title.text = title


## Sets the modal body from one catalogue entry.
func set_body(body: TranslationKey) -> void:
	# Symmetric with set_body_parts, which has to disable this: a modal reused for a
	# single-key body would otherwise draw the raw key.
	label_body.auto_translate_mode = Node.AUTO_TRANSLATE_MODE_INHERIT
	label_body.text = body.raw()
	_async_update_modal_size()


## Sets the modal body from text already composed by the caller, or clears it. Prefer
## [method set_body]; this exists for a formatted result, which is no longer a key.
func set_body_text(body: String) -> void:
	label_body.auto_translate_mode = Node.AUTO_TRANSLATE_MODE_DISABLED
	label_body.text = body
	_async_update_modal_size()


## Sets the modal body from several entries, rendered as separate paragraphs.
##
## Each is translated on its own, so a translator can rewrite one without the others.
## Never build this by concatenating a key with a literal — the result matches no key,
## the lookup misses, and the raw key is drawn on screen.
func set_body_parts(parts: Array[TranslationKey]) -> void:
	# Joining forces resolution here, so the label must not look the result up again.
	label_body.auto_translate_mode = Node.AUTO_TRANSLATE_MODE_DISABLED
	label_body.text = TranslationKey.join(parts)
	_async_update_modal_size()


## Sets the primary button text
func set_primary_button_text(text: TranslationKey) -> void:
	button_primary.text = text.raw()


## Sets the primary button font size
func set_primary_button_font_size(size: int) -> void:
	button_primary.add_theme_font_size_override("font_size", size)


## Sets the secondary button text
func set_secondary_button_text(text: TranslationKey) -> void:
	button_secondary.text = text.raw()


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


## Shows the consent checkbox, whose label is BBCode from the catalogue.
func show_checkbox(bbcode_text: TranslationKey) -> void:
	checkbox.button_pressed = false
	checkbox_text.text = bbcode_text.raw()
	checkbox_container.show()
	label_body.hide()
	_async_update_modal_size()


func hide_checkbox() -> void:
	checkbox_container.hide()
	checkbox.button_pressed = false
	label_body.show()


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


func _unhandled_input(_event: InputEvent) -> void:
	if not visible or not blocker:
		return
	get_viewport().set_input_as_handled()


func _on_gui_input(event: InputEvent) -> void:
	if blocker:
		return
	if event is InputEventScreenTouch:
		if event.pressed:
			Global.modal_manager.close_current_modal()
