class_name ModalManager
extends Node

## Global manager to show modals from anywhere in the application.
## Does not require the modal to be previously in any scene.
## Contains all business logic for different modal types.

signal connection_lost_retry
signal connection_lost_exit

const MODAL_SCENE_PATH = "res://src/ui/components/modal/modal.tscn"
const TRAVEL_MODAL_SCENE_PATH = "res://src/ui/components/modal/travel_modal.tscn"

# Modal text constants
const EXTERNAL_LINK_TITLE = "Open external link?"
const EXTERNAL_LINK_BODY = "You're about to visit an external website. Make sure you trust this site before continuing."
const EXTERNAL_LINK_PRIMARY = "OPEN LINK"
const EXTERNAL_LINK_SECONDARY = "CANCEL"

const SCENE_TIMEOUT_TITLE = "Loading  is taking longer than expected"
const SCENE_TIMEOUT_BODY = "You can reload the experience, or jump in now, it should keep loading in the background."
const SCENE_TIMEOUT_PRIMARY = "RELOAD"
const SCENE_TIMEOUT_SECONDARY = "START ANYWAY"

const CONNECTION_LOST_TITLE = "Connection lost"
const CONNECTION_LOST_BODY = "Please check your internet connection and try again."
const CONNECTION_LOST_PRIMARY = "RETRY"
const CONNECTION_LOST_SECONDARY = "EXIT"

const SCENE_CRASH_TITLE = "Scene error"
const SCENE_CRASH_BODY = "This scene stopped working. Please reload or go back to discover."
const SCENE_CRASH_PRIMARY = "RELOAD"
const SCENE_CRASH_SECONDARY = "BACK"

const BAN_PRE_CHECK_TITLE = "You can't enter"
const BAN_PRE_CHECK_BODY = "You're banned from this scene.\nPlease contact support for more information."
const BAN_PRE_CHECK_PRIMARY = "BACK TO DISCOVER"

const BAN_KICKED_TITLE = "You've been banned"
const BAN_KICKED_BODY = "Please contact support for more information."
const BAN_KICKED_PRIMARY = "BACK TO DISCOVER"

const LOW_SPEC_IPHONE_TITLE = "Limited performance"
const LOW_SPEC_IPHONE_BODY = "Your device is below our recommended specs (iPhone 13/SE 2023). You may notice slowdowns, crashes or heating issues while playing."
const LOW_SPEC_IPHONE_PRIMARY = "OK"

var current_modal: Modal = null
var current_travel_modal: TravelModal = null
var modal_scene: PackedScene = null
var travel_modal_scene: PackedScene = null
var ban_pre_check_active: bool = false
## Suppresses a stale ban_kicked_modal triggered by comms after a pre-check was already handled.
var _suppress_ban_kicked: bool = false
var _canvas_layer: CanvasLayer = null
var _travel_canvas_layer: CanvasLayer = null


func _ready() -> void:
	modal_scene = load(MODAL_SCENE_PATH)
	if not modal_scene:
		push_error("ModalManager: Could not load modal scene at: " + MODAL_SCENE_PATH)
	travel_modal_scene = load(TRAVEL_MODAL_SCENE_PATH)
	if not travel_modal_scene:
		push_error("ModalManager: Could not load travel modal scene at: " + TRAVEL_MODAL_SCENE_PATH)
	Global.on_menu_close.connect(_on_menu_close_ban_recheck)
	Global.loading_finished.connect(_on_loading_finished_clear_suppress)


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
## @param hide_buttons: If true, hides all buttons (used on iOS after retry fails)
func async_show_connection_lost_modal(hide_buttons: bool = false) -> void:
	if not current_modal:
		if not await _async_create_modal():
			print("NOT CREATED MODAL")
			return

	current_modal.blocker = true
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
	if OS.get_name() == "iOS":
		current_modal.button_secondary.hide()
		if hide_buttons:
			# No buttons at all — modal auto-closes only when connection restores
			current_modal.buttons_container.hide()
			current_modal.buttons_separator.hide()
			current_modal.set_body(CONNECTION_LOST_BODY + "\n \n Try restarting the app.")
	else:
		current_modal.button_secondary.pressed.connect(_on_connection_lost_secondary)


## Shows a TELEPORT type modal using the new TravelModal layout
## @param location: The position to teleport to
## @param realm: The destination realm (optional)
func async_show_teleport_modal(location: Vector2i, realm: String = "") -> void:
	var destination_realm = realm if not realm.is_empty() else DclUrls.main_realm()

	if not await _async_create_travel_modal():
		return

	current_travel_modal.closed.connect(close_travel_modal)
	current_travel_modal.jump_in_pressed.connect(
		_on_teleport_primary.bind(location, destination_realm)
	)

	current_travel_modal.show()

	await get_tree().process_frame
	await get_tree().process_frame

	# Load place data asynchronously and update modal
	await _async_load_travel_modal_data(location, destination_realm)


