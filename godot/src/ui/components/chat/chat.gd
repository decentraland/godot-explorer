extends Control

signal submit_message(message: String)
signal on_exit_chat
signal on_open_chat
signal on_enter_write_mode
signal on_exit_write_mode

const MAX_AUTOCOMPLETE_RESULTS: int = 3
const AUTOCOMPLETE_ITEM_STRIDE: float = 58.0
const AUTOCOMPLETE_SCROLL_PADDING: float = 14.0

## Design specs — header landscape
const HEADER_HEIGHT_LANDSCAPE := 53
const HEADER_SEPARATION_LANDSCAPE := 8
const HEADER_ICON_LANDSCAPE := 29
const HEADER_FONT_LANDSCAPE := 17
const HEADER_LABEL_LANDSCAPE := "NEARBY"

## Design specs — header portrait
const HEADER_HEIGHT_PORTRAIT := 66
const HEADER_SEPARATION_PORTRAIT := 12
const HEADER_ICON_PORTRAIT := 44
const HEADER_FONT_PORTRAIT := 27
const HEADER_LABEL_PORTRAIT := "Nearby"

## Design specs — write button landscape
const WRITE_HEIGHT_LANDSCAPE := 48
const WRITE_FONT_LANDSCAPE := 21
const WRITE_PADDING_LANDSCAPE := 10

## Design specs — write button portrait
const WRITE_HEIGHT_PORTRAIT := 66
const WRITE_FONT_PORTRAIT := 29
const WRITE_PADDING_PORTRAIT := 16

var scrolled: bool = false
var new_messages_count: int = 0
var _mention_item_scene: PackedScene
var _suppress_autocomplete: bool = false
var _autocomplete_queued: bool = false

@onready var panel_line_edit: PanelContainer = %PanelLineEdit
@onready var line_edit_command: LineEdit = %LineEdit_Command
@onready var v_box_container_chat: VBoxContainer = %VBoxContainerChat
@onready var scroll_container_chats_list: ScrollContainer = %ScrollContainer_ChatsList
@onready var button_go_to_last: Button = %Button_GoToLast
@onready var panel_container_new_messages: PanelContainer = %PanelContainer_NewMessages
@onready var label_new_messages: Label = %Label_NewMessages
@onready var button_send: Button = %Button_Send
@onready var button_write: Button = %Button_Write
@onready var panel_messages: PanelContainer = $VBoxContainer/HBoxContainer/PanelContainer
@onready var column_go_to_last: Control = $VBoxContainer/HBoxContainer/VSeparator

@onready var _line_edit_safe_area: MarginContainer = %MarginContainer_LineEditSafeArea
@onready var _header: PanelContainer = %PanelContainer_Header
@onready var _header_hbox: HBoxContainer = %PanelContainer_Header/HBoxContainer
@onready var _header_icon: TextureRect = %PanelContainer_Header/HBoxContainer/TextureRect
@onready var _header_label: Label = %PanelContainer_Header/HBoxContainer/Label
@onready var _autocomplete_panel: PanelContainer = %AutocompletePanel
@onready var _autocomplete_scroll: ScrollContainer = %AutocompleteScroll
@onready var _autocomplete_container: VBoxContainer = %AutocompleteItems


func _ready():
	Global.on_chat_message.connect(self._on_chat_message_arrived)
	Global.change_virtual_keyboard.connect(self._async_on_change_virtual_keyboard)
	Global.orientation_changed.connect(_on_orientation_changed)
	submit_message.connect(self._on_submit_message)

	exit_chat.call_deferred()
	button_go_to_last.hide()
	_apply_system_bar_insets()

	scroll_container_chats_list.get_v_scroll_bar().scrolling.connect(
		self._on_chat_scrollbar_scrolling
	)

	async_show_welcome_message.call_deferred()
	button_send.disabled = true
	_setup_autocomplete()


func async_show_welcome_message() -> void:
	await Global.loading_finished
	Global.on_chat_message.emit(
		"system",
		"[color=#cfc][b]Welcome to Decentraland! Respect others and have fun.[/b][/color]",
		Time.get_unix_time_from_system()
	)


func _on_submit_message(message: String):
	if !message.is_empty():
		var is_command: bool = message.begins_with("/")
		var is_mention: bool = message.contains("@")
		Global.metrics.track_chat_message_sent(
			message.length(), "nearby", false, is_mention, is_command, "", "CHAT"
		)
		UiSounds.play_sound("widget_chat_message_private_send")


func _scroll_to_bottom() -> void:
	if not scroll_container_chats_list:
		return

	new_messages_count = 0
	panel_container_new_messages.hide()
	var scrollbar = scroll_container_chats_list.get_v_scroll_bar()
	if scrollbar:
		var target_scroll: float = max(scrollbar.max_value - scrollbar.page, 0.0)
		scroll_container_chats_list.set_v_scroll(target_scroll)
		scrolled = false
		button_go_to_last.hide()
		_async_scroll_to_bottom_after_layout.call_deferred()


