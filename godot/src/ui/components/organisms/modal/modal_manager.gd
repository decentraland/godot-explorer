class_name ModalManager
extends Node

## Global manager to show modals from anywhere in the application.
## Does not require the modal to be previously in any scene.
## Contains all business logic for different modal types.

signal connection_lost_retry
signal connection_lost_exit
signal iap_terms_accepted
signal session_ended_sign_in
signal session_ended_retry
signal session_ended_exit

const MODAL_SCENE_PATH = "res://src/ui/components/organisms/modal/modal.tscn"
const TRAVEL_MODAL_SCENE_PATH = "res://src/ui/components/organisms/modal/travel_modal.tscn"
const INPUT_MODAL_SCENE_PATH = "res://src/ui/components/organisms/input_modal/input_modal.tscn"
const CODE_MODAL_SCENE_PATH = "res://src/ui/components/organisms/code_modal/code_modal.tscn"
const REWARD_MODAL_SCENE_PATH = "res://src/ui/components/organisms/reward_modal/reward_modal.tscn"
const UPGRADE_MODAL_SCENE_PATH = "res://src/ui/components/organisms/upgrade_modal/upgrade_modal.tscn"

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

const LOW_MEMORY_TITLE = "Low memory"
const LOW_MEMORY_BODY = "This place may not run smoothly on your device and could close unexpectedly."
const LOW_MEMORY_PRIMARY = "CONTINUE"
const LOW_MEMORY_SECONDARY = "BACK TO DISCOVER"

const BAN_PRE_CHECK_TITLE = "You can't enter"
const BAN_PRE_CHECK_BODY = "You're banned from this scene.\nPlease contact support for more information."
const BAN_PRE_CHECK_PRIMARY = "BACK TO DISCOVER"

const PRIVATE_WORLD_TITLE = "%s is private"
const PRIVATE_WORLD_BODY = "Only invited people can enter."
const PRIVATE_WORLD_PRIMARY = "OK"

const BAN_KICKED_TITLE = "You've been banned"
const BAN_KICKED_BODY = "Please contact support for more information."
const BAN_KICKED_PRIMARY = "BACK TO DISCOVER"

const LOW_SPEC_IPHONE_TITLE = "Limited performance"
const LOW_SPEC_IPHONE_BODY = "Your device is below our recommended specs (iPhone 13/SE 2023). You may notice slowdowns, crashes or heating issues while playing."
const LOW_SPEC_IPHONE_PRIMARY = "OK"

const PURCHASE_FAILED_TITLE = "Something\nwent wrong"
const PURCHASE_FAILED_BODY = "Your purchase could not be completed"
const PURCHASE_FAILED_PRIMARY = "OK"

const CREDIT_LIMIT_TITLE = "Limit reached"
const CREDIT_LIMIT_TOTAL_BODY = "You can't buy more credits because you've reached maximum holding limit. Spend your credits to buy more."
const CREDIT_LIMIT_DAILY_BODY = "You've reached the maximum amount of credits you can buy today."
const CREDIT_LIMIT_PRIMARY = "OK"

const PURCHASE_IN_FLIGHT_TITLE = "Purchase in progress"
const PURCHASE_IN_FLIGHT_BODY = "A purchase is already being processed. Please wait for it to complete before starting a new one."
const PURCHASE_IN_FLIGHT_PRIMARY = "OK"

# Shown at quote time when the server hit its global daily credit ceiling — nothing
# was charged, the user should try again later.
const PURCHASE_UNAVAILABLE_TITLE = "Temporarily\nunavailable"
const PURCHASE_UNAVAILABLE_BODY = "Credit purchases are temporarily unavailable. Please try again later."
const PURCHASE_UNAVAILABLE_PRIMARY = "OK"

# Shown after a successful charge whose credits are still being applied (the server
# was at its daily ceiling; the Apple webhook will credit shortly).
const PURCHASE_PROCESSING_TITLE = "Almost there"
const PURCHASE_PROCESSING_BODY = "Your purchase went through and your credits will be added shortly."
const PURCHASE_PROCESSING_PRIMARY = "OK"

const IAP_TERMS_TITLE = "Terms of Use"
const IAP_TERMS_CHECKBOX_BBCODE = (
	'I have read and accept Decentraland\'s [color=#E8B9FF][url="https://decentraland.org/terms"]Terms of Use[/url][/color], '
	+ '[color=#E8B9FF][url="https://decentraland.org/privacy"]Privacy Policy[/url][/color], '
	+ '[color=#E8B9FF][url="https://decentraland.org/content"]Content Policy[/url][/color] and '
	+ '[color=#E8B9FF][url="https://decentraland.org/credits-terms"]Credits Terms of Use[/url][/color].'
)
const IAP_TERMS_PRIMARY = "CONFIRM"
const IAP_TERMS_SECONDARY = "CANCEL"

