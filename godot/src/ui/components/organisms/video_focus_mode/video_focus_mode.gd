class_name VideoFocusMode
extends Control

## Full-screen video screen with the Nearby chat docked beside it (landscape) or under it
## (portrait) — issue #1381. Opens on the scene runner's open_video_focus_mode signal (the
## crosshair on a video screen, then the "Focus mode" pill) and closes from the exit button,
## a teleport or the menu.

const CHAT_SCENE := preload("res://src/ui/pages/chat/chat.tscn")
# i18n-keys: VIDEO_FOCUS_EXIT
const EXIT_LABEL_KEY := "VIDEO_FOCUS_EXIT"
## How long the exit button keeps its label before collapsing to the icon.
const EXIT_LABEL_SECONDS := 3.0
## Chat column beside the video (design: 430 of 1600 landscape px).
const LANDSCAPE_CHAT_WIDTH := 430
## Video height over width in portrait until the stream reports its own size.
const DEFAULT_VIDEO_ASPECT := 9.0 / 16.0
## get_backend_texture() hands out a 2x2 black placeholder before the first frame.
const PLACEHOLDER_MAX_SIZE := 2

var _video_player: Node = null
var _chat: Control = null
var _exit_label_tween: Tween

@onready var _layout: BoxContainer = %BoxContainer_Layout
@onready var _video_panel: Control = %Control_Video
@onready var _texture_rect: TextureRect = %TextureRect_Video
@onready var _chat_column: PanelContainer = %PanelContainer_Chat
@onready var _chat_slot: Control = %Control_ChatSlot
@onready var _label_members: Label = %Label_Members
@onready var _button_exit: Button = %Button_Exit
@onready var _button_rotate: Button = %Button_Rotate
@onready var _keyboard_margin: Control = %Control_KeyboardMargin


func _ready() -> void:
	hide()
	set_process(false)
	_button_exit.pressed.connect(exit)
	_button_rotate.pressed.connect(_on_button_rotate_pressed)
	resized.connect(_update_video_panel_size)
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
	_video_player = null
	_texture_rect.texture = null
	_keyboard_margin.custom_minimum_size.y = 0
	# The world HUD is landscape-only on mobile, like the HUD chat on close.
	if Global.is_orientation_portrait():
		Global.set_orientation_landscape()
	Global.explorer_grab_focus()


func _spawn_chat() -> void:
	_chat = CHAT_SCENE.instantiate()
	_chat.show_welcome_message = false
	_chat.hide_header = true
	_chat.on_exit_write_mode.connect(_on_chat_exit_write_mode)
	_chat.submit_message.connect(_on_chat_submit_message)
	_chat_slot.add_child(_chat)
	# Messages grow up from the input, as in the design, instead of hanging from the header.
	# EXPAND keeps the ScrollContainer sizing the list to its own height; SHRINK_END then
	# places a shorter list at the bottom of that height.
	_chat.v_box_container_chat.size_flags_vertical = (Control.SIZE_EXPAND | Control.SIZE_SHRINK_END)
	var explorer = Global.get_explorer()
	if explorer != null:
		_chat.copy_messages_from(explorer.chat_panel.chat)
	# chat.gd hides itself one deferred call after _ready; this queues behind it.
	_chat.async_start_chat.call_deferred()


func _apply_orientation(is_portrait: bool) -> void:
	_layout.vertical = is_portrait
	_chat_column.custom_minimum_size = (
		Vector2.ZERO if is_portrait else Vector2(LANDSCAPE_CHAT_WIDTH, 0)
	)
	_update_video_panel_size()
	if _chat != null:
		_chat.apply_orientation(is_portrait)
		# The docked column is narrow in both orientations, so the chat always uses its
		# full-width (portrait) layout; apply_orientation only sets the sizes and fonts.
		_chat.set_layout_portrait()


## In portrait the video panel takes the stream's aspect over the full width; in landscape it
## takes whatever the chat column leaves.
func _update_video_panel_size() -> void:
	if not _layout.vertical:
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
	_video_panel.custom_minimum_size = Vector2(0, roundf(size.x * aspect))


func _show_exit_label() -> void:
	_kill_exit_label_tween()
	_button_exit.text = EXIT_LABEL_KEY
	_exit_label_tween = create_tween()
	_exit_label_tween.tween_interval(EXIT_LABEL_SECONDS)
	_exit_label_tween.tween_callback(_collapse_exit_label)


func _collapse_exit_label() -> void:
	_button_exit.text = ""


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


func _on_button_rotate_pressed() -> void:
	Global.send_haptic_feedback()
	if Global.is_orientation_portrait():
		Global.set_orientation_landscape()
	else:
		Global.set_orientation_portrait()


func _on_orientation_changed(is_portrait: bool) -> void:
	if visible:
		_apply_orientation(is_portrait)


func _on_chat_exit_write_mode() -> void:
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
	# bottom inset) is the right amount to lift the chat by. Scaled like chat_panel.gd.
	var window_size: Vector2i = DisplayServer.window_get_size()
	var viewport_size: Vector2 = get_viewport().get_visible_rect().size
	var y_factor: float = viewport_size.y / max(float(window_size.y), 1.0)
	_keyboard_margin.custom_minimum_size.y = ceil(
		max(float(virtual_keyboard_height) * y_factor, 0.0)
	)
	if _chat == null:
		return
	if virtual_keyboard_height <= 0:
		_chat.close_write_mode_if_active()
	else:
		_chat.scroll_to_bottom_deferred()