func _async_scroll_to_bottom_after_layout() -> void:
	await get_tree().process_frame
	if not scroll_container_chats_list or not is_instance_valid(scroll_container_chats_list):
		return

	var scrollbar = scroll_container_chats_list.get_v_scroll_bar()
	if not scrollbar or not is_instance_valid(scrollbar):
		return

	var target_scroll: float = max(scrollbar.max_value - scrollbar.page, 0.0)
	scroll_container_chats_list.set_v_scroll(target_scroll)
	scrolled = false
	button_go_to_last.hide()


func _on_button_send_pressed():
	_hide_autocomplete()
	var message = line_edit_command.text
	submit_message.emit(message)
	line_edit_command.text = ""
	button_send.disabled = true

	_scroll_to_bottom()
	if message.begins_with("/") or Global.get_config().submit_message_closes_chat:
		exit_chat()
	else:
		line_edit_command.grab_focus()


func _on_button_write_pressed():
	button_write.hide()
	panel_line_edit.show()
	line_edit_command.text = ""
	button_send.disabled = true
	line_edit_command.grab_focus()
	DisplayServer.virtual_keyboard_show("")
	if not Global.is_orientation_portrait():
		_header.hide()
	on_enter_write_mode.emit()
	_relayout_all_messages()


func _on_line_edit_command_text_submitted(new_text):
	_hide_autocomplete()
	submit_message.emit(new_text)
	line_edit_command.text = ""
	button_send.disabled = true
	_scroll_to_bottom()
	if new_text.begins_with("/") or Global.get_config().submit_message_closes_chat:
		exit_chat()
	else:
		line_edit_command.grab_focus()


func _close_write_mode() -> void:
	_hide_autocomplete()
	panel_line_edit.hide()
	button_write.show()
	_header.show()
	if Global.is_mobile():
		DisplayServer.virtual_keyboard_hide()
	on_exit_write_mode.emit()
	_relayout_all_messages()


func exit_chat() -> void:
	_close_write_mode()
	hide()
	on_exit_chat.emit()


func async_start_chat():
	show()
	panel_line_edit.hide()
	on_open_chat.emit()
	# Re-layout all messages now that we have a valid width
	_relayout_all_messages()
	if !scrolled:
		await get_tree().process_frame
		_scroll_to_bottom()


func _relayout_all_messages() -> void:
	var is_portrait := Global.is_orientation_portrait()
	for child in v_box_container_chat.get_children():
		if child.has_method("set_portrait"):
			child.set_portrait(is_portrait)
		if child.has_method("_update_layout"):
			child._update_layout.call_deferred()


func _deferred_relayout_all_messages() -> void:
	await get_tree().process_frame
	_relayout_all_messages()


func _on_chat_message_arrived(address: String, message: String, timestamp: float):
	var new_chat = Global.preload_assets.CHAT_MESSAGE.instantiate()
	v_box_container_chat.add_child(new_chat)
	new_chat.reduce_text = false
	if Global.is_orientation_portrait():
		new_chat.set_portrait(true)
	new_chat.set_chat(address, message, timestamp)

	if !scrolled:
		_scroll_to_bottom()
	else:
		new_messages_count = new_messages_count + 1
		panel_container_new_messages.show()
		label_new_messages.text = str(new_messages_count)


func is_at_bottom() -> bool:
	if not scroll_container_chats_list or not is_instance_valid(scroll_container_chats_list):
		return true  # Consider it "at bottom" if container doesn't exist

	var scrollbar = scroll_container_chats_list.get_v_scroll_bar()
	if not scrollbar or not is_instance_valid(scrollbar):
		return true

	# Works even if the scrollbar is set to "never show".
	var max_scroll: float = max(scrollbar.max_value - scrollbar.page, 0.0)
	if max_scroll <= 0.0:
		return true

	# Check if at bottom with small tolerance
	var tolerance = 5.0
	return scrollbar.value >= max_scroll - tolerance


func _on_chat_scrollbar_scrolling() -> void:
	scrolled = !is_at_bottom()
	button_go_to_last.visible = scrolled


func _on_button_go_to_last_pressed() -> void:
	_scroll_to_bottom()


func _async_on_change_virtual_keyboard(keyboard_height: int) -> void:
	if keyboard_height <= 0:
		if panel_line_edit.visible:
			_close_write_mode()
		_line_edit_safe_area.add_theme_constant_override("margin_right", 0)
		_line_edit_safe_area.add_theme_constant_override("margin_left", 0)
		return
	_apply_system_bar_insets()
	await get_tree().process_frame
	_scroll_to_bottom()


