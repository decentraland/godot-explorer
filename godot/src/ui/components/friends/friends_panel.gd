extends PanelContainer

signal panel_closed

@onready var color_rect_friends: ColorRect = %ColorRect_Friends
@onready var color_rect_nearby: ColorRect = %ColorRect_Nearby
@onready var color_rect_blocked: ColorRect = %ColorRect_Blocked
@onready var button_friends: Button = %Button_Friends
@onready var avatars_list: Control = %AvatarsList

func _ready() -> void:
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


func hide_panel() -> void:
	hide()


func _hide_all() -> void:
	color_rect_friends.self_modulate = Color.TRANSPARENT
	color_rect_nearby.self_modulate = Color.TRANSPARENT
	color_rect_blocked.self_modulate = Color.TRANSPARENT


func _on_button_friends_toggled(toggled_on: bool) -> void:
	if toggled_on:
		_hide_all()
		color_rect_friends.self_modulate = Color.WHITE


func _on_button_nearby_toggled(toggled_on: bool) -> void:
	if toggled_on:
		_hide_all()
		color_rect_nearby.self_modulate = Color.WHITE


func _on_button_blocked_toggled(toggled_on: bool) -> void:
	if toggled_on:
		_hide_all()
		color_rect_blocked.self_modulate = Color.WHITE
