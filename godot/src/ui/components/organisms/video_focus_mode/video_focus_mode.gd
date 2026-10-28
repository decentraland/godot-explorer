class_name VideoFocusMode
extends Control

## Full-screen video screen with the Nearby chat docked beside it (landscape) or under it
## (portrait) — issue #1381. Opens on the scene runner's open_video_focus_mode signal (the
## crosshair on a playing video screen, then the "Focus mode" pill) and closes from the exit
## button, a teleport or the menu.

const CHAT_SCENE := preload("res://src/ui/pages/chat/chat.tscn")
## Two drawings so the stroke stays 4 px at each orientation's chevron size.
const CHEVRON_ICON_LANDSCAPE := preload(
	"res://src/ui/components/atoms/buttons/hud_button/icons/hud-chevron-down.svg"
)
const CHEVRON_ICON_PORTRAIT := preload(
	"res://src/ui/components/atoms/buttons/hud_button/icons/hud-chevron-down-large.svg"
)
## How long the exit button keeps its label before collapsing to the icon.
const EXIT_LABEL_SECONDS := 3.0
## Chat column beside the video (design: 430 of 1600 landscape px).
const LANDSCAPE_CHAT_WIDTH := 430
## Video height over width in portrait until the stream reports its own size.
const DEFAULT_VIDEO_ASPECT := 9.0 / 16.0
## get_backend_texture() hands out a 2x2 black placeholder before the first frame.
const PLACEHOLDER_MAX_SIZE := 2
## Messages re-measure over a few frames after a relayout; the scroll to the latest line
## has to wait for that or it lands on the old content height.
const SCROLL_SETTLE_SECONDS := 0.25
## The NEARBY title and the member count, per the design.
const HEADER_FONT_SIZE_LANDSCAPE := 24
const HEADER_FONT_SIZE_PORTRAIT := 28
## Side margin of the exit/rotate row: the HUD chat's 40 px in portrait (hud_margins.tres).
const CONTROLS_SIDE_MARGIN_LANDSCAPE := 48
const CONTROLS_SIDE_MARGIN_PORTRAIT := 40
## Chat header per the design: 76 px tall, 16 x 10 px padding, 24 px icon and 48 px chevron in
## landscape; 96 px tall, 32 x 13 px padding, 28 px icon and 56 px chevron in portrait.
const HEADER_HEIGHT_LANDSCAPE := 76
const HEADER_HEIGHT_PORTRAIT := 96
const HEADER_PADDING_X_LANDSCAPE := 16
const HEADER_PADDING_X_PORTRAIT := 32
const HEADER_PADDING_Y_LANDSCAPE := 10
const HEADER_PADDING_Y_PORTRAIT := 13
const MEMBERS_ICON_LANDSCAPE := 24
const MEMBERS_ICON_PORTRAIT := 28
const CHEVRON_SIZE_LANDSCAPE := 48
const CHEVRON_SIZE_PORTRAIT := 56
## Space between the member count and the chevron.
const CHEVRON_GAP_LANDSCAPE := 18
const CHEVRON_GAP_PORTRAIT := 32
## Exit glyph inside its disc, per the design: centred in height, a little left of centre.
const EXIT_ICON_LANDSCAPE := 36
const EXIT_ICON_PORTRAIT := 54
const EXIT_ICON_SHIFT_LANDSCAPE := 2
const EXIT_ICON_SHIFT_PORTRAIT := 4
## The edge tab's distance from the screen edge, before the device's side inset is added.
const EXPAND_TAB_EDGE_MARGIN := 24

var _video_player: Node = null
var _chat: Control = null
var _exit_label_tween: Tween
## Hidden beside the video in landscape, shrunk to a strip under it in portrait.
var _chat_collapsed: bool = false
## Bumped on every relayout and on exit, so a settle timer from an earlier layout (or an
## earlier session) never reveals or scrolls the chat of a later one.
var _chat_reveal_epoch: int = 0
## The device's left inset from the last _apply_side_insets, for the landscape input.
var _inset_left: int = 0

