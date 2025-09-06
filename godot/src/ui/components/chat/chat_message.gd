@tool
extends Control

const EMOTE: String = "␐"
const REQUEST_PING: String = "␑"
const ACK: String = "␆"
const MY_MENTION_COLOR = "#FC03EC"
const URL_COLOR = "#66B3FF"
const MENTION_COLOR = "#FFD700"

@export var compact_view := false:
	set(value):
		compact_view = value
		_update_compact_view()
var nickname: String = "Unknown"
var tag: String = ""
var nickname_color_hex: String = "#FFFFFF"
var is_own_message: bool = false

@onready var rich_text_label_compact_chat: RichTextLabel = %RichTextLabel_CompactChat
@onready var h_box_container_extended_chat: HBoxContainer = %HBoxContainer_ExtendedChat
@onready var h_box_container_compact_chat: HBoxContainer = %HBoxContainer_CompactChat
@onready var label_nickname: Label = %Label_Nickname
@onready var label_tag: Label = %Label_Tag
@onready var rich_text_label_message: RichTextLabel = %RichTextLabel_Message
@onready var label_timestamp: Label = %Label_Timestamp
@onready var claimed_checkmark: MarginContainer = %ClaimedCheckmark
@onready var profile_picture: ProfilePicture = %ProfilePicture
@onready var profile_picture_compact: ProfilePicture = %ProfilePicture_Compact
@onready var chat_message_notification: VBoxContainer = %ChatMessage_Notification
@onready var panel_container_extended: PanelContainer = %PanelContainer_Extended
@onready var panel_container_compact: PanelContainer = %PanelContainer_Compact


func _ready() -> void:
	Global.chat_compact_changed.connect(_on_chat_compact_changed)
	compact_view = Global.is_chat_compact

	# Connect signals for clickable URLs
	rich_text_label_message.meta_clicked.connect(_on_url_clicked)
	rich_text_label_compact_chat.meta_clicked.connect(_on_url_clicked)

	configure_link_styles()
	_update_compact_view()


func configure_link_styles() -> void:
	if rich_text_label_message:
		_configure_richtext_theme(rich_text_label_message)

	if rich_text_label_compact_chat:
		_configure_richtext_theme(rich_text_label_compact_chat)


func _configure_richtext_theme(richtext_label: RichTextLabel) -> void:
	var custom_theme = Theme.new()
	var link_style = StyleBoxFlat.new()
	link_style.bg_color = Color.TRANSPARENT
	link_style.border_width_left = 0
	link_style.border_width_right = 0
	link_style.border_width_top = 0
	link_style.border_width_bottom = 0

	custom_theme.set_stylebox("normal", "LinkButton", link_style)
	richtext_label.theme = custom_theme


func _update_compact_view() -> void:
	if (
		not h_box_container_extended_chat
		or not h_box_container_compact_chat
		or not rich_text_label_compact_chat
	):
		return

	h_box_container_extended_chat.visible = !compact_view
	h_box_container_compact_chat.visible = compact_view
	rich_text_label_compact_chat.visible = compact_view