const SESSION_ENDED_DUPLICATE_TITLE = "Session Ended"
const SESSION_ENDED_DUPLICATE_BODY = "Your session was ended because your account logged in from another location."
const SESSION_ENDED_ROOM_CLOSED_TITLE = "Room Closed"
const SESSION_ENDED_ROOM_CLOSED_BODY = "The room you were in has been closed."
const SESSION_ENDED_OTHER_TITLE = "Disconnected"
const SESSION_ENDED_OTHER_BODY = "You have been disconnected from the server. Please try again later."
const SESSION_ENDED_SIGN_IN_PRIMARY = "SIGN IN"
const SESSION_ENDED_RETRY_PRIMARY = "RECONNECT"
const SESSION_ENDED_SECONDARY = "EXIT"

var current_modal: Modal = null
var current_travel_modal: TravelModal = null
var current_input_modal: InputModal = null
var current_code_modal: CodeModal = null
var current_reward_modal: RewardModal = null
var current_upgrade_modal: UpgradeModal = null
var modal_scene: PackedScene = null
var travel_modal_scene: PackedScene = null
var input_modal_scene: PackedScene = null
var code_modal_scene: PackedScene = null
var reward_modal_scene: PackedScene = null
var upgrade_modal_scene: PackedScene = null
var ban_pre_check_active: bool = false
## Suppresses a stale ban_kicked_modal triggered by comms after a pre-check was already handled.
var _suppress_ban_kicked: bool = false
var _canvas_layer: CanvasLayer = null
var _travel_canvas_layer: CanvasLayer = null
var _input_canvas_layer: CanvasLayer = null
var _code_canvas_layer: CanvasLayer = null
var _reward_canvas_layer: CanvasLayer = null
var _upgrade_canvas_layer: CanvasLayer = null

static var _add_email_regex: RegEx = RegEx.create_from_string("^[^@\\s]+@[^@\\s]+\\.[^@\\s]+$")


func _ready() -> void:
	modal_scene = load(MODAL_SCENE_PATH)
	if not modal_scene:
		push_error("ModalManager: Could not load modal scene at: " + MODAL_SCENE_PATH)
	travel_modal_scene = load(TRAVEL_MODAL_SCENE_PATH)
	if not travel_modal_scene:
		push_error("ModalManager: Could not load travel modal scene at: " + TRAVEL_MODAL_SCENE_PATH)
	input_modal_scene = load(INPUT_MODAL_SCENE_PATH)
	if not input_modal_scene:
		push_error("ModalManager: Could not load input modal scene at: " + INPUT_MODAL_SCENE_PATH)
	code_modal_scene = load(CODE_MODAL_SCENE_PATH)
	if not code_modal_scene:
		push_error("ModalManager: Could not load code modal scene at: " + CODE_MODAL_SCENE_PATH)
	reward_modal_scene = load(REWARD_MODAL_SCENE_PATH)
	if not reward_modal_scene:
		push_error("ModalManager: Could not load reward modal scene at: " + REWARD_MODAL_SCENE_PATH)
	upgrade_modal_scene = load(UPGRADE_MODAL_SCENE_PATH)
	if not upgrade_modal_scene:
		push_error(
			"ModalManager: Could not load upgrade modal scene at: " + UPGRADE_MODAL_SCENE_PATH
		)
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


## Shows a LOW_MEMORY warning modal (issue #2002): the memory monitor detected
## critical pressure with nothing left to evict but the current scene. Rather
## than kill it, let the user choose. Reuses the crash modal design (alert icon).
## - "CONTINUE ANYWAY": keep the scene; enable verbose memory logging so we can
##   observe how far it gets before a possible OOM; stop nagging this session.
## - "BACK TO DISCOVER": leave the scene.
## @param _entity_id: The entity ID of the current scene (unused for now)
func async_show_low_memory_warning_modal(
	entity_id: String, footprint_mb: int = -1, available_mb: int = -1
) -> void:
	if not current_modal:
		if not await _async_create_modal():
			return

	current_modal.blocker = true
	current_modal.set_title(LOW_MEMORY_TITLE)
	current_modal.set_body(LOW_MEMORY_BODY)
	current_modal.set_primary_button_text(LOW_MEMORY_PRIMARY)
	current_modal.set_secondary_button_text(LOW_MEMORY_SECONDARY)
	current_modal.show_icon(Modal.MODAL_ALERT_ICON)
	current_modal.hide_url()
	current_modal.show()

	_disconnect_button_signals()
	current_modal.button_primary.pressed.connect(_on_low_memory_continue)
	current_modal.button_secondary.pressed.connect(_on_low_memory_back)

	# Measure how often the low-memory warning surfaces, on which scene/device and at
	# what memory level (issue #2002). Flush eagerly: the app may be jetsam-killed by the
	# OS shortly after this warning, so we can't rely on the periodic flush to send it.
	if Global.metrics != null:
		Global.metrics.track_screen_viewed(
			"LOW_MEMORY_WARNING",
			JSON.stringify(
				{
					"entity_id": entity_id,
					"footprint_mb": footprint_mb,
					"available_mb": available_mb,
					"platform": OS.get_name()
				}
			)
		)
		Global.metrics.flush.call_deferred()


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