@onready var _layout: BoxContainer = %BoxContainer_Layout
@onready var _video_panel: Control = %Control_Video
@onready var _texture_rect: TextureRect = %TextureRect_Video
@onready var _chat_bottom_inset: MarginContainer = %SafeMarginContainer_ChatBottom
@onready var _chat_column: PanelContainer = %PanelContainer_Chat
@onready var _chat_header: PanelContainer = %PanelContainer_Header
@onready var _collapse_slot: Control = %Control_CollapseSlot
@onready var _chat_slot: Control = %Control_ChatSlot
@onready var _label_nearby: Label = %Label_Nearby
@onready var _members_icon: TextureRect = %TextureRect_Members
@onready var _label_members: Label = %Label_Members
@onready var _controls_margin: MarginContainer = %MarginContainer_Controls
@onready var _button_exit: Button = %Button_Exit
@onready var _label_exit: Label = %Label_Exit
@onready var _button_rotate: HudButton = %Button_Rotate
@onready var _button_collapse_chat: Button = %Button_CollapseChat
@onready var _button_expand_chat: Button = %Button_ExpandChat
@onready var _keyboard_margin: Control = %Control_KeyboardMargin


func _ready() -> void:
	hide()
	set_process(false)
	_button_exit.pressed.connect(exit)
	_button_rotate.pressed.connect(_on_button_rotate_pressed)
	_button_collapse_chat.pressed.connect(_on_button_collapse_chat_pressed)
	_button_expand_chat.pressed.connect(_on_button_collapse_chat_pressed)
	# The chevrons turn in place: the style's content margins make the buttons wider than
	# tall, so the pivot follows the real size.
	_button_collapse_chat.resized.connect(
		func(): _button_collapse_chat.pivot_offset = _button_collapse_chat.size * 0.5
	)
	_button_expand_chat.resized.connect(
		func(): _button_expand_chat.pivot_offset = _button_expand_chat.size * 0.5
	)
	resized.connect(_update_video_panel_size)
	# The safe area changes with the rotation, which lands as a resize of the whole overlay.
	resized.connect(_apply_side_insets)
	Global.scene_runner.open_video_focus_mode.connect(enter)
	Global.orientation_changed.connect(_on_orientation_changed)
	Global.change_virtual_keyboard.connect(_on_change_virtual_keyboard)
	Global.loading_started.connect(exit)
	Global.on_menu_open.connect(exit)


func _exit_tree() -> void:
	# Global and its scene_runner outlive this node.
	Global.scene_runner.open_video_focus_mode.disconnect(enter)
	Global.orientation_changed.disconnect(_on_orientation_changed)
	Global.change_virtual_keyboard.disconnect(_on_change_virtual_keyboard)
	Global.loading_started.disconnect(exit)
	Global.on_menu_open.disconnect(exit)


func _process(_delta: float) -> void:
	if not is_instance_valid(_video_player):
		exit()
		return
	# The placeholder is swapped for the real texture on the first frame; ExoPlayer and
	# AVPlayer then keep the same ExternalTexture, so this rarely reassigns.
	var texture: Texture2D = _video_player.get_backend_texture()
	if texture != _texture_rect.texture:
		_texture_rect.texture = texture
		_update_video_panel_size()


func _gui_input(event: InputEvent) -> void:
	if event.is_action_pressed("ui_cancel"):
		exit()
		accept_event()


## `video_player` is the screen's DclVideoPlayer; any Node with get_backend_texture() works.
func enter(video_player: Node) -> void:
	# The camera cannot move while focused, so a repeat is the same screen: a pointer press
	# that lands on the overlay (desktop click, joypad button) must not re-open it.
	if visible:
		return
	_video_player = video_player
	_texture_rect.texture = null
	# The HUD chat would otherwise keep its own write mode and keyboard under the overlay.
	Global.close_chat.emit()
	show()
	set_process(true)
	grab_focus()
	_spawn_chat()
	_chat_collapsed = false
	_apply_orientation(Global.is_orientation_portrait())
	_show_exit_label()
	_update_members_count()
	Global.avatars.avatar_added.connect(_on_avatars_changed)
	Global.avatars.avatar_removed.connect(_on_avatars_changed)