func set_chat(chat) -> void:
	var new_text: String
	var own_address: String = Global.player_identity.get_address_str()
	var address: String = chat[0]
	is_own_message = own_address == address

	if is_own_message:
		h_box_container_extended_chat.layout_direction = Control.LAYOUT_DIRECTION_RTL
		h_box_container_compact_chat.layout_direction = Control.LAYOUT_DIRECTION_RTL
		h_box_container_compact_chat.alignment = BoxContainer.ALIGNMENT_END
	else:
		h_box_container_extended_chat.layout_direction = Control.LAYOUT_DIRECTION_LTR
		h_box_container_compact_chat.layout_direction = Control.LAYOUT_DIRECTION_LTR
		h_box_container_compact_chat.alignment = BoxContainer.ALIGNMENT_BEGIN

	var timestamp: float = chat[1]
	var message: String = chat[2]

	var datetime
	if is_own_message:
		# For own messages, use current system time
		datetime = Time.get_time_dict_from_system()
	else:
		# For other messages, convert UTC timestamp to local device time
		# The bias is in minutes and represents offset from UTC (negative for ahead of UTC)
		var timezone_info = Time.get_time_zone_from_system()
		var local_unix_time = int(timestamp) + (timezone_info.bias * 60)
		datetime = Time.get_datetime_dict_from_unix_time(local_unix_time)
	var time_string = "%02d:%02d" % [datetime.hour, datetime.minute]

	var processed_message = make_urls_clickable(message)
	new_text = ("[b][color=#fff]%s[/color]" % [processed_message])
	rich_text_label_message.text = new_text
	label_timestamp.text = time_string
	

	var avatar
	if address != "system":
		avatar = Global.avatars.get_avatar_by_address(address)

	if avatar == null:
		if address == Global.player_identity.get_address_str():
			avatar = Global.scene_runner.player_avatar_node

	if avatar != null and is_instance_valid(avatar):
		set_avatar(avatar)
	else:
		set_system_avatar()

	if message.begins_with(EMOTE):
		message = message.substr(1)  # Remove prefix
		var expression_id = message.split(" ")[0]  # Get expression id ([1] is timestamp)
		if avatar != null and is_instance_valid(avatar):
			avatar.emote_controller.async_play_emote(expression_id)
	elif message.begins_with(REQUEST_PING):
		pass  # TODO: Send ACK
	elif message.begins_with(ACK):
		pass  # TODO: Calculate ping
	else:
		Global.player_said.emit(address, message)
		var processed_message_compact = make_urls_clickable(message)

		if is_own_message:
			new_text = ("[b][color=#fff]%s[/color]" % [processed_message_compact])
			#profile_picture_compact.hide()
		else:
			new_text = (
				"[b][color=#%s]%s[/color][/b][color=#a9a9a9]%s[/color] [b][color=#fff]%s[/color]"
				% [nickname_color_hex, nickname, tag, processed_message_compact]
			)
			profile_picture_compact.show()
		rich_text_label_compact_chat.text = new_text

	async_adjust_panel_size.call_deferred()


func set_avatar(avatar: DclAvatar) -> void:
	nickname = avatar.get_avatar_name()
	var color = avatar.get_nickname_color(nickname)
	label_nickname.add_theme_color_override("font_color", color)
	nickname_color_hex = color.to_html(false) if color != null else "ffffff"

	var splitted_nickname = nickname.split("#", false)
	if splitted_nickname.size() > 1:
		nickname = splitted_nickname[0]
		label_nickname.text = nickname
		tag = "#" + splitted_nickname[1]
		label_tag.text = tag
		claimed_checkmark.hide()
	else:
		label_nickname.text = nickname
		label_tag.text = ""
		claimed_checkmark.show()

	# Update both profile pictures (extended and compact)
	profile_picture.async_update_profile_picture(avatar)
	profile_picture_compact.async_update_profile_picture(avatar)


func set_system_avatar() -> void:
	nickname = "System"

	tag = ""
	nickname_color_hex = "00ff00"

	label_nickname.text = nickname
	label_tag.text = ""
	label_nickname.add_theme_color_override("font_color", Color.GREEN)
	claimed_checkmark.hide()

	profile_picture.set_dcl_logo()
	profile_picture_compact.set_dcl_logo()


func _on_chat_compact_changed(is_compact: bool) -> void:
	compact_view = is_compact
	_update_compact_view()

	async_adjust_panel_size.call_deferred()


