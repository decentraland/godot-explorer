extends PanelContainer

signal panel_closed

const NO_FRIENDS_MSG: String = "You don’t have any friends or pending requests.\nGo make some friends!"

var down_arrow_icon: CompressedTexture2D = load("res://assets/ui/down_arrow.svg")
var up_arrow_icon: CompressedTexture2D = load("res://assets/ui/up_arrow.svg")

var online_friends: int = 1
var offline_friends: int = 2
var pending_request: int = 3

@onready var color_rect_friends: ColorRect = %ColorRect_Friends
@onready var color_rect_nearby: ColorRect = %ColorRect_Nearby
@onready var color_rect_blocked: ColorRect = %ColorRect_Blocked
@onready var scroll_container_friends: ScrollContainer = %ScrollContainer_Friends
@onready var scroll_container_nearby: ScrollContainer = %ScrollContainer_Nearby
@onready var scroll_container_blocked: ScrollContainer = %ScrollContainer_Blocked

@onready var request_button: Button = %RequestButton
@onready var online_button: Button = %OnlineButton
@onready var offline_button: Button = %OfflineButton

@onready var v_box_container_request: VBoxContainer = %VBoxContainer_Request
@onready var v_box_container_online: VBoxContainer = %VBoxContainer_Online
@onready var v_box_container_offline: VBoxContainer = %VBoxContainer_Offline
@onready var request_list: VBoxContainer = %RequestList
@onready var online_list: VBoxContainer = %OnlineList
@onready var offline_list: VBoxContainer = %OfflineList
@onready var nearby_list: SocialList = %NearbyList
@onready var blocked_list: SocialList = %BlockedList

@onready var label_empty_state: Label = %LabelEmptyState


func _ready() -> void:
	_update_dropdown_visibility()
	request_list.hide()
	offline_list.hide()
	# Ensure the panel blocks touch/mouse events from passing through when visible
	mouse_filter = Control.MOUSE_FILTER_STOP
	set_process_input(true)
	_on_button_friends_toggled(true)


func _input(event: InputEvent) -> void:
	# Only handle input when panel is visible in tree
	if not is_visible_in_tree():
		return

	# Only process touch events (includes emulated touch from mouse)
	# Ignore mouse events to avoid duplicate processing with emulation enabled
	if not (event is InputEventScreenTouch or event is InputEventScreenDrag):
		return

	# Check if input is within the panel's rectangle
	var pos = event.position
	var rect = get_global_rect()
	var is_inside_panel = rect.has_point(pos)

	# Only release focus on touch press (not during drag) to prevent camera rotation
	# This allows ScrollContainer to handle drag events normally
	if is_inside_panel and event is InputEventScreenTouch and event.pressed:
		if Global.explorer_has_focus():
			Global.explorer_release_focus()


func show_panel() -> void:
	show()
	update_all_lists()


func hide_panel() -> void:
	hide()
	update_all_lists()


func _hide_all() -> void:
	color_rect_friends.self_modulate = Color.TRANSPARENT
	color_rect_nearby.self_modulate = Color.TRANSPARENT
	color_rect_blocked.self_modulate = Color.TRANSPARENT
	scroll_container_nearby.hide()
	scroll_container_friends.hide()
	scroll_container_blocked.hide()


func _on_button_friends_toggled(toggled_on: bool) -> void:
	if toggled_on:
		_hide_all()
		color_rect_friends.self_modulate = Color.WHITE
		scroll_container_friends.show()


func _on_button_nearby_toggled(toggled_on: bool) -> void:
	if toggled_on:
		_hide_all()
		color_rect_nearby.self_modulate = Color.WHITE
		scroll_container_nearby.show()


func _on_button_blocked_toggled(toggled_on: bool) -> void:
	print(Global.social_blacklist.get_blocked_list())
	if toggled_on:
		_hide_all()
		color_rect_blocked.self_modulate = Color.WHITE
		scroll_container_blocked.show()


func _on_request_button_toggled(toggled_on: bool) -> void:
	if toggled_on:
		request_button.icon = up_arrow_icon
		request_list.show()
	else:
		request_button.icon = down_arrow_icon
		request_list.hide()


func _on_online_button_toggled(toggled_on: bool) -> void:
	if toggled_on:
		online_button.icon = up_arrow_icon
		online_list.show()
	else:
		online_button.icon = down_arrow_icon
		online_list.hide()


func _on_offline_button_toggled(toggled_on: bool) -> void:
	if toggled_on:
		offline_button.icon = up_arrow_icon
		offline_list.show()
	else:
		offline_button.icon = down_arrow_icon
		offline_list.hide()


func _update_dropdown_visibility() -> void:
	if pending_request == 0:
		v_box_container_request.hide()
	else:
		v_box_container_request.show()
		request_button.text = "REQUESTS (" + str(pending_request) + ")"

	if online_friends == 0:
		v_box_container_online.hide()
	else:
		v_box_container_online.show()
		online_button.text = "ONLINE (" + str(online_friends) + ")"

	if offline_friends == 0:
		v_box_container_offline.hide()
	else:
		v_box_container_offline.show()
		offline_button.text = "OFFLINE (" + str(offline_friends) + ")"

	if online_friends == 0 and offline_friends == 0 and pending_request == 0:
		label_empty_state.text = NO_FRIENDS_MSG
	else:
		label_empty_state.hide()


func update_all_lists():
	request_list.update_list()
	online_list.update_list()
	offline_list.update_list()
	nearby_list.update_list()
	blocked_list.update_list()
