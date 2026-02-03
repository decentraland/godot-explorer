class_name ModalManager
extends Node

## Global manager to show modals from anywhere in the application.
## Does not require the modal to be previously in any scene.
## Contains all business logic for different modal types.

signal connection_lost_retry
signal connection_lost_exit

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
func async_show_external_link_modal(external_url: String) -> void:
	if not current_modal:
		if not await _async_create_modal():
			print("NOT CREATED MODAL")
			return

	current_modal.set_title(EXTERNAL_LINK_TITLE)
	current_modal.set_body(EXTERNAL_LINK_BODY)
	current_modal.set_primary_button_text(EXTERNAL_LINK_PRIMARY)
	current_modal.set_secondary_button_text(EXTERNAL_LINK_SECONDARY)
	current_modal.show_url(external_url)
	current_modal.show_icon(Modal.MODAL_ALERT_ICON)
	current_modal.show()

	# Disconnect previous connections and connect button actions
	_disconnect_button_signals()
	current_modal.button_primary.pressed.connect(_on_external_link_primary.bind(external_url))
	current_modal.button_secondary.pressed.connect(close_current_modal)


## Shows a SCENE_TIMEOUT type modal
func async_show_scene_timeout_modal() -> void:
	if not current_modal:
		if not await _async_create_modal():
			print("NOT CREATED MODAL")
			return

	current_modal.set_title(SCENE_TIMEOUT_TITLE)
	current_modal.set_body(SCENE_TIMEOUT_BODY)
	current_modal.set_primary_button_text(SCENE_TIMEOUT_PRIMARY)
	current_modal.set_secondary_button_text(SCENE_TIMEOUT_SECONDARY)
	current_modal.show_icon(Modal.MODAL_ALERT_ICON)
	current_modal.show()

	# Disconnect previous connections and connect button actions
	_disconnect_button_signals()
	current_modal.button_primary.pressed.connect(_on_scene_timeout_primary)
	current_modal.button_secondary.pressed.connect(_on_scene_timeout_secondary)


## Shows a CONNECTION_LOST type modal
func async_show_connection_lost_modal() -> void:
	if not current_modal:
		if not await _async_create_modal():
			print("NOT CREATED MODAL")
			return

	current_modal.dismissable = false
	current_modal.set_title(CONNECTION_LOST_TITLE)
	current_modal.set_body(CONNECTION_LOST_BODY)
	current_modal.set_primary_button_text(CONNECTION_LOST_PRIMARY)
	current_modal.set_secondary_button_text(CONNECTION_LOST_SECONDARY)
	current_modal.show_icon(Modal.MODAL_CONNECTION_ICON)
	current_modal.hide_url()
	current_modal.show()

	# Disconnect previous connections and connect button actions
	_disconnect_button_signals()
	current_modal.button_primary.pressed.connect(_on_connection_lost_primary)
	current_modal.button_secondary.pressed.connect(_on_connection_lost_secondary)


## Shows a TELEPORT type modal
## @param location: The position to teleport to
## @param realm: The destination realm (optional)
func async_show_teleport_modal(location: Vector2i, realm: String = "") -> void:
	if not current_modal:
		if not await _async_create_modal():
			print("NOT CREATED MODAL")
			return

	var destination_realm = realm if not realm.is_empty() else DclUrls.main_realm()

	current_modal.set_title(TELEPORT_TITLE)
	current_modal.set_body(TELEPORT_BODY + str(location))
	current_modal.set_primary_button_text(TELEPORT_PRIMARY)
	current_modal.set_secondary_button_text(TELEPORT_SECONDARY)
	current_modal.hide_icon()
	current_modal.hide_url()

	# Disconnect previous connections and connect button actions
	_disconnect_button_signals()
	current_modal.button_primary.pressed.connect(
		_on_teleport_primary.bind(location, destination_realm)
	)
	current_modal.button_secondary.pressed.connect(close_current_modal)

	# Show modal immediately so ResponsiveContainer can calculate size correctly
	# This is especially important when called from SDK/Rust
	current_modal.show()

	# Wait a bit for the modal to be fully visible and layout calculated
	await get_tree().process_frame
	await get_tree().process_frame

	# Load place name asynchronously and update modal
	await _async_load_place_name(location)