## Shows the private-world modal (#1725): the target world restricts access to an
## allow-list the user is not on, so the realm change was refused before loading.
## @param world_name: The world being refused, e.g. "myworld.dcl.eth"
func async_show_private_world_modal(world_name: String) -> void:
	_force_hide_loading_screen()

	if not current_modal:
		if not await _async_create_modal():
			return

	current_modal.blocker = true
	current_modal.set_title(PRIVATE_WORLD_TITLE % world_name.trim_suffix(".dcl.eth"))
	current_modal.set_body(PRIVATE_WORLD_BODY)
	current_modal.set_primary_button_text(PRIVATE_WORLD_PRIMARY)
	current_modal.show_icon(Modal.MODAL_BLOCK_ICON)
	current_modal.hide_url()
	current_modal.button_secondary.hide()
	current_modal.show()

	_disconnect_button_signals()
	current_modal.button_primary.pressed.connect(close_current_modal)


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


## Shows the modal for a DuplicateIdentity disconnect (another session signed in with same account).
## Primary action signs the user out and returns them to the sign-in screen.
func async_show_session_ended_modal() -> void:
	await _async_show_disconnect_modal(
		SESSION_ENDED_DUPLICATE_TITLE,
		SESSION_ENDED_DUPLICATE_BODY,
		SESSION_ENDED_SIGN_IN_PRIMARY,
		_on_session_ended_sign_in
	)


## Shows the modal for a RoomClosed disconnect. Primary action retries the connection.
func async_show_room_closed_modal() -> void:
	await _async_show_disconnect_modal(
		SESSION_ENDED_ROOM_CLOSED_TITLE,
		SESSION_ENDED_ROOM_CLOSED_BODY,
		SESSION_ENDED_RETRY_PRIMARY,
		_on_session_ended_retry
	)


## Shows the modal for a generic/other disconnect (after reconnect attempts are exhausted).
## Primary action retries the connection.
func async_show_disconnected_modal() -> void:
	await _async_show_disconnect_modal(
		SESSION_ENDED_OTHER_TITLE,
		SESSION_ENDED_OTHER_BODY,
		SESSION_ENDED_RETRY_PRIMARY,
		_on_session_ended_retry
	)


func _async_show_disconnect_modal(
	title: String, body: String, primary_label: String, primary_handler: Callable
) -> void:
	if not current_modal:
		if not await _async_create_modal():
			return

	current_modal.blocker = true
	current_modal.set_title(title)
	current_modal.set_body(body)
	current_modal.set_primary_button_text(primary_label)
	current_modal.set_secondary_button_text(SESSION_ENDED_SECONDARY)
	current_modal.show_icon(Modal.MODAL_CONNECTION_ICON)
	current_modal.hide_url()
	current_modal.show()

	_disconnect_button_signals()
	current_modal.button_primary.pressed.connect(primary_handler)
	current_modal.button_secondary.pressed.connect(_on_session_ended_secondary)


func _on_session_ended_sign_in() -> void:
	session_ended_sign_in.emit()
	close_current_modal()


func _on_session_ended_retry() -> void:
	session_ended_retry.emit()
	close_current_modal()


func _on_session_ended_secondary() -> void:
	session_ended_exit.emit()
	close_current_modal()


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


## Shows a purchase failed modal
func async_show_purchase_failed_modal() -> void:
	if not current_modal:
		if not await _async_create_modal():
			return

	current_modal.set_title(PURCHASE_FAILED_TITLE)
	current_modal.set_body(PURCHASE_FAILED_BODY)
	current_modal.set_primary_button_text(PURCHASE_FAILED_PRIMARY)
	current_modal.show_icon(Modal.MODAL_ALERT_ICON)
	current_modal.hide_url()
	current_modal.button_secondary.hide()
	current_modal.show()

	_disconnect_button_signals()
	current_modal.button_primary.pressed.connect(close_current_modal)


## Shows a total credit limit reached modal
func async_show_credit_limit_total_modal() -> void:
	if not current_modal:
		if not await _async_create_modal():
			return

	current_modal.set_title(CREDIT_LIMIT_TITLE)
	current_modal.set_body(CREDIT_LIMIT_TOTAL_BODY)
	current_modal.set_primary_button_text(CREDIT_LIMIT_PRIMARY)
	current_modal.show_icon(Modal.MODAL_ALERT_ICON)
	current_modal.hide_url()
	current_modal.button_secondary.hide()
	current_modal.show()

	_disconnect_button_signals()
	current_modal.button_primary.pressed.connect(close_current_modal)