## Shows a WORLD travel modal (for .dcl.eth worlds)
## Validates the world exists before showing the modal.
## @param world_name: The world name (e.g. "something.dcl.eth")
func async_show_world_modal(world_name: String) -> void:
	# Validate world exists before creating the modal
	var result = await PlacesHelper.async_get_by_names(world_name)

	if result is PromiseError:
		printerr("World not found or error: ", world_name, " ", result.get_error())
		NotificationsManager.show_system_toast(
			"World not found", world_name + " could not be reached.", "error", "alert"
		)
		return

	var json: Dictionary = result.get_string_response_as_json()
	if not json.has("data") or json.data.is_empty():
		printerr("World does not exist: ", world_name)
		NotificationsManager.show_system_toast(
			"World not found", world_name + " does not exist.", "error", "alert"
		)
		return

	var world_data: Dictionary = json.data[0]

	if not await _async_create_travel_modal():
		return

	current_travel_modal.closed.connect(close_travel_modal)
	current_travel_modal.jump_in_pressed.connect(_on_world_jump_in.bind(world_name))

	var title = str(world_data.get("title", world_name))
	current_travel_modal.set_place_name(title if not title.is_empty() else world_name)

	var creator = world_data.get("contact_name", "")
	current_travel_modal.set_creator("" if creator == null else str(creator))

	current_travel_modal.show()

	# Fire-and-forget: image is non-critical, modal is already usable without it
	var image_url = world_data.get("image", "")
	if image_url != null and not str(image_url).is_empty():
		_async_load_travel_modal_image(str(image_url))


## Shows a CHANGE_REALM type modal using TravelModal
## @param realm_name: The destination realm name
## @param message: Optional message from the scene
func async_show_change_realm_modal(realm_name: String, _message: String = "") -> void:
	if not await _async_create_travel_modal():
		return

	current_travel_modal.closed.connect(close_travel_modal)
	current_travel_modal.jump_in_pressed.connect(_on_change_realm_primary.bind(realm_name))
	current_travel_modal.show()

	await get_tree().process_frame
	await get_tree().process_frame

	# Try to load realm data from Places API
	await _async_load_change_realm_data(realm_name)


## Shows a SCENE_CRASH type modal
## @param entity_id: The entity ID of the crashed scene
func async_show_scene_crash_modal(entity_id: String) -> void:
	if not current_modal:
		if not await _async_create_modal():
			return

	current_modal.blocker = true
	current_modal.set_title(SCENE_CRASH_TITLE)
	current_modal.set_body(SCENE_CRASH_BODY)
	current_modal.set_primary_button_text(SCENE_CRASH_PRIMARY)
	current_modal.set_secondary_button_text(SCENE_CRASH_SECONDARY)
	current_modal.show_icon(Modal.MODAL_ALERT_ICON)
	current_modal.hide_url()
	current_modal.show()

	# Disconnect previous connections and connect button actions
	_disconnect_button_signals()
	current_modal.button_primary.pressed.connect(_on_scene_crash_reload.bind(entity_id))
	current_modal.button_secondary.pressed.connect(_on_scene_crash_back)


## Shows a ban pre-check modal (when trying to enter a scene the user is banned from)
func async_show_ban_pre_check_modal() -> void:
	_force_hide_loading_screen()

	if not current_modal:
		if not await _async_create_modal():
			return

	current_modal.blocker = true
	current_modal.set_title(BAN_PRE_CHECK_TITLE)
	current_modal.set_body(BAN_PRE_CHECK_BODY)
	current_modal.set_primary_button_text(BAN_PRE_CHECK_PRIMARY)
	current_modal.show_icon(Modal.MODAL_BAN_ICON)
	current_modal.hide_url()
	current_modal.button_secondary.hide()
	current_modal.show()

	_disconnect_button_signals()
	current_modal.button_primary.pressed.connect(_on_ban_pre_check_go_to_discover)


## Shows a ban kicked modal (when kicked from a scene in real-time)
func async_show_ban_kicked_modal() -> void:
	# A pre-check already handled this ban — ignore the stale comms disconnect
	if _suppress_ban_kicked:
		_suppress_ban_kicked = false
		return
	if not current_modal:
		if not await _async_create_modal():
			return

	current_modal.blocker = true
	current_modal.set_title(BAN_KICKED_TITLE)
	current_modal.set_body(BAN_KICKED_BODY)
	current_modal.set_primary_button_text(BAN_KICKED_PRIMARY)
	current_modal.show_icon(Modal.MODAL_BAN_ICON)
	current_modal.hide_url()
	current_modal.button_secondary.hide()
	current_modal.show()

	_disconnect_button_signals()
	current_modal.button_primary.pressed.connect(_on_ban_go_to_discover)


