class_name DiscoverPanel
extends PanelContainer

## Landscape side panel opened from the navbar Discover button. Shows the current scene header
## (title + creator + actions menu), the featured places carousel and the events carousel, plus an
## EXPLORE MORE button that opens the full-screen Discover. Mirrors the show/hide contract of
## NotificationsPanel / FriendsPanel so explorer.gd can dock it in %VBoxContainer_LeftPanels.

signal panel_closed
## Emitted when the user picks "Share" in the actions menu. explorer.gd shares the current scene.
signal share_requested

# Tint of the actions (⋮) button: light when idle, purple while its menu is open.
const MENU_COLOR_IDLE := Color(0.9882353, 0.9882353, 0.9882353, 1)
const MENU_COLOR_ACTIVE := Color(0.9098039, 0.7254902, 1, 1)

@export var featured: VBoxContainer

var _header_request_id: int = 0
# Carousels are populated on first show (not at _ready): while the panel is hidden it has no width,
# and cards built then trim their titles to nothing and skip thumbnails permanently.
var _content_loaded: bool = false

@onready var scroll_container: ScrollContainer = %ScrollContainer
@onready var events: VBoxContainer = %Events
@onready var label_title: Label = %Label_Title
@onready var label_creator: Label = %Label_Creator
@onready var button_menu: TextureButton = %Button_Menu
@onready var menu_overlay: MarginContainer = %MenuOverlay
@onready var menu_dropdown: PanelContainer = %MenuDropdown
@onready var button_share: Button = %Button_Share
@onready var button_report_content: Button = %Button_ReportContent
@onready var button_report_bug: Button = %Button_ReportBug
@onready var button_explore_more: Button = %Button_ExploreMore


func _ready() -> void:
	# Block touch/mouse from reaching the 3D camera while the panel is up.
	mouse_filter = Control.MOUSE_FILTER_STOP
	set_process_input(true)

	button_menu.pressed.connect(_toggle_menu)
	menu_overlay.gui_input.connect(_on_menu_overlay_gui_input)
	button_share.pressed.connect(_on_share_pressed)
	button_report_content.pressed.connect(_on_report_content_pressed)
	button_report_bug.pressed.connect(_on_report_bug_pressed)
	button_explore_more.pressed.connect(_on_explore_more_pressed)
	# i18n-keys: DISCOVER_EXPLORE_MORE
	_apply_explore_more_label()
	featured.generator.item_pressed.connect(_on_card_jump_in)
	events.generator.item_pressed.connect(_on_card_jump_in)

	_close_menu()

	# Keep the header in sync while the panel stays open and the avatar walks into a new scene.
	Global.change_parcel.connect(_on_change_parcel)


func _input(event: InputEvent) -> void:
	if not is_visible_in_tree():
		return
	if not (event is InputEventScreenTouch or event is InputEventScreenDrag):
		return
	# Release camera focus on a touch press inside the panel so drags scroll the list instead of
	# rotating the camera (same behaviour as NotificationsPanel / FriendsPanel).
	if event is InputEventScreenTouch and event.pressed:
		if get_global_rect().has_point(event.position) and Global.explorer_has_focus():
			Global.explorer_release_focus()


func show_panel() -> void:
	show()
	_close_menu()
	_reset_scroll()
	_load_content_once()
	_async_refresh_header()


func _reset_scroll() -> void:
	scroll_container.scroll_vertical = 0
	# No-op if a carousel hasn't built any cards yet (e.g. the very first show, before
	# _load_content_once's start_loading() runs) — reset_position() checks for a valid child.
	featured.scroll_to_start()
	events.scroll_to_start()


func _load_content_once() -> void:
	# Build the carousels now that the panel is visible and has a real width (see _content_loaded).
	if _content_loaded:
		return
	_content_loaded = true
	# One frame so the panel is laid out at its real width before cards are built; otherwise
	# thumbnails/titles can bake a zero-size layout from when the panel was still hidden.
	await get_tree().process_frame
	if not is_inside_tree():
		return
	featured.start_loading()
	events.start_loading()


# --- Carousel cards ---


## A Featured/Events card was tapped: collapse the navbar (this panel closes with it) and show
## the same jump-in confirmation modal used elsewhere in the app (deep links, chat links, ...).
func _on_card_jump_in(data) -> void:
	var explorer = Global.get_explorer()
	if is_instance_valid(explorer):
		explorer.navbar.collapse()
	if PlacesHelper.is_world(data):
		var realm: String = PlacesHelper.get_position_and_realm(data)[1]
		Global.modal_manager.async_show_world_modal(realm)
	else:
		Global.modal_manager.async_show_teleport_modal(PlacesHelper.parse_position(data))