func exit() -> void:
	if not visible:
		return
	hide()
	set_process(false)
	_kill_exit_label_tween()
	Global.avatars.avatar_added.disconnect(_on_avatars_changed)
	Global.avatars.avatar_removed.disconnect(_on_avatars_changed)
	if _chat != null:
		_chat.close_write_mode_if_active()
		_chat.queue_free()
		_chat = null
	_chat_reveal_epoch += 1
	_video_player = null
	_texture_rect.texture = null
	_keyboard_margin.custom_minimum_size.y = 0
	_chat_header.visible = true
	# The world HUD is landscape-only on mobile, like the HUD chat on close.
	if Global.is_orientation_portrait():
		Global.set_orientation_landscape()
	Global.explorer_grab_focus()


func _spawn_chat() -> void:
	_chat = CHAT_SCENE.instantiate()
	_chat.show_welcome_message = false
	_chat.hide_header = true
	_chat.on_enter_write_mode.connect(_on_chat_enter_write_mode)
	_chat.on_exit_write_mode.connect(_on_chat_exit_write_mode)
	_chat.submit_message.connect(_on_chat_submit_message)
	_chat_slot.add_child(_chat)
	# The column paints the chat's background down to the screen edge, so the chat's own
	# translucent panel would leave the safe-area band under the input a different shade.
	_chat.get_node("%PanelContainer_Chat").add_theme_stylebox_override("panel", StyleBoxEmpty.new())
	# Messages grow up from the input, as in the design, instead of hanging from the header.
	# EXPAND keeps the ScrollContainer sizing the list to its own height; SHRINK_END then
	# places a shorter list at the bottom of that height.
	_chat.v_box_container_chat.size_flags_vertical = (Control.SIZE_EXPAND | Control.SIZE_SHRINK_END)
	# Invisible until the first layout settles: chat.gd hides itself one deferred call after
	# _ready, then re-shows, and the replayed messages re-measure over a few frames. Drawn
	# meanwhile, that reads as the text flickering. _scroll_chat_to_bottom reveals it.
	_chat.modulate = Color.TRANSPARENT
	var explorer = Global.get_explorer()
	if explorer != null:
		_chat.copy_messages_from(explorer.chat_panel.chat)
	_chat.async_start_chat.call_deferred()


func _apply_orientation(is_portrait: bool) -> void:
	_layout.vertical = is_portrait
	_apply_chat_layout()
	_refresh_chat()


## Re-measures the docked chat for the column it is in now. Message widths are measured a
## few frames after this call, so it runs after the column is visible and sized, never while
## it is hidden, or the messages keep a stale width and overflow when the column comes back.
func _refresh_chat() -> void:
	if _chat == null or not _chat_column.visible or not _chat_slot.visible:
		return
	_chat.apply_orientation(_layout.vertical)
	# The docked column is narrow in both orientations, so the chat always uses its
	# full-width (portrait) layout; apply_orientation only sets the sizes and fonts.
	_chat.set_layout_portrait()


