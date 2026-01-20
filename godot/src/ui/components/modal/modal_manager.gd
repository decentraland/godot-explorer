extends Node

## Global manager to show modals from anywhere in the application.
## Does not require the modal to be previously in any scene.
## Contains all business logic for different modal types.

const MODAL_SCENE_PATH = "res://src/ui/components/modal/modal.tscn"

# Modal text constants
const EXTERNAL_LINK_TITLE = "Open external link?"
const EXTERNAL_LINK_BODY = "You're about to visit an external website. Make sure you trust this site before continuing."
const EXTERNAL_LINK_PRIMARY = "OPEN LINK"
const EXTERNAL_LINK_SECONDARY = "CANCEL"

const SCENE_TIMEOUT_TITLE = "Loading  is taking longer than expected"
const SCENE_TIMEOUT_BODY = "You can reload the experience, or jump in now, it should keep loading in the background."
const SCENE_TIMEOUT_PRIMARY = "RELOAD"
const SCENE_TIMEOUT_SECONDARY = "START ANYWAY"

const TELEPORT_TITLE = "Teleport"
const TELEPORT_BODY = "You'll be traveling to "
const TELEPORT_PRIMARY = "JUMP TO"
const TELEPORT_SECONDARY = "CANCEL"

const CONNECTION_LOST_TITLE = "Connection lost"
const CONNECTION_LOST_BODY = "We can't connect to Decentraland right now. Please check your connection and try again."
const CONNECTION_LOST_PRIMARY = "RETRY"
const CONNECTION_LOST_SECONDARY = "EXIT APP"

var current_modal: Modal = null
var modal_scene: PackedScene = null

func _ready() -> void:
	modal_scene = load(MODAL_SCENE_PATH)
	if not modal_scene:
		push_error("ModalManager: Could not load modal scene at: " + MODAL_SCENE_PATH)


## Shows an EXTERNAL_LINK type modal
## @param external_url: The external URL to open
func show_external_link_modal(external_url: String) -> void:
	var modal = _create_modal()
	if not modal:
		return
	
	modal.set_title(EXTERNAL_LINK_TITLE)
	modal.set_body(EXTERNAL_LINK_BODY)
	modal.set_primary_button_text(EXTERNAL_LINK_PRIMARY)
	modal.set_secondary_button_text(EXTERNAL_LINK_SECONDARY)
	modal.show_url(external_url)
	modal.show_icon(Modal.MODAL_ALERT_ICON)
	modal.show()
	
	# Connect button actions - always use default behavior
	modal.button_primary.pressed.connect(_on_external_link_primary.bind(external_url))
	modal.button_secondary.pressed.connect(close_current_modal)


## Shows a SCENE_TIMEOUT type modal
func show_scene_timeout_modal() -> void:
	var modal = _create_modal()
	if not modal:
		return
	
	modal.set_title(SCENE_TIMEOUT_TITLE)
	modal.set_body(SCENE_TIMEOUT_BODY)
	modal.set_primary_button_text(SCENE_TIMEOUT_PRIMARY)
	modal.set_secondary_button_text(SCENE_TIMEOUT_SECONDARY)
	modal.show_icon(Modal.MODAL_ALERT_ICON)
	modal.show()
	
	# Connect button actions
	modal.button_primary.pressed.connect(_on_scene_timeout_primary)
	modal.button_secondary.pressed.connect(_on_scene_timeout_secondary)


## Shows a CONNECTION_LOST type modal
func show_connection_lost_modal() -> void:
	var modal = _create_modal()
	if not modal:
		return
	
	modal.set_title(CONNECTION_LOST_TITLE)
	modal.set_body(CONNECTION_LOST_BODY)
	modal.set_primary_button_text(CONNECTION_LOST_PRIMARY)
	modal.set_secondary_button_text(CONNECTION_LOST_SECONDARY)
	modal.show_icon(Modal.MODAL_CONNECTION_ICON)
	modal.hide_url()
	modal.show()
	
	# Connect button actions
	modal.button_primary.pressed.connect(_on_connection_lost_primary)
	modal.button_secondary.pressed.connect(_on_connection_lost_secondary)


## Shows a TELEPORT type modal
## @param location: The position to teleport to
## @param realm: The destination realm (optional)
func show_teleport_modal(location: Vector2i, realm: String = "") -> void:
	var modal = _create_modal()
	if not modal:
		return
	
	var destination_realm = realm if not realm.is_empty() else Realm.MAIN_REALM
	
	modal.set_title(TELEPORT_TITLE)
	modal.set_body(TELEPORT_BODY + str(location))
	modal.set_primary_button_text(TELEPORT_PRIMARY)
	modal.set_secondary_button_text(TELEPORT_SECONDARY)
	modal.hide_icon()
	modal.hide_url()
	
	# Load place name asynchronously
	_load_place_name(modal, location)
	
	# Connect button actions - always use default behavior
	modal.button_primary.pressed.connect(_on_teleport_primary.bind(location, destination_realm))
	modal.button_secondary.pressed.connect(_remove_modal)