## Shows a daily credit limit reached modal
func async_show_credit_limit_daily_modal() -> void:
	if not current_modal:
		if not await _async_create_modal():
			return

	current_modal.set_title(CREDIT_LIMIT_TITLE)
	current_modal.set_body(CREDIT_LIMIT_DAILY_BODY)
	current_modal.set_primary_button_text(CREDIT_LIMIT_PRIMARY)
	current_modal.show_icon(Modal.MODAL_ALERT_ICON)
	current_modal.hide_url()
	current_modal.button_secondary.hide()
	current_modal.show()

	_disconnect_button_signals()
	current_modal.button_primary.pressed.connect(close_current_modal)


## Shows a purchase-in-flight modal when the user tries to buy while another purchase is pending
func async_show_purchase_in_flight_modal() -> void:
	if not current_modal:
		if not await _async_create_modal():
			return

	current_modal.set_title(PURCHASE_IN_FLIGHT_TITLE)
	current_modal.set_body(PURCHASE_IN_FLIGHT_BODY)
	current_modal.set_primary_button_text(PURCHASE_IN_FLIGHT_PRIMARY)
	current_modal.show_icon(Modal.MODAL_ALERT_ICON)
	current_modal.hide_url()
	current_modal.button_secondary.hide()
	current_modal.show()

	_disconnect_button_signals()
	current_modal.button_primary.pressed.connect(close_current_modal)


## Shows a modal when credit purchases are temporarily unavailable (server daily cap)
func async_show_purchase_unavailable_modal() -> void:
	if not current_modal:
		if not await _async_create_modal():
			return

	current_modal.set_title(PURCHASE_UNAVAILABLE_TITLE)
	current_modal.set_body(PURCHASE_UNAVAILABLE_BODY)
	current_modal.set_primary_button_text(PURCHASE_UNAVAILABLE_PRIMARY)
	current_modal.show_icon(Modal.MODAL_ALERT_ICON)
	current_modal.hide_url()
	current_modal.button_secondary.hide()
	current_modal.show()

	_disconnect_button_signals()
	current_modal.button_primary.pressed.connect(close_current_modal)


## Shows a modal when a purchase succeeded but its credits are still being applied
func async_show_purchase_processing_modal() -> void:
	if not current_modal:
		if not await _async_create_modal():
			return

	current_modal.set_title(PURCHASE_PROCESSING_TITLE)
	current_modal.set_body(PURCHASE_PROCESSING_BODY)
	current_modal.set_primary_button_text(PURCHASE_PROCESSING_PRIMARY)
	current_modal.show_icon(Modal.MODAL_ALERT_ICON)
	current_modal.hide_url()
	current_modal.button_secondary.hide()
	current_modal.show()

	_disconnect_button_signals()
	current_modal.button_primary.pressed.connect(close_current_modal)


## Shows IAP terms of use modal with a checkbox that must be accepted before confirming
func async_show_iap_terms_modal() -> void:
	if not current_modal:
		if not await _async_create_modal():
			return

	current_modal.set_title(IAP_TERMS_TITLE)
	current_modal.set_body("")
	current_modal.set_primary_button_text(IAP_TERMS_PRIMARY)
	current_modal.set_secondary_button_text(IAP_TERMS_SECONDARY)
	current_modal.hide_icon()
	current_modal.hide_url()
	current_modal.blocker = true
	current_modal.button_primary.disabled = true

	current_modal.show_checkbox(IAP_TERMS_CHECKBOX_BBCODE)
	current_modal.checkbox.toggled.connect(_on_iap_terms_checkbox_toggled)
	current_modal.checkbox_text.meta_clicked.connect(_on_iap_terms_link_clicked)

	current_modal.show()

	_disconnect_button_signals()
	current_modal.button_primary.pressed.connect(_on_iap_terms_confirmed)
	current_modal.button_secondary.pressed.connect(_on_iap_terms_cancelled)


func _on_iap_terms_checkbox_toggled(checked: bool) -> void:
	if current_modal:
		current_modal.button_primary.disabled = not checked


func _on_iap_terms_confirmed() -> void:
	Iap.accept_terms()
	close_current_modal()
	iap_terms_accepted.emit()


func _on_iap_terms_cancelled() -> void:
	close_current_modal()


func _on_iap_terms_link_clicked(meta) -> void:
	Global.open_url(str(meta))


## Clears the suppress flag so the next ban_kicked_modal call is not silenced.
func clear_suppress_ban_kicked() -> void:
	_suppress_ban_kicked = false


## Shows a generic input modal. Returns the InputModal instance so callers
## can connect to its confirmed/cancelled signals.
func async_show_input_modal(
	title: String,
	subtitle: String,
	placeholder: String,
	confirm_text: String,
	cancel_text: String,
	validation: Callable,
) -> InputModal:
	var modal = await _async_create_input_modal()
	if not modal:
		return null
	modal.setup(title, subtitle, placeholder, confirm_text, cancel_text, validation)
	modal.open()
	return modal