## Lays the chat out for the orientation and the collapsed state. Landscape: a fixed-width
## column, or nothing (the video takes the whole screen and an edge tab brings it back).
## Portrait: the chat fills under the video, or shrinks to its header and the video takes the rest.
func _apply_chat_layout() -> void:
	var is_portrait := _layout.vertical
	# Exit and rotate follow the HUD chat bar's discs: 66 px in landscape, 96 px in portrait.
	var preset := HudButton.Size.LARGE if is_portrait else HudButton.Size.SMALL
	_button_rotate.size_preset = preset
	var spec: Vector2i = HudButton.SIZES[preset]
	var exit_px: int = spec.x
	_button_exit.custom_minimum_size = Vector2(exit_px, exit_px)
	_button_exit.add_theme_constant_override(
		"icon_max_width", EXIT_ICON_PORTRAIT if is_portrait else EXIT_ICON_LANDSCAPE
	)
	# A right content margin of twice the shift moves the centred glyph left by the shift.
	var exit_shift: int = EXIT_ICON_SHIFT_PORTRAIT if is_portrait else EXIT_ICON_SHIFT_LANDSCAPE
	for slot in [&"normal", &"pressed"]:
		var box := _button_exit.get_theme_stylebox(slot) as StyleBoxFlat
		box.set_corner_radius_all(roundi(exit_px * 0.5))
		box.content_margin_right = exit_shift * 2
	var header_box := _chat_header.get_theme_stylebox("panel") as StyleBoxFlat
	var padding_y: int = HEADER_PADDING_Y_PORTRAIT if is_portrait else HEADER_PADDING_Y_LANDSCAPE
	header_box.content_margin_top = padding_y
	header_box.content_margin_bottom = padding_y
	_chat_header.custom_minimum_size.y = (
		HEADER_HEIGHT_PORTRAIT if is_portrait else HEADER_HEIGHT_LANDSCAPE
	)
	var chevron_px: int = CHEVRON_SIZE_PORTRAIT if is_portrait else CHEVRON_SIZE_LANDSCAPE
	# The slot grows by the gap and the chevron sits at its right end; the header row's own
	# separation already counts toward the gap.
	var gap: int = CHEVRON_GAP_PORTRAIT if is_portrait else CHEVRON_GAP_LANDSCAPE
	var lead: int = maxi(gap - _collapse_slot.get_parent().get_theme_constant("separation"), 0)
	_collapse_slot.custom_minimum_size = Vector2(lead + chevron_px, chevron_px)
	_button_collapse_chat.position = Vector2(lead, 0)
	_button_collapse_chat.size = Vector2(chevron_px, chevron_px)
	_button_collapse_chat.icon = CHEVRON_ICON_PORTRAIT if is_portrait else CHEVRON_ICON_LANDSCAPE
	_button_collapse_chat.add_theme_constant_override("icon_max_width", chevron_px)
	_label_exit.custom_minimum_size = Vector2(0, exit_px)
	var header_font: int = HEADER_FONT_SIZE_PORTRAIT if is_portrait else HEADER_FONT_SIZE_LANDSCAPE
	_label_nearby.label_settings.font_size = header_font
	_label_members.label_settings.font_size = header_font
	var icon_px: int = MEMBERS_ICON_PORTRAIT if is_portrait else MEMBERS_ICON_LANDSCAPE
	_members_icon.custom_minimum_size = Vector2(icon_px, icon_px)
	_button_expand_chat.visible = _chat_collapsed and not is_portrait
	_chat_slot.visible = not (_chat_collapsed and is_portrait)
	if is_portrait:
		_chat_column.visible = true
		if _chat_collapsed:
			_chat_column.custom_minimum_size = Vector2.ZERO
			_chat_column.size_flags_vertical = Control.SIZE_FILL
			_video_panel.size_flags_vertical = Control.SIZE_EXPAND_FILL
		else:
			_chat_column.custom_minimum_size = Vector2.ZERO
			_chat_column.size_flags_vertical = Control.SIZE_EXPAND_FILL
			_video_panel.size_flags_vertical = Control.SIZE_FILL
		# Down to minimize, up to bring the chat back.
		_button_collapse_chat.pivot_offset = _button_collapse_chat.size * 0.5
		_button_collapse_chat.rotation_degrees = 180.0 if _chat_collapsed else 0.0
	else:
		_chat_column.visible = not _chat_collapsed
		_chat_column.custom_minimum_size = Vector2(LANDSCAPE_CHAT_WIDTH, 0)
		_chat_column.size_flags_vertical = Control.SIZE_EXPAND_FILL
		_video_panel.size_flags_vertical = Control.SIZE_FILL
		# Points at the edge the chat slides out to; the edge tab points back in.
		_button_collapse_chat.pivot_offset = _button_collapse_chat.size * 0.5
		_button_collapse_chat.rotation_degrees = -90.0
		_button_expand_chat.pivot_offset = _button_expand_chat.size * 0.5
		_button_expand_chat.rotation_degrees = 90.0
	_update_video_panel_size()
	_apply_side_insets()
	if _chat != null:
		_chat.scroll_to_bottom_deferred()
		_chat_reveal_epoch += 1
		var settle := get_tree().create_timer(SCROLL_SETTLE_SECONDS)
		settle.timeout.connect(_scroll_chat_to_bottom.bind(_chat_reveal_epoch))