func _apply_system_bar_insets() -> void:
	if not OS.get_name() == "Android":
		return
	await get_tree().process_frame
	var insets := _get_android_system_bar_insets()
	var win_size := DisplayServer.window_get_size()
	var viewport_size := get_viewport().get_visible_rect().size
	var x_factor: float = viewport_size.x / max(float(win_size.x), 1.0)
	var nav_right: int = int(ceil(float(insets.get("right", 0)) * x_factor))
	var nav_left: int = int(ceil(float(insets.get("left", 0)) * x_factor))
	_line_edit_safe_area.add_theme_constant_override("margin_right", nav_right)
	_line_edit_safe_area.add_theme_constant_override("margin_left", nav_left)


func _get_android_system_bar_insets() -> Dictionary:
	var result := {"left": 0, "top": 0, "right": 0, "bottom": 0}
	if not Engine.has_singleton("AndroidRuntime"):
		return result
	var android_runtime: Object = Engine.get_singleton("AndroidRuntime")
	var activity: Object = android_runtime.getActivity()
	var window: Object = activity.getWindow()
	var window_insets_types: Object = JavaClassWrapper.wrap("android.view.WindowInsets$Type")
	var root_insets: Object = window.getDecorView().getRootWindowInsets()
	var nav_bars: int = window_insets_types.navigationBars()
	var insets_ignoring: Object = root_insets.getInsetsIgnoringVisibility(nav_bars)
	var insets_str: String = insets_ignoring.toString()
	var regex := RegEx.new()
	regex.compile("(\\w+)=(\\d+)")
	for m in regex.search_all(insets_str):
		result[m.get_string(1)] = int(m.get_string(2))
	return result


func _on_line_edit_command_focus_exited() -> void:
	_close_write_mode()


func _on_line_edit_command_text_changed(new_text: String) -> void:
	button_send.disabled = new_text.length() == 0
	if _suppress_autocomplete:
		_suppress_autocomplete = false
		return
	# Deferred so caret_column is fully updated before we read it.
	# Only queue one call per frame to avoid rapid node churn on fast typing/deleting.
	if not _autocomplete_queued:
		_autocomplete_queued = true
		_update_autocomplete.call_deferred()


# region Layout


## Reading mode: panel_messages fixed width, separator expands
func set_layout_reading(panel_width: int) -> void:
	panel_messages.custom_minimum_size.x = panel_width
	panel_messages.size_flags_horizontal = Control.SIZE_SHRINK_BEGIN
	column_go_to_last.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	column_go_to_last.show()


## Writing mode: both panel_messages and separator expand 50/50
func set_layout_writing() -> void:
	panel_messages.custom_minimum_size.x = 0
	panel_messages.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	column_go_to_last.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	column_go_to_last.show()


## Portrait mode: panel_messages fills all, separator hidden
func set_layout_portrait() -> void:
	panel_messages.custom_minimum_size.x = 0
	panel_messages.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	column_go_to_last.hide()


func _on_orientation_changed(is_portrait: bool) -> void:
	if is_portrait:
		_header.custom_minimum_size.y = HEADER_HEIGHT_PORTRAIT
		_header_hbox.add_theme_constant_override("separation", HEADER_SEPARATION_PORTRAIT)
		_header_icon.custom_minimum_size = Vector2(HEADER_ICON_PORTRAIT, HEADER_ICON_PORTRAIT)
		_header_label.text = HEADER_LABEL_PORTRAIT
		_header_label.label_settings.font_size = HEADER_FONT_PORTRAIT
		button_write.custom_minimum_size.y = WRITE_HEIGHT_PORTRAIT
		button_write.add_theme_font_size_override("font_size", WRITE_FONT_PORTRAIT)
		_set_button_write_padding(WRITE_PADDING_PORTRAIT)
	else:
		_header.custom_minimum_size.y = HEADER_HEIGHT_LANDSCAPE
		_header_hbox.add_theme_constant_override("separation", HEADER_SEPARATION_LANDSCAPE)
		_header_icon.custom_minimum_size = Vector2(HEADER_ICON_LANDSCAPE, HEADER_ICON_LANDSCAPE)
		_header_label.text = HEADER_LABEL_LANDSCAPE
		_header_label.label_settings.font_size = HEADER_FONT_LANDSCAPE
		button_write.custom_minimum_size.y = WRITE_HEIGHT_LANDSCAPE
		button_write.add_theme_font_size_override("font_size", WRITE_FONT_LANDSCAPE)
		_set_button_write_padding(WRITE_PADDING_LANDSCAPE)
	if panel_line_edit.visible:
		_close_write_mode()
	_apply_system_bar_insets()
	_deferred_relayout_all_messages()