## Closes the current input modal if it exists
func close_input_modal() -> void:
	if current_input_modal:
		current_input_modal.close()
		_remove_input_modal()


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


## When any modal opens, tear down the chat's text-input ("write") mode so the
## on-screen keyboard is dismissed and focus is handed back. Otherwise a modal
## (e.g. the crash blocker) can appear over an active chat, leaving the mobile
## virtual keyboard stuck on screen with no way to dismiss it — a dead end.
## Idempotent and safe to call unconditionally before every modal. See #2427.
func _dismiss_chat_input_for_modal() -> void:
	var explorer = Global.get_explorer()
	if not is_instance_valid(explorer):
		return
	var chat_panel = explorer.chat_panel
	if is_instance_valid(chat_panel) and is_instance_valid(chat_panel.chat):
		chat_panel.chat.close_write_mode_if_active()


func _async_create_modal() -> Modal:
	_dismiss_chat_input_for_modal()
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
		_canvas_layer.get_parent().remove_child(_canvas_layer)
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
	_dismiss_chat_input_for_modal()
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
		_travel_canvas_layer.get_parent().remove_child(_travel_canvas_layer)
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


# Low-memory warning (issue #2002) — "Continue anyway": keep the scene running,
# turn on verbose memory logging to capture the run-up to a possible OOM, and
# tell the runner to stop warning for the rest of the session.
func _on_low_memory_continue() -> void:
	if Global.scene_runner != null:
		Global.scene_runner.set_memory_verbose_logging(true)
		Global.scene_runner.dismiss_memory_warning()
	close_current_modal()


# Low-memory warning — "Back to Discover": leave the heavy scene.
func _on_low_memory_back() -> void:
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
	# Re-show the ban pre-check modal if it is still active and re-open discover
	if ban_pre_check_active:
		async_show_ban_pre_check_modal.call_deferred()


## Re-shows the ban pre-check modal if it is still keeping the user out of the explorer.
## Returns true when it was re-shown, so callers can stop their own navigation. The private
## world modal is intentionally not covered here — its OK button just dismisses.
func reshow_ban_modal() -> bool:
	if ban_pre_check_active:
		async_show_ban_pre_check_modal()
		return true
	return false


## Clear suppress flag after loading finishes.
func _on_loading_finished_clear_suppress() -> void:
	_suppress_ban_kicked = false


func _remove_modal() -> void:
	if current_modal:
		_disconnect_button_signals()
		if current_modal.tree_exited.is_connected(_on_modal_tree_exited):
			current_modal.tree_exited.disconnect(_on_modal_tree_exited)
		current_modal.queue_free()
		current_modal = null
	if _canvas_layer and is_instance_valid(_canvas_layer):
		_canvas_layer.queue_free()
		_canvas_layer = null


func _disconnect_travel_signals() -> void:
	if not current_travel_modal:
		return
	for connection in current_travel_modal.closed.get_connections():
		current_travel_modal.closed.disconnect(connection.callable)
	for connection in current_travel_modal.jump_in_pressed.get_connections():
		current_travel_modal.jump_in_pressed.disconnect(connection.callable)


func _remove_travel_modal() -> void:
	if current_travel_modal:
		_disconnect_travel_signals()
		if current_travel_modal.tree_exited.is_connected(_on_travel_modal_tree_exited):
			current_travel_modal.tree_exited.disconnect(_on_travel_modal_tree_exited)
		current_travel_modal.queue_free()
		current_travel_modal = null
	if _travel_canvas_layer and is_instance_valid(_travel_canvas_layer):
		_travel_canvas_layer.queue_free()
		_travel_canvas_layer = null


func _async_create_input_modal() -> InputModal:
	if current_input_modal:
		close_input_modal()

	if not input_modal_scene:
		push_error("ModalManager: Input modal scene is not loaded")
		return null

	var modal = input_modal_scene.instantiate() as InputModal
	if not modal:
		push_error("ModalManager: Could not instantiate input modal")
		return null

	if _input_canvas_layer and is_instance_valid(_input_canvas_layer):
		_input_canvas_layer.get_parent().remove_child(_input_canvas_layer)
		_input_canvas_layer.queue_free()

	_input_canvas_layer = CanvasLayer.new()
	_input_canvas_layer.layer = 100

	var root = get_tree().root
	if not root:
		push_error("ModalManager: Could not get scene tree root")
		return null

	root.add_child(_input_canvas_layer)
	_input_canvas_layer.add_child(modal)
	current_input_modal = modal

	current_input_modal.tree_exited.connect(_on_input_modal_tree_exited)

	await get_tree().process_frame
	await get_tree().process_frame

	return modal