func _scroll_chat_to_bottom(epoch: int) -> void:
	if epoch != _chat_reveal_epoch or _chat == null or not is_instance_valid(_chat):
		return
	_chat.scroll_to_bottom_deferred()
	_chat.modulate = Color.WHITE


## Keeps the buttons, the chat header and the chat clear of the device's side insets (camera
## cutout, curved edges) while the video and the chat background stay full-bleed. Only the
## edges that touch the screen take an inset: in landscape the chat column is on the right.
func _apply_side_insets() -> void:
	if not visible:
		return
	var left := 0
	var right := 0
	if Global.is_mobile() or Global.is_emulating_safe_area():
		# Scaled from window to viewport pixels like SafeMarginContainer.
		var safe_area: Rect2i = Global.get_safe_area()
		var window_width := float(maxi(DisplayServer.window_get_size().x, 1))
		var x_factor := get_viewport().get_visible_rect().size.x / window_width
		left = maxi(roundi(safe_area.position.x * x_factor), 0)
		right = maxi(roundi((window_width - safe_area.end.x) * x_factor), 0)
	_inset_left = left
	var is_portrait := _layout.vertical
	var controls_side: int = (
		CONTROLS_SIDE_MARGIN_PORTRAIT if is_portrait else CONTROLS_SIDE_MARGIN_LANDSCAPE
	)
	var video_reaches_right := is_portrait or _chat_collapsed
	_controls_margin.add_theme_constant_override("margin_left", controls_side + left)
	_controls_margin.add_theme_constant_override(
		"margin_right", controls_side + (right if video_reaches_right else 0)
	)
	var chat_left: int = left if is_portrait else 0
	var header_box := _chat_header.get_theme_stylebox("panel") as StyleBoxFlat
	var header_side: int = HEADER_PADDING_X_PORTRAIT if is_portrait else HEADER_PADDING_X_LANDSCAPE
	header_box.content_margin_left = header_side + chat_left
	header_box.content_margin_right = header_side + right
	if _chat != null:
		_chat.offset_left = chat_left
		_chat.offset_right = -right
	_button_expand_chat.offset_right = -(EXPAND_TAB_EDGE_MARGIN + right)
	_button_expand_chat.offset_left = (
		_button_expand_chat.offset_right - _button_expand_chat.custom_minimum_size.x
	)


## In portrait the video sits under the exit/rotate row and takes the stream's aspect over
## the full width (or the whole remaining height when the chat is collapsed); in landscape
## the row floats over the video, which takes whatever the chat column leaves.
func _update_video_panel_size() -> void:
	var controls_band := 0.0
	if _layout.vertical:
		controls_band = (
			_controls_margin.get_theme_constant("margin_top") * 2
			+ _button_exit.custom_minimum_size.y
		)
	_texture_rect.offset_top = controls_band
	if not _layout.vertical or _chat_collapsed:
		_video_panel.custom_minimum_size = Vector2.ZERO
		return
	var aspect := DEFAULT_VIDEO_ASPECT
	var texture := _texture_rect.texture
	if (
		texture != null
		and texture.get_width() > PLACEHOLDER_MAX_SIZE
		and texture.get_height() > PLACEHOLDER_MAX_SIZE
	):
		aspect = float(texture.get_height()) / float(texture.get_width())
	_video_panel.custom_minimum_size = Vector2(0, roundf(size.x * aspect + controls_band))


## The caption beside the exit button, shown for a few seconds on every entry.
func _show_exit_label() -> void:
	_kill_exit_label_tween()
	_label_exit.show()
	_exit_label_tween = create_tween()
	_exit_label_tween.tween_interval(EXIT_LABEL_SECONDS)
	_exit_label_tween.tween_callback(_label_exit.hide)


func _kill_exit_label_tween() -> void:
	if _exit_label_tween != null and _exit_label_tween.is_valid():
		_exit_label_tween.kill()
	_exit_label_tween = null


func _update_members_count() -> void:
	# The same set the Nearby list shows (social_list.gd): loaded avatars with an address,
	# minus blocked users, plus the player.
	var count := 1
	for avatar in Global.avatars.get_avatars():
		if not avatar is Avatar or avatar.avatar_id.is_empty():
			continue
		if Global.social_blacklist and Global.social_blacklist.is_blocked(avatar.avatar_id):
			continue
		count += 1
	_label_members.text = str(count)