func make_urls_clickable(text: String) -> String:
	var processed_text = text

	# First, detect and process mentions (@Nick#TAG format)
	var mention_regex = RegEx.new()
	mention_regex.compile(r"@([^#\s]+)#([^\s]+)")

	var mention_results = mention_regex.search_all(processed_text)
	# Process mentions from end to start to maintain correct positions
	for i in range(mention_results.size() - 1, -1, -1):
		var mention_match = mention_results[i]
		print(mention_match)
		var full_mention = mention_match.get_string()
		var start_pos = mention_match.get_start()
		var end_pos = mention_match.get_end()

		# Check if this mention matches an existing avatar
		if _is_valid_mention(full_mention):
			var clickable_mention = "[url=mention:%s][color=%s]%s[/color][/url]" % [full_mention, MENTION_COLOR, full_mention]
			processed_text = (
				processed_text.substr(0, start_pos)
				+ clickable_mention
				+ processed_text.substr(end_pos)
			)
			
		if _is_mentioning_me(full_mention):
			var clickable_mention = "[color=%s]%s[/color]" % [MY_MENTION_COLOR, full_mention]
			processed_text = (
				processed_text.substr(0, start_pos)
				+ clickable_mention
				+ processed_text.substr(end_pos)
			)
			_apply_mention_style()
			

	# Then, detect and process coordinates (#,# format)
	var coord_regex = RegEx.new()
	coord_regex.compile(r"(-?\d+,-?\d+)")

	var coord_results = coord_regex.search_all(processed_text)
	# Process coordinates from end to start to maintain correct positions
	for i in range(coord_results.size() - 1, -1, -1):
		var coord_match = coord_results[i]
		var coord = coord_match.get_string()
		var start_pos = coord_match.get_start()
		var end_pos = coord_match.get_end()

		# Validate coordinate range (-150 to 150 for both x and y)
		if _is_valid_coordinate(coord):
			var clickable_coord = "[url=coord:%s][color=#66B3FF]%s[/color][/url]" % [coord, coord]
			processed_text = (
				processed_text.substr(0, start_pos)
				+ clickable_coord
				+ processed_text.substr(end_pos)
			)

	# Finally, detect and process URLs (http/https/www)
	var url_regex = RegEx.new()
	url_regex.compile(r"(https?://[^\s]+|www\.[^\s]+)")

	var url_results = url_regex.search_all(processed_text)
	# Process URLs from end to start to maintain correct positions
	for i in range(url_results.size() - 1, -1, -1):
		var url_match = url_results[i]
		var url = url_match.get_string()
		var start_pos = url_match.get_start()
		var end_pos = url_match.get_end()

		# Add https:// if URL starts with www
		var full_url = url
		if url.begins_with("www."):
			full_url = "https://" + url

		var clickable_url = "[url=%s][color=#66B3FF]%s[/color][/url]" % [URL_COLOR, full_url, url]
		processed_text = (
			processed_text.substr(0, start_pos) + clickable_url + processed_text.substr(end_pos)
		)

	return processed_text


func _is_valid_mention(mention: String) -> bool:
	# Check if this mention matches an existing avatar
	if not Global.avatars:
		return false
	
	# Remove @ from mention
	if not mention.begins_with("@"):
		return false
	
	var mention_without_at = mention.substr(1)  # Remove @
	
	var avatars = Global.avatars.get_avatars()
	for avatar in avatars:
		if avatar and avatar.has_method("get_avatar_name"):
			var avatar_name = avatar.get_avatar_name()
			if avatar_name == mention_without_at:
				return true
	return false


func _is_mentioning_me(mention: String) -> bool:
	if not mention.begins_with("@"):
		return false
	
	var mention_without_at = mention.substr(1)  # Remove @
	
	var me = Global.player_identity.get_profile_or_null()
	if not me:
		return false
		
	var my_name = me.get_name() + "#" + me.get_user_id().right(4)
	
	if my_name == mention_without_at:
		return true
	return false


func _apply_mention_style():
	var stylebox_compact = panel_container_compact.get_theme_stylebox("panel").duplicate()
	stylebox_compact.border_width_left = 2
	stylebox_compact.border_width_right = 2
	stylebox_compact.border_width_top = 2
	stylebox_compact.border_width_bottom = 2
	stylebox_compact.border_color = MY_MENTION_COLOR
	if panel_container_compact:
		panel_container_compact.add_theme_stylebox_override("panel", stylebox_compact)
	
	var stylebox_extended = panel_container_extended.get_theme_stylebox("panel").duplicate()
	stylebox_extended.border_width_left = 2
	stylebox_extended.border_width_right = 2
	stylebox_extended.border_width_top = 2
	stylebox_extended.border_width_bottom = 2
	stylebox_extended.border_color = MY_MENTION_COLOR
	if panel_container_extended:
		panel_container_extended.add_theme_stylebox_override("panel", stylebox_extended)



func _is_valid_coordinate(coord_str: String) -> bool:
	# Parse coordinates and validate range (-150 to 150)
	var coords = coord_str.split(",")
	if coords.size() != 2:
		return false

	var x = int(coords[0])
	var y = int(coords[1])

	# Check if both coordinates are within valid range
	return x >= -150 and x <= 150 and y >= -150 and y <= 150