func _on_input_modal_tree_exited() -> void:
	if current_input_modal:
		current_input_modal = null


func _remove_input_modal() -> void:
	if current_input_modal:
		if current_input_modal.tree_exited.is_connected(_on_input_modal_tree_exited):
			current_input_modal.tree_exited.disconnect(_on_input_modal_tree_exited)
		current_input_modal.queue_free()
		current_input_modal = null
	if _input_canvas_layer and is_instance_valid(_input_canvas_layer):
		_input_canvas_layer.queue_free()
		_input_canvas_layer = null


func async_show_code_modal(email: String = "") -> CodeModal:
	var modal = await _async_create_code_modal()
	if not modal:
		return null
	modal.open(email)
	return modal


func close_code_modal() -> void:
	if current_code_modal:
		current_code_modal.close()
		_remove_code_modal()


func _async_create_code_modal() -> CodeModal:
	if current_code_modal:
		close_code_modal()

	if not code_modal_scene:
		push_error("ModalManager: Code modal scene is not loaded")
		return null

	var modal = code_modal_scene.instantiate() as CodeModal
	if not modal:
		push_error("ModalManager: Could not instantiate code modal")
		return null

	if _code_canvas_layer and is_instance_valid(_code_canvas_layer):
		_code_canvas_layer.get_parent().remove_child(_code_canvas_layer)
		_code_canvas_layer.queue_free()

	_code_canvas_layer = CanvasLayer.new()
	_code_canvas_layer.layer = 100

	var root = get_tree().root
	if not root:
		push_error("ModalManager: Could not get scene tree root")
		return null

	root.add_child(_code_canvas_layer)
	_code_canvas_layer.add_child(modal)
	current_code_modal = modal

	current_code_modal.tree_exited.connect(_on_code_modal_tree_exited)

	await get_tree().process_frame
	await get_tree().process_frame

	return modal


func _on_code_modal_tree_exited() -> void:
	if current_code_modal:
		current_code_modal = null


func _remove_code_modal() -> void:
	if current_code_modal:
		if current_code_modal.tree_exited.is_connected(_on_code_modal_tree_exited):
			current_code_modal.tree_exited.disconnect(_on_code_modal_tree_exited)
		current_code_modal.queue_free()
		current_code_modal = null
	if _code_canvas_layer and is_instance_valid(_code_canvas_layer):
		_code_canvas_layer.queue_free()
		_code_canvas_layer = null


## Shows the reward modal for a claimable campaign.
## @param campaign: { campaign_id, campaign_key, urn } — see RewardCampaigns.CAMPAIGNS.
## Fetches the item thumbnail (when a urn is provided) before revealing the modal.
## Returns true only if the modal was actually created and revealed, so callers can
## gate analytics / cadence bookkeeping on a real appearance.
func async_show_reward_modal(campaign: Dictionary) -> bool:
	var modal = await _async_create_reward_modal()
	if not modal:
		return false
	await modal.async_setup(campaign)
	return true


func close_reward_modal() -> void:
	if current_reward_modal:
		current_reward_modal.hide()
		_remove_reward_modal()


func _async_create_reward_modal() -> RewardModal:
	if current_reward_modal:
		close_reward_modal()

	if not reward_modal_scene:
		push_error("ModalManager: Reward modal scene is not loaded")
		return null

	var modal = reward_modal_scene.instantiate() as RewardModal
	if not modal:
		push_error("ModalManager: Could not instantiate reward modal")
		return null

	if _reward_canvas_layer and is_instance_valid(_reward_canvas_layer):
		_reward_canvas_layer.get_parent().remove_child(_reward_canvas_layer)
		_reward_canvas_layer.queue_free()

	_reward_canvas_layer = CanvasLayer.new()
	_reward_canvas_layer.layer = 100

	var root = get_tree().root
	if not root:
		push_error("ModalManager: Could not get scene tree root")
		return null

	root.add_child(_reward_canvas_layer)
	_reward_canvas_layer.add_child(modal)
	current_reward_modal = modal

	current_reward_modal.tree_exited.connect(_on_reward_modal_tree_exited)

	await get_tree().process_frame
	await get_tree().process_frame

	return modal


func _on_reward_modal_tree_exited() -> void:
	if current_reward_modal:
		current_reward_modal = null


func _remove_reward_modal() -> void:
	if current_reward_modal:
		if current_reward_modal.tree_exited.is_connected(_on_reward_modal_tree_exited):
			current_reward_modal.tree_exited.disconnect(_on_reward_modal_tree_exited)
		current_reward_modal.queue_free()
		current_reward_modal = null
	if _reward_canvas_layer and is_instance_valid(_reward_canvas_layer):
		_reward_canvas_layer.queue_free()
		_reward_canvas_layer = null