func _set_button_write_padding(left: int) -> void:
	var style: StyleBoxFlat = button_write.get_theme_stylebox("normal").duplicate()
	style.content_margin_left = left
	for state in [
		"normal",
		"pressed",
		"hover",
		"hover_pressed",
		"disabled",
		"focus",
		"normal_mirrored",
		"pressed_mirrored",
		"hover_mirrored",
		"hover_pressed_mirrored",
		"disabled_mirrored"
	]:
		button_write.add_theme_stylebox_override(state, style)


# endregion

# region Mention Autocomplete


func _setup_autocomplete() -> void:
	_mention_item_scene = load("res://src/ui/components/chat/mention_item.tscn")


## Returns the partial name being typed after @, or null if not currently typing a mention.
func _get_mention_query():
	var text: String = line_edit_command.text
	var caret: int = line_edit_command.caret_column
	var before_caret: String = text.substr(0, caret)

	var at_pos: int = before_caret.rfind("@")
	if at_pos == -1:
		return null

	# @ must be at start of text or preceded by a space
	if at_pos > 0 and before_caret[at_pos - 1] != " ":
		return null

	var query: String = before_caret.substr(at_pos + 1)
	# If there's a space in the query portion, the mention is already finished
	if query.contains(" "):
		return null

	return query


func _get_matching_avatars(query: String) -> Array:
	var results: Array = []
	if not Global.avatars:
		return results

	var avatars = Global.avatars.get_avatars()

	for avatar in avatars:
		if not avatar is Avatar:
			continue
		var avatar_name: String = avatar.get_avatar_name()
		if avatar_name.is_empty():
			continue
		if query.is_empty() or avatar_name.containsn(query):
			results.append(avatar)

	results.sort_custom(
		func(a, b): return a.get_avatar_name().nocasecmp_to(b.get_avatar_name()) < 0
	)
	return results


func _update_autocomplete() -> void:
	_autocomplete_queued = false
	var query = _get_mention_query()
	if query == null:
		_hide_autocomplete()
		return

	var avatars: Array = _get_matching_avatars(query)
	if avatars.is_empty():
		_hide_autocomplete()
		return

	_show_autocomplete(avatars)


func _show_autocomplete(avatars: Array) -> void:
	# Clear previous items immediately to avoid ghost nodes
	for child in _autocomplete_container.get_children():
		_autocomplete_container.remove_child(child)
		child.queue_free()

	for avatar in avatars:
		var item = _mention_item_scene.instantiate()
		_autocomplete_container.add_child(item)
		item.setup(avatar)
		item.mention_selected.connect(_on_autocomplete_item_pressed)

	_resize_autocomplete_scroll(avatars.size())
	_autocomplete_panel.visible = true
	_autocomplete_scroll.scroll_vertical = 0


func _hide_autocomplete() -> void:
	if not _autocomplete_panel:
		return
	_autocomplete_panel.visible = false
	for child in _autocomplete_container.get_children():
		_autocomplete_container.remove_child(child)
		child.queue_free()


func _resize_autocomplete_scroll(item_count: int) -> void:
	var visible_count: int = mini(item_count, MAX_AUTOCOMPLETE_RESULTS)
	var scroll_height: float = (
		visible_count * AUTOCOMPLETE_ITEM_STRIDE + AUTOCOMPLETE_SCROLL_PADDING
	)
	_autocomplete_scroll.custom_minimum_size.y = scroll_height


func _on_autocomplete_item_pressed(avatar_name: String) -> void:
	var caret: int = line_edit_command.caret_column
	var text: String = line_edit_command.text
	var before_caret: String = text.substr(0, caret)

	var at_pos: int = before_caret.rfind("@")
	if at_pos == -1:
		_hide_autocomplete()
		return

	var mention: String = "@" + avatar_name + " "
	var new_text: String = text.substr(0, at_pos) + mention + text.substr(caret)
	var new_caret: int = at_pos + mention.length()

	_hide_autocomplete()
	_suppress_autocomplete = true
	line_edit_command.text = new_text
	line_edit_command.caret_column = new_caret
	button_send.disabled = new_text.is_empty()
	# Sync the OS keyboard/IME text buffer with the new content, otherwise
	# backspace won't work on programmatically inserted text because the OS
	# still holds the old buffer. This is what a tap on the LineEdit triggers.
	if DisplayServer.has_feature(DisplayServer.FEATURE_VIRTUAL_KEYBOARD):
		DisplayServer.virtual_keyboard_show(
			new_text,
			Rect2i(),
			DisplayServer.KEYBOARD_TYPE_DEFAULT,
			line_edit_command.max_length,
			new_caret,
			new_caret
		)

# endregion