func _on_avatars_changed(_avatar_or_address: Variant) -> void:
	_update_members_count()


func _on_button_collapse_chat_pressed() -> void:
	Global.send_haptic_feedback()
	if _chat != null:
		_chat.close_write_mode_if_active()
	_chat_collapsed = not _chat_collapsed
	_apply_chat_layout()
	_refresh_chat()


func _on_button_rotate_pressed() -> void:
	Global.send_haptic_feedback()
	if Global.is_orientation_portrait():
		Global.set_orientation_landscape()
	else:
		Global.set_orientation_portrait()


func _on_orientation_changed(is_portrait: bool) -> void:
	if visible:
		_apply_orientation(is_portrait)


## In landscape the input stretches left over the video so it spans the keyboard, like the
## HUD chat's. A negative margin moves it without reparenting, which would drop its focus
## and with it the keyboard.
func _on_chat_enter_write_mode() -> void:
	if _layout.vertical:
		return
	var column_left: float = _chat.global_position.x - global_position.x
	var input_row: MarginContainer = _chat.get_node("%MarginContainer_LineEditSafeArea")
	input_row.add_theme_constant_override("margin_left", -roundi(column_left - _inset_left))
	# No room kept for the navigation bar: it stays hidden while typing here.
	input_row.add_theme_constant_override("margin_right", 0)


func _on_chat_exit_write_mode() -> void:
	_chat.reset_safe_area_insets()
	# chat.gd hands focus back to the explorer root (deferred), which re-enables movement
	# under the overlay; this is queued after it and takes the focus back.
	_grab_focus_if_active.call_deferred()


func _grab_focus_if_active() -> void:
	if visible and is_inside_tree():
		grab_focus()


func _on_chat_submit_message(message: String) -> void:
	# Same route as the HUD chat: explorer.tscn connects the panel's signal to ChatCommands.
	var explorer = Global.get_explorer()
	if explorer != null:
		explorer.chat_panel.submit_message.emit(message)


func _on_change_virtual_keyboard(virtual_keyboard_height: int) -> void:
	if not visible:
		return
	# The overlay spans the whole window, so the reported height (which on iOS includes the
	# bottom inset) is the amount to lift the chat by, minus the bottom margin the chat
	# column already keeps. Scaled like chat_panel.gd.
	var window_size: Vector2i = DisplayServer.window_get_size()
	var viewport_size: Vector2 = get_viewport().get_visible_rect().size
	var y_factor: float = viewport_size.y / max(float(window_size.y), 1.0)
	var safe_bottom: float = float(_chat_bottom_inset.get_theme_constant("margin_bottom"))
	_keyboard_margin.custom_minimum_size.y = ceil(
		max(float(virtual_keyboard_height) * y_factor - safe_bottom, 0.0)
	)
	# Landscape leaves the chat column about a hundred pixels tall over the keyboard: the
	# header would be drawn under the input, so it steps aside while typing.
	_chat_header.visible = _layout.vertical or virtual_keyboard_height <= 0
	if _chat == null:
		return
	if virtual_keyboard_height <= 0:
		_chat.close_write_mode_if_active()
	else:
		_chat.scroll_to_bottom_deferred()
		if not _layout.vertical:
			_hide_android_navigation_bar()


## Android brings the navigation bar back with the keyboard even in immersive mode; in
## landscape it would sit beside the full-width input. WindowInsetsController needs API 30.
func _hide_android_navigation_bar() -> void:
	if OS.get_name() != "Android" or not Engine.has_singleton("AndroidRuntime"):
		return
	var android_runtime: Object = Engine.get_singleton("AndroidRuntime")
	var activity: Object = android_runtime.getActivity()
	# Window calls must run on the Android UI thread.
	var hide_bar := func():
		var controller: Object = activity.getWindow().getInsetsController()
		if controller == null:
			return
		var insets_type: Object = JavaClassWrapper.wrap("android.view.WindowInsets$Type")
		controller.hide(insets_type.navigationBars())
	activity.runOnUiThread(android_runtime.createRunnableFromGodotCallable(hide_bar))