## Shows the aspirational guest-upgrade nudge modal (issue #2372). Returns true only if the
## modal was actually created and opened, so the coordinator advances its cadence only then.
func async_show_upgrade_modal() -> bool:
	var modal = await _async_create_upgrade_modal()
	if not modal:
		return false
	modal.open()
	return true


func close_upgrade_modal() -> void:
	if current_upgrade_modal:
		current_upgrade_modal.hide()
		_remove_upgrade_modal()


func _async_create_upgrade_modal() -> UpgradeModal:
	if current_upgrade_modal:
		close_upgrade_modal()

	if not upgrade_modal_scene:
		push_error("ModalManager: Upgrade modal scene is not loaded")
		return null

	var modal = upgrade_modal_scene.instantiate() as UpgradeModal
	if not modal:
		push_error("ModalManager: Could not instantiate upgrade modal")
		return null

	if _upgrade_canvas_layer and is_instance_valid(_upgrade_canvas_layer):
		_upgrade_canvas_layer.get_parent().remove_child(_upgrade_canvas_layer)
		_upgrade_canvas_layer.queue_free()

	_upgrade_canvas_layer = CanvasLayer.new()
	_upgrade_canvas_layer.layer = 100

	var root = get_tree().root
	if not root:
		push_error("ModalManager: Could not get scene tree root")
		return null

	root.add_child(_upgrade_canvas_layer)
	_upgrade_canvas_layer.add_child(modal)
	current_upgrade_modal = modal

	current_upgrade_modal.tree_exited.connect(_on_upgrade_modal_tree_exited)

	await get_tree().process_frame
	await get_tree().process_frame

	return modal


func _on_upgrade_modal_tree_exited() -> void:
	if current_upgrade_modal:
		current_upgrade_modal = null


func _remove_upgrade_modal() -> void:
	if current_upgrade_modal:
		if current_upgrade_modal.tree_exited.is_connected(_on_upgrade_modal_tree_exited):
			current_upgrade_modal.tree_exited.disconnect(_on_upgrade_modal_tree_exited)
		current_upgrade_modal.queue_free()
		current_upgrade_modal = null
	if _upgrade_canvas_layer and is_instance_valid(_upgrade_canvas_layer):
		_upgrade_canvas_layer.queue_free()
		_upgrade_canvas_layer = null


# --- Add Email (guest upgrade) OTP flow (issue #2377) --------------------------------------
# Shared entry point that links an email to the active thirdweb guest wallet. Used by both the
# Discover/Settings upgrade notice (guest_upgrade_card) and the aspirational upgrade modal, so
# the "Add Email" experience is identical everywhere. On success it grants the reward wearable
# (issue #2372) and notifies the app that the guest upgraded.


static func is_valid_email(text: String) -> bool:
	return _add_email_regex.search(text) != null


## Opens the "Add Email" input modal and drives the full OTP link flow (email → code → verify →
## reward). Callers fire their own entry CLICK_BUTTON metric before calling this.
func async_start_add_email_flow() -> void:
	var modal = await async_show_input_modal(
		"Add Email", "My email", "name@email.com", "ADD", "CANCEL", is_valid_email
	)
	if modal:
		# Add Email modal shown — the OTP upgrade funnel has started (issue #2377).
		Global.metrics.track_screen_viewed("UPGRADE_OTP_START", "")
		modal.dismissable = false
		modal.dcl_text_edit.wrap_text = false
		modal.dcl_text_edit.validate_on_blur = true
		modal.set_submit_handler(_async_add_email_submit)
		modal.confirmed.connect(_async_add_email_code_sent)
		modal.failed.connect(_async_add_email_send_failed)


# gdlint:ignore = async-function-name
func _async_add_email_submit(email: String) -> Dictionary:
	Global.metrics.track_screen_viewed("UPGRADE_OTP_SUBMIT", "")
	return await _async_add_email_send_code(email)


# Shared "send verification code" routine used by both the initial submit and resend.
# gdlint:ignore = async-function-name
func _async_add_email_send_code(email: String) -> Dictionary:
	var promise: Promise = Global.player_identity.async_link_email_start(email)
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		var raw: String = result.get_error()
		push_warning("Upgrade to OTP - send code failed: " + raw)
		if _add_email_is_invalid_error(raw):
			Global.metrics.track_screen_viewed("UPGRADE_OTP_EMAIL_INVALID", "")
			return {"status": InputModal.SUBMIT_INVALID, "message": "Invalid email"}
		return {"status": InputModal.SUBMIT_ERROR, "message": raw}
	print("[UpgradeOTP] send_code OK for: ", email)
	return {"status": InputModal.SUBMIT_OK}


# Code was sent successfully (Add Email modal already closed): open the code step.
# gdlint:ignore = async-function-name
func _async_add_email_code_sent(email: String) -> void:
	var code_modal = await async_show_code_modal(email)
	if code_modal:
		Global.metrics.track_screen_viewed("UPGRADE_OTP_ENTERCODE", "")
		code_modal.set_verifier(_async_add_email_verify.bind(email))
		code_modal.set_resend_handler(_async_add_email_resend.bind(email))
		code_modal.confirmed.connect(_async_add_email_confirmed.bind(email))
		code_modal.cancelled.connect(close_code_modal)


