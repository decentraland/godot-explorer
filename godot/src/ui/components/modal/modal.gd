class_name Modal
extends ColorRect

enum ModalType { EXTERNAL_LINK, TELEPORT, SCENE_TIMEOUT, CONNECTION_LOST }

const V_MARGIN_RATIO_LANDSCAPE = 0.1  #Modal max height = 80%
const V_MARGIN_RATIO_PORTRAIT = 0.2  #Modal max height = 60%
const H_MARGIN_RATIO_LANDSCAPE = 0.275  #Modal max width = 45%
const H_MARGIN_RATIO_PORTRAIT = 0.14  #Modal max width = 72%

const H_MARGIN_CONTENT_LANDSCAPE = 80 * 2 / 3
const TOP_MARGIN_CONTENT_LANDSCAPE = 80 * 2 / 3
const BOTTOM_MARGIN_CONTENT_LANDSCAPE = 70 * 2 / 3
const H_MARGIN_CONTENT_PORTRAIT = 80 * 2 / 3
const TOP_MARGIN_CONTENT_PORTRAIT = 100 * 2 / 3
const BOTTOM_MARGIN_CONTENT_PORTRAIT = 90 * 2 / 3

const MODAL_ALERT_ICON = preload("res://assets/ui/modal-alert-icon.svg")
const MODAL_BLOCK_ICON = preload("res://assets/ui/modal-block-icon.svg")
const MODAL_CONNECTION_ICON = preload("res://assets/ui/modal-connection-icon.svg")

const EXTERNAL_LINK_TITLE = "Open external link?"
const EXTERNAL_LINK_BODY = "You’re about to visit an external website. Make sure you trust this site before continuing."
const EXTERNAL_LINK_PRIMARY = "OPEN LINK"
const EXTERNAL_LINK_SECONDARY = "CANCEL"

const SCENE_TIMEOUT_TITLE = "Loading  is taking longer than expected"
const SCENE_TIMEOUT_BODY = "You can reload the experience, or jump in now, it should keep loading in the background."
const SCENE_TIMEOUT_PRIMARY = "RELOAD"
const SCENE_TIMEOUT_SECONDARY = "START ANYWAY"

const TELEPORT_TITLE = "Teleport"
const TELEPORT_BODY = "You’ll be traveling to "
const TELEPORT_PRIMARY = "JUMP TO"
const TELEPORT_SECONDARY = "CANCEL"

const CONNECTION_LOST_TITLE = "Connection lost"
const CONNECTION_LOST_BODY = "We can’t connect to Decentraland right now. Please check your connection and try again."
const CONNECTION_LOST_PRIMARY = "RETRY"
const CONNECTION_LOST_SECONDARY = "EXIT APP"

var url: String
var scene_id: String
var modal_type: ModalType
var location: Vector2i = Vector2i(0, 0)
var realm: String = Realm.MAIN_REALM
var destination_name: String = "Unknown Place"

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


func _set_title(title: String) -> void:
	label_title.text = title


func _set_body(body: String) -> void:
	label_body.text = body


func _set_modal_type(type: ModalType) -> void:
	modal_type = type
	_update_content_visibility()
	resize_modal()
	show()


func _update_content_visibility() -> void:
	_hide_content()
	match modal_type:
		ModalType.EXTERNAL_LINK:
			_show_external_link_content()
		ModalType.SCENE_TIMEOUT:
			_show_scene_timeout_content()
		ModalType.CONNECTION_LOST:
			_show_connection_lost_content()
		ModalType.TELEPORT:
			_show_teleport_content()
		_:
			return


func _hide_content() -> void:
	h_separator_url.hide()
	label_url.hide()
	icon.hide()


func _show_external_link_content() -> void:
	h_separator_url.show()
	label_url.text = url
	label_url.show()
	button_secondary.text = EXTERNAL_LINK_SECONDARY
	button_primary.text = EXTERNAL_LINK_PRIMARY


func _show_scene_timeout_content() -> void:
	icon.texture = MODAL_ALERT_ICON
	icon.show()
	button_secondary.text = SCENE_TIMEOUT_SECONDARY
	button_primary.text = SCENE_TIMEOUT_PRIMARY


func _show_connection_lost_content() -> void:
	icon.texture = MODAL_CONNECTION_ICON
	icon.show()
	button_secondary.text = CONNECTION_LOST_SECONDARY
	button_primary.text = CONNECTION_LOST_PRIMARY


func _show_teleport_content() -> void:
	button_secondary.text = TELEPORT_SECONDARY
	button_primary.text = TELEPORT_PRIMARY


func open_external_link(external_url: String) -> void:
	url = external_url
	_set_title(EXTERNAL_LINK_TITLE)
	_set_body(EXTERNAL_LINK_BODY)
	_set_modal_type(ModalType.EXTERNAL_LINK)


func open_scene_load_timeout() -> void:
	_set_title(SCENE_TIMEOUT_TITLE)
	_set_body(SCENE_TIMEOUT_BODY)
	_set_modal_type(ModalType.SCENE_TIMEOUT)


func open_connection_lost() -> void:
	_set_title(CONNECTION_LOST_TITLE)
	_set_body(CONNECTION_LOST_BODY)
	_set_modal_type(ModalType.CONNECTION_LOST)


func open_for_teleport(new_location: Vector2i, new_realm = null) -> void:
	location = new_location
	async_load_place_position()
	_set_title(TELEPORT_TITLE)
	_set_body(TELEPORT_BODY + str(location))
	_set_modal_type(ModalType.TELEPORT)


func _on_button_pressed() -> void:
	open_connection_lost()
	resize_modal()


func _on_button_2_pressed() -> void:
	open_external_link("www.google.com")
	resize_modal()


func _on_button_3_pressed() -> void:
	open_scene_load_timeout()
	resize_modal()


func _on_button_4_pressed() -> void:
	open_for_teleport(Vector2(22,5))
	resize_modal()

func async_load_place_position():
	var place_url: String = "https://places.decentraland.org/api/places?limit=1"
	place_url += "&positions=%d,%d" % [location.x, location.y]

	var headers = {"Content-Type": "application/json"}
	var promise: Promise = Global.http_requester.request_json(
		place_url, HTTPClient.METHOD_GET, "", headers
	)
	var result = await PromiseUtils.async_awaiter(promise)

	if result is PromiseError:
		printerr("Error request places jump in", result.get_error())
		return

	var json: Dictionary = result.get_string_response_as_json()

	if json.data.is_empty():
		destination_name = "Unknown Place"
	else:
		var title = json.data[0].get("title", "interactive-text")
		if title != "interactive-text":
			destination_name = title
		else:
			destination_name = "Unknown Place"
	_set_body(TELEPORT_BODY + destination_name)


func _on_button_primary_pressed() -> void:
	match modal_type:
		ModalType.EXTERNAL_LINK:
			Global.open_url(url)
		ModalType.SCENE_TIMEOUT:
			Global.reload_scene.emit()
		ModalType.CONNECTION_LOST:
			pass
		ModalType.TELEPORT:
			Global.teleport_to(location, realm)
		_:
			pass
	hide()


func _on_button_secondary_pressed() -> void:
	match modal_type:
		ModalType.EXTERNAL_LINK:
			pass
		ModalType.SCENE_TIMEOUT:
			Global.run_anyway.emit()
		ModalType.CONNECTION_LOST:
			pass
		ModalType.TELEPORT:
			pass
		_:
			pass
	hide()