# The EXPLORE MORE label is shouted per the design; DISCOVER_EXPLORE_MORE is shared with the FTUE
# (lower-case there), so we upper-case only this instance from code instead of in the catalogue.
func _apply_explore_more_label() -> void:
	button_explore_more.text = tr("DISCOVER_EXPLORE_MORE").to_upper()


func _notification(what: int) -> void:
	if what == NOTIFICATION_TRANSLATION_CHANGED and is_instance_valid(button_explore_more):
		_apply_explore_more_label()


func hide_panel() -> void:
	_close_menu()
	hide()


func _on_change_parcel(_new_parcel: Vector2i) -> void:
	if is_visible_in_tree():
		_async_refresh_header()


# --- Header (current scene) ---


func _async_refresh_header() -> void:
	# A place lookup can outlive a fast scene change; only the newest request may write the labels.
	_header_request_id += 1
	var request_id := _header_request_id

	var scene_title := _current_scene_title()
	label_title.text = scene_title
	label_creator.text = ""

	var result
	if Realm.is_genesis_city(Global.realm.realm_url):
		var pos: Vector2i = Global.scene_fetcher.current_position
		if pos == SceneFetcher.INVALID_PARCEL:
			return
		result = await PlacesHelper.async_get_by_position(pos)
	else:
		# Worlds don't share Genesis City's coordinate grid — (0,0) there is not Genesis Plaza.
		# Look the place up by realm name instead, same as places_generator.gd's last-places list.
		result = await PlacesHelper.async_get_by_names(Global.realm.realm_name)

	if request_id != _header_request_id or not is_visible_in_tree():
		return
	if result is PromiseError:
		return

	var json: Dictionary = result.get_string_response_as_json()
	var data: Array = json.get("data", [])
	if data.is_empty():
		return

	var place: Dictionary = data[0]
	# The places API can return `title`/`contact_name` as JSON null; Dictionary.get returns that
	# null (not the default) when the key exists, and Label.text = null crashes — so guard both.
	var title = place.get("title", scene_title)
	if title == null:
		title = scene_title
	if not title.is_empty():
		label_title.text = title
	var creator = place.get("contact_name", "")
	label_creator.text = creator if creator != null else ""


func _current_scene_title() -> String:
	var scene = Global.scene_fetcher.get_current_scene_data()
	if scene != null and scene.scene_entity_definition != null:
		return scene.scene_entity_definition.get_title()
	return ""


# --- Actions menu (⋮) ---


func _toggle_menu() -> void:
	if menu_dropdown.visible:
		_close_menu()
	else:
		_open_menu()


func _open_menu() -> void:
	menu_dropdown.show()
	# While open the overlay swallows taps so a tap outside the dropdown closes it.
	menu_overlay.mouse_filter = Control.MOUSE_FILTER_STOP
	button_menu.modulate = MENU_COLOR_ACTIVE


func _close_menu() -> void:
	menu_dropdown.hide()
	menu_overlay.mouse_filter = Control.MOUSE_FILTER_IGNORE
	button_menu.modulate = MENU_COLOR_IDLE


func _on_menu_overlay_gui_input(event: InputEvent) -> void:
	# The overlay only receives events in the area NOT covered by the dropdown, so any press here
	# is a tap outside the menu → dismiss it.
	if event is InputEventScreenTouch and event.pressed:
		_close_menu()
	elif event is InputEventMouseButton and event.pressed:
		_close_menu()


func _on_share_pressed() -> void:
	_close_menu()
	share_requested.emit()


func _on_report_content_pressed() -> void:
	_close_menu()
	ReportContentHelper.open_form()


func _on_report_bug_pressed() -> void:
	_close_menu()
	# Same entry point as tapping Discover / EXPLORE MORE: opens the full-screen (portrait) Discover
	# behind the modal — existing wiring already collapses the navbar/this panel and forces portrait,
	# so there's nothing extra to do here before showing the modal on top.
	Global.open_discover.emit()
	_async_open_bug_report()


func _async_open_bug_report() -> void:
	# Same flow as Settings' Report Bug: the screenshot was captured when the panel opened.
	var modal = await Global.modal_manager.async_show_bug_report_modal(
		BugReportCapture.latest_jpeg()
	)
	if not is_instance_valid(modal):
		return
	modal.submitted.connect(_async_on_bug_report_submitted)
	modal.failed.connect(_on_bug_report_failed)


func _async_on_bug_report_submitted(_ticket_id: String) -> void:
	await Global.modal_manager.async_show_bug_report_success_modal()


func _on_bug_report_failed(message: String) -> void:
	push_warning("Bug report failed: %s" % message)
	NotificationsManager.show_system_toast(
		tr("TOAST_BUG_REPORT_FAILED_TITLE"),
		tr("COMMON_SOMETHING_WENT_WRONG_RETRY"),
		"system",
		"alert"
	)


func _on_explore_more_pressed() -> void:
	_close_menu()
	Global.open_discover.emit()