func _on_url_clicked(meta):
	var meta_str = str(meta)
	if meta_str.begins_with("coord:"):
		var coord_str = meta_str.substr(6)
		_handle_coordinate_click(coord_str)
	elif meta_str.begins_with("mention:"):
		var mention_str = meta_str.substr(8)  # Remove "mention:" prefix
		_handle_mention_click(mention_str)
	else:
		Global.show_url_popup(meta_str)


func _handle_coordinate_click(coord_str: String):
	var coords = coord_str.split(",")
	if coords.size() == 2:
		var x = int(coords[0])
		var y = int(coords[1])
		Global.show_jump_in_popup(Vector2i(x, y))


func _handle_mention_click(mention_str: String):
	# Handle mention click (format: "@Nick#TAG")
	if not mention_str.begins_with("@"):
		return
	
	var mention_without_at = mention_str.substr(1)  # Remove @
	
	# Find the avatar that matches this mention
	if Global.avatars:
		var avatars = Global.avatars.get_avatars()
		for avatar in avatars:
			if avatar and avatar.has_method("get_avatar_name"):
				var avatar_name = avatar.get_avatar_name()
				if avatar_name == mention_without_at:
					# Show some kind of user profile or interaction
					Global.get_explorer()._async_open_profile(avatar)
					break


func async_adjust_panel_size():
	# Wait multiple frames for content to render and layout to be ready
	await get_tree().process_frame
	await get_tree().process_frame

	# Ensure parent is valid and has proper size
	if not get_parent():
		return

	# Force layout update
	get_parent().queue_redraw()
	await get_tree().process_frame

	# Get available width from parent container
	var parent_width = get_parent().size.x if get_parent().size.x > 0 else 400.0

	# Maximum panel width (leaving space for avatar and margins)
	var max_panel_width = parent_width - 100  # Avatar + margins

	if compact_view:
		# Handle compact chat sizing
		_adjust_compact_panel_size(max_panel_width)
	else:
		# Handle extended chat sizing
		_adjust_extended_panel_size(max_panel_width)


func _adjust_extended_panel_size(max_panel_width: float):
	# Calculate required width based on text for extended chat
	var font = rich_text_label_message.get_theme_default_font()
	var font_size = rich_text_label_message.get_theme_font_size("normal_font_size")
	if font_size == -1:
		font_size = 12  # default size

	var text_width = (
		font
		. get_string_size(
			rich_text_label_message.get_parsed_text(), HORIZONTAL_ALIGNMENT_LEFT, -1, font_size
		)
		. x
	)

	# Minimum and maximum width
	var min_width = 100.0
	var desired_width = max(min_width, min(text_width + 40, max_panel_width))  # +40 for internal margins

	# Set custom size
	panel_container_compact.custom_minimum_size.x = desired_width
	panel_container_extended.custom_minimum_size.x = desired_width

	# If text is too long, allow RichTextLabel to wrap
	if text_width > max_panel_width - 40:
		rich_text_label_message.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	else:
		rich_text_label_message.autowrap_mode = TextServer.AUTOWRAP_OFF


func _adjust_compact_panel_size(max_panel_width: float):
	# Calculate required width based on text for compact chat
	var font = rich_text_label_compact_chat.get_theme_default_font()
	var font_size = rich_text_label_compact_chat.get_theme_font_size("normal_font_size")
	if font_size == -1:
		font_size = 12  # default size

	var text_width = (
		font
		. get_string_size(
			rich_text_label_compact_chat.get_parsed_text(), HORIZONTAL_ALIGNMENT_LEFT, -1, font_size
		)
		. x
	)

	# Minimum and maximum width for compact mode (smaller than extended)
	var min_width = 1.0  # Smaller minimum for compact mode
	var desired_width = max(min_width, min(text_width + 20, max_panel_width))  # +20 for smaller margins

	# Set custom size for compact panel
	if panel_container_compact:
		panel_container_compact.custom_minimum_size.x = desired_width

	# If text is too long, allow RichTextLabel to wrap
	if text_width > max_panel_width - 20:
		rich_text_label_compact_chat.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	else:
		rich_text_label_compact_chat.autowrap_mode = TextServer.AUTOWRAP_OFF