# gdlint:ignore = async-function-name
func _async_add_email_resend(email: String) -> Dictionary:
	Global.metrics.track_click_button("RESEND_OTP", "UPGRADE_OTP_ENTERCODE", "")
	return await _async_add_email_send_code(email)


# gdlint:ignore = async-function-name
func _async_add_email_send_failed(_message: String) -> void:
	await _async_add_email_error_modal(
		"Something went wrong", "Something went wrong. Please try again later."
	)


func _add_email_is_invalid_error(raw: String) -> bool:
	var lower := raw.to_lower()
	if lower.contains("invalid email"):
		return true
	return (
		lower.contains("email") and (lower.contains("zoderror") or lower.contains("invalid_string"))
	)


# Returns "" on success, or a friendly error string the code modal shows inline.
# gdlint:ignore = async-function-name
func _async_add_email_verify(code: String, email: String) -> String:
	Global.metrics.track_screen_viewed("UPGRADE_OTP_VERIFY", "")
	var anchor: String = Global.get_device_anchor_id()
	var promise: Promise = Global.player_identity.async_link_email_verify(email, code, anchor)
	var result = await PromiseUtils.async_awaiter(promise)
	print("[UpgradeOTP] verify result: ", result)
	if result is PromiseError:
		var raw: String = result.get_error()
		push_warning("[UpgradeOTP] verify FAILED: " + raw)
		if _add_email_is_already_linked_error(raw):
			Global.metrics.track_screen_viewed("UPGRADE_OTP_EMAIL_BUSY", "")
			close_code_modal.call_deferred()
			_async_add_email_in_use.call_deferred()
			return " "
		Global.metrics.track_screen_viewed("UPGRADE_OTP_CODE_INVALID", "")
		return _add_email_friendly_error(raw)
	print("[UpgradeOTP] verify OK")
	return ""


func _add_email_is_already_linked_error(raw: String) -> bool:
	var lower := raw.to_lower()
	return lower.contains("already") or lower.contains("linked") or lower.contains("conflict")


func _async_add_email_in_use() -> void:
	await _async_add_email_error_modal(
		"Email already in use",
		"This email is already linked to another account.\nTry a different email.",
	)


# gdlint:ignore = async-function-name
func _async_add_email_confirmed(_code: String, _email: String) -> void:
	# Verification already succeeded inside the code modal before `confirmed` fired.
	Global.metrics.track_screen_viewed("ACCOUNT_UPGRADE_SUCCESS", JSON.stringify({"method": "otp"}))
	# Mirror the lobby's AUTH_SUCCESS so the funnel sees the upgraded user as registered.
	Global.metrics.track_screen_viewed(
		"AUTH_SUCCESS", JSON.stringify({"login_type": "fully_registered"})
	)
	close_code_modal()
	# Grant the exclusive upgrade wearable via the reward modal (issue #2372). The reward
	# modal IS the success screen ("Your email has been verified. Enjoy the free wearable!").
	# Report the SCREEN_VIEW only once the modal has actually been revealed.
	var reward_shown := await async_show_reward_modal(RewardCampaigns.CAMPAIGNS["MobilePet"])
	if reward_shown:
		Global.metrics.track_screen_viewed(
			"ACCOUNT_UPGRADE_REWARD_SHOW", JSON.stringify({"method": "otp"})
		)
	# The guest is now upgraded (Rust set the cached flag on link) — notify the app so the
	# Discover/Settings notice, badge, credits button, etc. update without a network call.
	Global.guest_upgrade_state_refreshed.emit(true)


# Maps raw thirdweb errors to friendly copy. The raw error is still logged.
func _add_email_friendly_error(raw: String) -> String:
	var lower := raw.to_lower()
	if lower.contains("429") or lower.contains("rate"):
		return "Too many attempts. Please wait a moment and try again."
	if lower.contains("already") or lower.contains("linked") or lower.contains("conflict"):
		return "This email is already linked to another account."
	if lower.contains("invalid") or lower.contains("code") or lower.contains("400"):
		return "The code is invalid or expired. Please resend code."
	return "Something went wrong. Please try again."


func _async_add_email_error_modal(title: String, body: String) -> void:
	var modal = await _async_create_modal()
	if not modal:
		return
	modal.set_title(title)
	modal.set_body(body)
	modal.set_primary_button_text("OK")
	modal.show_icon(Modal.MODAL_ALERT_ICON)
	modal.button_secondary.hide()
	modal.hide_url()
	modal.blocker = true
	modal.show()
	await modal.button_primary.pressed
	close_current_modal()