## Shows a low-spec iPhone warning modal (lobby popup)
func async_show_low_spec_iphone_modal() -> void:
	if not current_modal:
		if not await _async_create_modal():
			return

	current_modal.set_title(LOW_SPEC_IPHONE_TITLE)
	current_modal.set_body(LOW_SPEC_IPHONE_BODY)
	current_modal.set_primary_button_text(LOW_SPEC_IPHONE_PRIMARY)
	current_modal.set_primary_button_font_size(24)
	current_modal.show_icon(Modal.MODAL_ALERT_ICON)
	current_modal.hide_url()
	current_modal.button_secondary.hide()
	current_modal.blocker = true
	current_modal.show()

	_disconnect_button_signals()
	current_modal.button_primary.pressed.connect(close_current_modal)


## Clears the suppress flag so the next ban_kicked_modal call is not silenced.
func clear_suppress_ban_kicked() -> void:
	_suppress_ban_kicked = false


## Closes the current travel modal if it exists
func close_travel_modal() -> void:
	if current_travel_modal:
		current_travel_modal.hide()
		_remove_travel_modal()


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

	# Wrap in a CanvasLayer with high layer to ensure modals render above all overlays
	if _canvas_layer and is_instance_valid(_canvas_layer):
		_canvas_layer.queue_free()

	_canvas_layer = CanvasLayer.new()
	_canvas_layer.layer = 100

	var root = get_tree().root
	if not root:
		push_error("ModalManager: Could not get scene tree root")
		return null

	root.add_child(_canvas_layer)
	_canvas_layer.add_child(modal)
	current_modal = modal

	# Connect signal to clean up when modal exits the tree
	current_modal.tree_exited.connect(_on_modal_tree_exited)

	current_modal.hide_url()
	current_modal.hide_icon()
	current_modal.blocker = false

	# Wait for modal to be fully in tree and @onready nodes initialized
	# This is especially important when called from SDK/Rust
	await get_tree().process_frame
	await get_tree().process_frame

	return modal


func _async_create_travel_modal() -> TravelModal:
	if current_travel_modal:
		close_travel_modal()

	if not travel_modal_scene:
		push_error("ModalManager: Travel modal scene is not loaded")
		return null

	var modal = travel_modal_scene.instantiate() as TravelModal
	if not modal:
		push_error("ModalManager: Could not instantiate travel modal")
		return null

	if _travel_canvas_layer and is_instance_valid(_travel_canvas_layer):
		_travel_canvas_layer.queue_free()

	_travel_canvas_layer = CanvasLayer.new()
	_travel_canvas_layer.layer = 100

	var root = get_tree().root
	if not root:
		push_error("ModalManager: Could not get scene tree root")
		return null

	root.add_child(_travel_canvas_layer)
	_travel_canvas_layer.add_child(modal)
	current_travel_modal = modal

	current_travel_modal.tree_exited.connect(_on_travel_modal_tree_exited)

	await get_tree().process_frame
	await get_tree().process_frame

	return modal


func _on_world_jump_in(world_name: String) -> void:
	Global.async_teleport_to(Vector2i.ZERO, world_name)
	close_travel_modal()


func _async_load_travel_modal_data(location: Vector2i, _realm: String) -> void:
	if not is_instance_valid(current_travel_modal):
		return

	var result = await PlacesHelper.async_get_by_position(location)

	if result is PromiseError:
		printerr("Error requesting place data for travel modal", result.get_error())
		return

	if not is_instance_valid(current_travel_modal):
		return

	var json: Dictionary = result.get_string_response_as_json()

	if not json.has("data") or json.data.is_empty():
		return

	var place_data: Dictionary = json.data[0]

	var title = str(place_data.get("title", ""))
	if not title.is_empty() and title != "interactive-text":
		current_travel_modal.set_place_name(title)

	var creator = place_data.get("contact_name", "")
	current_travel_modal.set_creator("" if creator == null else str(creator))

	var image_url = place_data.get("image", "")
	if image_url != null and not str(image_url).is_empty():
		_async_load_travel_modal_image(str(image_url))


func _async_load_change_realm_data(realm_name: String) -> void:
	if not is_instance_valid(current_travel_modal):
		return

	# Try to fetch world/realm data from Places API
	var result = await PlacesHelper.async_get_by_names(realm_name)

	if not is_instance_valid(current_travel_modal):
		return

	if result is PromiseError:
		# API error — show realm name as fallback
		current_travel_modal.set_place_name(realm_name)
		return

	var json: Dictionary = result.get_string_response_as_json()

	if not json.has("data") or json.data.is_empty():
		# No data found — show realm name as fallback
		current_travel_modal.set_place_name(realm_name)
		return

	var realm_data: Dictionary = json.data[0]

	var title = str(realm_data.get("title", realm_name))
	current_travel_modal.set_place_name(title if not title.is_empty() else realm_name)

	var creator = realm_data.get("contact_name", "")
	current_travel_modal.set_creator("" if creator == null else str(creator))

	var image_url = realm_data.get("image", "")
	if image_url != null and not str(image_url).is_empty():
		_async_load_travel_modal_image(str(image_url))