## Shows a CHANGE_REALM type modal
## @param realm_name: The destination realm name
## @param message: Optional message from the scene
func show_change_realm_modal(realm_name: String, message: String = "") -> void:
	var modal = _create_modal()
	if not modal:
		return
	
	var body_text = "The scene wants to move you to a new realm\nTo: `" + realm_name + "`"
	if not message.is_empty():
		body_text += "\nScene message: " + message
	
	modal.set_title("Change Realm")
	modal.set_body(body_text)
	modal.set_primary_button_text("Let's go!")
	modal.set_secondary_button_text("No thanks")
	modal.hide_icon()
	modal.hide_url()
	modal.show()
	
	# Connect button actions - always use default behavior
	modal.button_primary.pressed.connect(_on_change_realm_primary.bind(realm_name))
	modal.button_secondary.pressed.connect(_remove_modal)


## Closes the current modal if it exists
func close_current_modal() -> void:
	if current_modal:
		current_modal.hide()
		_remove_modal()


func _create_modal() -> Modal:
	# If there's already a modal open, close it first
	if current_modal:
		close_current_modal()
	
	if not modal_scene:
		push_error("ModalManager: Modal scene is not loaded")
		return null
	
	var modal = modal_scene.instantiate() as Modal
	if not modal:
		push_error("ModalManager: Could not instantiate modal")
		return null
	
	# Add the modal to the main viewport so it's always visible
	# Use get_tree().root to ensure it's at the highest level
	var root = get_tree().root
	if root:
		root.add_child(modal)
		current_modal = modal
		
		# Connect signal to clean up when modal exits the tree
		modal.tree_exited.connect(_on_modal_tree_exited)
		
		modal.hide_url()
		modal.hide_icon()
		modal.resize_modal()
		return modal
	else:
		push_error("ModalManager: Could not get scene tree root")
		return null


func _load_place_name(modal: Modal, location: Vector2i) -> void:
	var place_url: String = "https://places.decentraland.org/api/places?limit=1"
	place_url += "&positions=%d,%d" % [location.x, location.y]
	
	var headers = {"Content-Type": "application/json"}
	var promise: Promise = Global.http_requester.request_json(
		place_url, HTTPClient.METHOD_GET, "", headers
	)
	var result = await PromiseUtils.async_awaiter(promise)
	
	if result is PromiseError:
		printerr("Error requesting place name for teleport", result.get_error())
		return
	
	var json: Dictionary = result.get_string_response_as_json()
	var destination_name: String = "Unknown Place"
	
	if not json.data.is_empty():
		var title = json.data[0].get("title", "interactive-text")
		if title != "interactive-text":
			destination_name = title
	
	# Update modal body with place name
	if modal and is_instance_valid(modal):
		modal.set_body(TELEPORT_BODY + destination_name)
	modal.show()

# Button action handlers
func _on_external_link_primary(url: String) -> void:
	Global.open_url(url)
	_remove_modal()


func _on_scene_timeout_primary() -> void:
	Global.metrics.track_click_button("reload", "LOADING", "")
	Global.realm.async_set_realm(Global.realm.get_realm_string())
	_remove_modal()


func _on_scene_timeout_secondary() -> void:
	Global.metrics.track_click_button("run_anyway", "LOADING", "")
	Global.run_anyway.emit()
	_remove_modal()


func _on_connection_lost_primary() -> void:
	# Retry connection logic would go here
	_remove_modal()


func _on_connection_lost_secondary() -> void:
	# Exit app logic would go here
	_remove_modal()


func _on_teleport_primary(location: Vector2i, realm: String) -> void:
	Global.teleport_to(location, realm)
	_remove_modal()


func _on_change_realm_primary(realm_name: String) -> void:
	# Default behavior: call Global.realm.async_set_realm
	Global.realm.async_set_realm(realm_name)
	_remove_modal()


func _on_modal_tree_exited() -> void:
	# Modal was removed from tree, clear reference
	if current_modal:
		current_modal = null


func _on_modal_button_pressed() -> void:
	# Wait a frame so the modal can execute its actions first
	await get_tree().process_frame
	if current_modal and not current_modal.visible:
		# Modal was hidden, remove it from tree
		_remove_modal()


func _remove_modal() -> void:
	if current_modal:
		current_modal.queue_free()
		current_modal = null