## Shows a CHANGE_REALM type modal
## @param realm_name: The destination realm name
## @param message: Optional message from the scene
func async_show_change_realm_modal(realm_name: String, message: String = "") -> void:
	if not current_modal:
		if not await _async_create_modal():
			print("NOT CREATED MODAL")
			return

	var body_text = "The scene wants to move you to a new realm\nTo: `" + realm_name + "`"
	if not message.is_empty():
		body_text += "\nScene message: " + message

	current_modal.set_title("Change Realm")
	current_modal.set_body(body_text)
	current_modal.set_primary_button_text("Let's go!")
	current_modal.set_secondary_button_text("No thanks")
	current_modal.hide_icon()
	current_modal.hide_url()
	current_modal.show()

	# Disconnect previous connections and connect button actions
	_disconnect_button_signals()
	current_modal.button_primary.pressed.connect(_on_change_realm_primary.bind(realm_name))
	current_modal.button_secondary.pressed.connect(close_current_modal)


## Closes the current modal if it exists
func close_current_modal() -> void:
	if current_modal:
		current_modal.hide()
		_remove_modal()


## Disconnects all button signals from the current modal
func _disconnect_button_signals() -> void:
	if not current_modal:
		return

	# Disconnect all connections from primary button
	if current_modal.button_primary:
		for connection in current_modal.button_primary.pressed.get_connections():
			current_modal.button_primary.pressed.disconnect(connection.callable)

	# Disconnect all connections from secondary button
	if current_modal.button_secondary:
		for connection in current_modal.button_secondary.pressed.get_connections():
			current_modal.button_secondary.pressed.disconnect(connection.callable)


func _async_create_modal() -> Modal:
	# If there's already a modal open, close it first
	if current_modal:
		close_current_modal()

	if not modal_scene:
		push_error("ModalManager: Modal scene is not loaded at: " + MODAL_SCENE_PATH)
		return null

	var modal = modal_scene.instantiate() as Modal
	if not modal:
		push_error("ModalManager: Could not instantiate modal from scene")
		return null

	# Add the modal to the main viewport so it's always visible
	# Use get_tree().root to ensure it's at the highest level
	var root = get_tree().root
	if not root:
		push_error("ModalManager: Could not get scene tree root")
		return null

	root.add_child(modal)
	current_modal = modal

	# Connect signal to clean up when modal exits the tree
	current_modal.tree_exited.connect(_on_modal_tree_exited)

	current_modal.hide_url()
	current_modal.hide_icon()

	# Wait for modal to be fully in tree and @onready nodes initialized
	# This is especially important when called from SDK/Rust
	await get_tree().process_frame
	await get_tree().process_frame

	return modal


func _async_load_place_name(location: Vector2i) -> void:
	if !is_instance_valid(current_modal):
		# Modal was already freed, cannot recreate it here as it would lose button connections
		return

	var result = await PlacesHelper.async_get_by_position(location)

	if result is PromiseError:
		printerr("Error requesting place name for teleport", result.get_error())
		return

	# Check if modal is still valid after await (it might have been closed)
	if !is_instance_valid(current_modal):
		return

	var json: Dictionary = result.get_string_response_as_json()
	var destination_name: String = "Unknown Place"

	if not json.data.is_empty():
		var title = json.data[0].get("title", "interactive-text")
		if title != "interactive-text":
			destination_name = title

	# Update modal body with place name
	# Double check validity before updating (modal might have been closed during await)
	if is_instance_valid(current_modal):
		current_modal.set_body(TELEPORT_BODY + destination_name)
		# Modal is already shown, just update the size


# Button action handlers
func _on_external_link_primary(url: String) -> void:
	Global.open_url(url)
	close_current_modal()


func _on_scene_timeout_primary() -> void:
	Global.metrics.track_click_button("reload", "LOADING", "")
	Global.realm.async_set_realm(Global.realm.get_realm_string())
	close_current_modal()


func _on_scene_timeout_secondary() -> void:
	Global.metrics.track_click_button("run_anyway", "LOADING", "")
	Global.run_anyway.emit()
	close_current_modal()


func _on_connection_lost_primary() -> void:
	connection_lost_retry.emit()
	close_current_modal()


func _on_connection_lost_secondary() -> void:
	connection_lost_exit.emit()
	close_current_modal()


func _on_teleport_primary(location: Vector2i, realm: String) -> void:
	Global.teleport_to(location, realm)
	close_current_modal()


func _on_change_realm_primary(realm_name: String) -> void:
	# Default behavior: call Global.realm.async_set_realm
	Global.realm.async_set_realm(realm_name)
	close_current_modal()


func _on_modal_tree_exited() -> void:
	# Modal was removed from tree, clear reference
	if current_modal:
		current_modal = null


func _remove_modal() -> void:
	if current_modal:
		current_modal.queue_free()
		current_modal = null