func _async_load_travel_modal_image(url: String) -> void:
	var url_hash = url.md5_text()
	var promise = Global.content_provider.fetch_texture_by_url(url_hash, url)
	var result = await PromiseUtils.async_awaiter(promise)

	if result is PromiseError:
		printerr("ModalManager: Error downloading travel modal image: ", result.get_error())
		return

	if is_instance_valid(current_travel_modal):
		current_travel_modal.set_image(result.texture)


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
	# Emit loading_timeout so loading_screen_progress_logic hides the loading screen and shows the scene
	Global.scene_runner.loading_timeout.emit(-1)
	close_current_modal()


func _on_connection_lost_primary() -> void:
	connection_lost_retry.emit()
	close_current_modal()


func _on_connection_lost_secondary() -> void:
	# Intentionally does NOT close the modal: the listener (e.g. CQM) handles
	# get_tree().quit() and we want the modal to stay visible until the app is gone,
	# otherwise the user briefly sees the broken UI underneath before exit.
	connection_lost_exit.emit()


func _on_teleport_primary(location: Vector2i, realm: String) -> void:
	Global.async_teleport_to(location, realm)
	close_travel_modal()


func _on_change_realm_primary(realm_name: String) -> void:
	Global.realm.async_set_realm(realm_name)
	close_travel_modal()


func _on_scene_crash_reload(_entity_id: String) -> void:
	Global.realm.async_set_realm(Global.realm.get_realm_string())
	close_current_modal()


func _on_scene_crash_back() -> void:
	Global.open_discover.emit()
	close_current_modal()


func _on_ban_pre_check_go_to_discover() -> void:
	close_current_modal()
	_suppress_ban_kicked = true

	if (
		Global.realm.get_realm_string().is_empty()
		and is_instance_valid(Global.get_explorer())
		and not Global.is_orientation_portrait()
	):
		# Case 1: Cold start deep link, landscape — explorer loaded but no realm, open discover
		_force_hide_loading_screen()
		Global.set_orientation_portrait()
		Global.open_discover.emit()
		# Activate the loop AFTER opening discover, so any on_menu_close signals
		# fired during the transition don't trigger a premature reshow.
		ban_pre_check_active = true
	elif not Global.is_orientation_portrait():
		# Case 2: In-game command (/world, /goto) — open discover
		_force_hide_loading_screen()
		Global.set_orientation_portrait()
		Global.open_discover.emit()
		ban_pre_check_active = true
	# Case 3: Already in discover — modal closed, discover is already behind


func _on_ban_go_to_discover() -> void:
	close_current_modal()
	Global.set_orientation_portrait()
	Global.open_discover.emit()


func _on_modal_tree_exited() -> void:
	# Modal was removed from tree, clear reference
	if current_modal:
		current_modal = null


func _on_travel_modal_tree_exited() -> void:
	if current_travel_modal:
		current_travel_modal = null


## Instantly kills the loading screen and runs the normal post-loading cleanup
## (release comms, restore audio, close navbar, emit loading_finished, etc.).
func _force_hide_loading_screen() -> void:
	var explorer = Global.get_explorer()
	if not is_instance_valid(explorer) or not explorer.loading_ui.visible:
		return
	if not is_instance_valid(explorer.loading_ui.loading_screen_progress_logic):
		return
	# Hide the Control instantly so the tween in async_hide_loading_screen_effect
	# has nothing visible to fade — avoids the alpha bleed-through.
	explorer.loading_ui.hide()
	# Run the normal post-loading path (release comms, restore audio, close navbar, etc.)
	explorer.loading_ui.loading_screen_progress_logic.hide_loading_screen()


func _on_menu_close_ban_recheck() -> void:
	if not ban_pre_check_active:
		return
	# Re-show the ban modal and re-open discover
	async_show_ban_pre_check_modal.call_deferred()


## Clear suppress flag after loading finishes.
func _on_loading_finished_clear_suppress() -> void:
	_suppress_ban_kicked = false


func _remove_modal() -> void:
	if current_modal:
		current_modal.queue_free()
		current_modal = null
	if _canvas_layer and is_instance_valid(_canvas_layer):
		_canvas_layer.queue_free()
		_canvas_layer = null


func _remove_travel_modal() -> void:
	if current_travel_modal:
		current_travel_modal.queue_free()
		current_travel_modal = null
	if _travel_canvas_layer and is_instance_valid(_travel_canvas_layer):
		_travel_canvas_layer.queue_free()
		_travel_canvas_layer = null
