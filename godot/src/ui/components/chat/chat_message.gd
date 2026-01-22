@tool
extends Control

const MAX_CHARS_COMPACT_VIEW: int = 80
const MY_MENTION_COLOR = "#FC03EC"
const URL_COLOR = "#66B3FF"
const MENTION_COLOR = "#FFD700"

@export var compact_view := false:
	set(value):
		compact_view = value
		_update_compact_view()
@export var reduce_text := false
var nickname: String = "Unknown"
var tag: String = ""
var nickname_color_hex: String = "#FFFFFF"
var is_own_message: bool = false
var has_claimed_name: bool = false
var max_panel_width: int = 370

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
@onready var panel_container_extended: PanelContainer = %PanelContainer_Extended
@onready var panel_container_compact: PanelContainer = %PanelContainer_Compact
@onready var nickname_container: VBoxContainer = %NicknameContainer
@onready var nickname_container_compact: HBoxContainer = %NicknameContainerCompact
@onready var nickname_tag: HBoxContainer = %NicknameTag


func _ready() -> void:
	# Connect signals for clickable URLs
	rich_text_label_message.meta_clicked.connect(_on_url_clicked)
	rich_text_label_compact_chat.meta_clicked.connect(_on_url_clicked)

	configure_link_styles()
	async_adjust_panel_size()


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

	const MULTILINGUAL_REGULAR := preload(
		"res://assets/themes/fonts/multilanguage/multilanguage_font-Regular.tres"
	)
	const MULTILINGUAL_BOLD := preload(
		"res://assets/themes/fonts/multilanguage/multilanguage_font-Bold.tres"
	)

	richtext_label.add_theme_font_override("normal_font", MULTILINGUAL_REGULAR)
	richtext_label.add_theme_font_override("bold_font", MULTILINGUAL_BOLD)
	richtext_label.add_theme_font_override("bold_italics_font", MULTILINGUAL_BOLD)
	richtext_label.add_theme_font_override("italics_font", MULTILINGUAL_REGULAR)
	richtext_label.add_theme_font_override("mono_font", MULTILINGUAL_REGULAR)


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


func set_chat(address: String, message: String, timestamp: float) -> void:
	var new_text: String = message
	if address != "system":
		new_text = escape_bbcode(new_text)
	var own_address: String = Global.player_identity.get_address_str()
	is_own_message = own_address == address

	h_box_container_extended_chat.layout_direction = Control.LAYOUT_DIRECTION_LTR
	h_box_container_compact_chat.layout_direction = Control.LAYOUT_DIRECTION_LTR
	h_box_container_compact_chat.alignment = BoxContainer.ALIGNMENT_BEGIN

	# For other messages, convert UTC timestamp to local device time
	# The bias is in minutes and represents offset from UTC (negative for ahead of UTC)
	var timezone_info = Time.get_time_zone_from_system()
	var local_unix_time = int(timestamp) + (timezone_info.bias * 60)
	var datetime = Time.get_datetime_dict_from_unix_time(local_unix_time)
	var time_string = "%02d:%02d" % [datetime.hour, datetime.minute]

	label_timestamp.text = time_string

	if address == Global.player_identity.get_address_str():
		set_avatar(Global.scene_runner.player_avatar_node)
	elif address != "system":
		set_avatar(Global.avatars.get_avatar_by_address(address))
	else:
		set_system_avatar()

	if reduce_text and new_text.length() > MAX_CHARS_COMPACT_VIEW:
		new_text = new_text.substr(0, MAX_CHARS_COMPACT_VIEW) + "..."

	var processed_message_compact = make_urls_clickable(new_text)

	if compact_view:
		var check_mark_text = ""
		if has_claimed_name:
			check_mark_text = " [img=14]res://assets/check-mark.svg[/img]"
		new_text = (
			"[b][color=#%s]%s[/color][/b][color=#a9a9a9]%s[/color]%s: [b][color=#fff]%s[/color]"
			% [nickname_color_hex, nickname, tag, check_mark_text, processed_message_compact]
		)
		rich_text_label_compact_chat.text = new_text
	else:
		new_text = ("[b][color=#fff]%s[/color]" % [processed_message_compact])
		rich_text_label_message.text = new_text
	async_adjust_panel_size.call_deferred()


func set_avatar(avatar: DclAvatar) -> void:
	if avatar == null or !is_instance_valid(avatar):
		return
	nickname = avatar.get_avatar_name()
	var color = DclAvatar.get_nickname_color(nickname)
	label_nickname.add_theme_color_override("font_color", color)
	nickname_color_hex = color.to_html(false) if color != null else "ffffff"

	var splitted_nickname = nickname.split("#", false)
	if splitted_nickname.size() > 1:
		nickname = splitted_nickname[0]
		label_nickname.text = nickname
		tag = "#" + splitted_nickname[1]
		label_tag.text = tag
		claimed_checkmark.hide()
		has_claimed_name = false
	else:
		label_nickname.text = nickname
		label_tag.text = ""
		tag = ""
		claimed_checkmark.show()
		has_claimed_name = true

	# Update both profile pictures (extended and compact)
	var social_data: SocialItemData = SocialHelper.social_data_from_avatar(avatar)
	profile_picture.async_update_profile_picture(social_data)
	profile_picture_compact.async_update_profile_picture(social_data)


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


func make_urls_clickable(text: String) -> String:
	var processed_text = text

	# First, detect and process mentions (@Nick or @Nick#TAG format)
	var mention_regex = RegEx.new()
	# Updated regex: optional tag that must be exactly 4 hex characters if present
	mention_regex.compile(r"@([^#\s]+)(?:#([0-9a-fA-F]{4}))?")

	var mention_results = mention_regex.search_all(processed_text)
	# Process mentions from end to start to maintain correct positions
	for i in range(mention_results.size() - 1, -1, -1):
		var mention_match = mention_results[i]
		var full_mention = mention_match.get_string()
		var start_pos = mention_match.get_start()
		var end_pos = mention_match.get_end()

		# Get the username (always present)
		var username = mention_match.get_string(1)

		# Get the tag if it exists (group 2)
		var user_tag = ""
		if mention_match.get_group_count() > 1:
			var tag_match = mention_match.get_string(2)
			if tag_match != null and tag_match != "":
				user_tag = tag_match

		# For functions that need the unique identifier
		# If no tag, return @username (with @), otherwise return full mention
		var unique_name = ("@" + username) if user_tag.is_empty() else full_mention

		# Check if this mention matches an existing avatar
		if _is_valid_mention(unique_name):
			var clickable_mention = (
				"[url=mention:%s][color=%s]%s[/color][/url]"
				% [unique_name, MENTION_COLOR, full_mention]
			)
			processed_text = (
				processed_text.substr(0, start_pos)
				+ clickable_mention
				+ processed_text.substr(end_pos)
			)

		if _is_mentioning_me(unique_name):
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

	# Finally, detect and process URLs (https only for security)
	var url_regex = RegEx.new()
	# Only accept https:// URLs and www. (which will be converted to https)
	url_regex.compile(r"(https://[^\s]+|www\.[^\s]+)")

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

		# Security validation for the URL
		if _is_safe_url(full_url):
			# Sanitize the URL for display
			var sanitized_url = _sanitize_url(full_url)
			var display_url = _sanitize_for_display(url)

			var clickable_url = (
				"[url=%s][color=#66B3FF]%s[/color][/url]" % [sanitized_url, display_url]
			)
			processed_text = (
				processed_text.substr(0, start_pos) + clickable_url + processed_text.substr(end_pos)
			)

	return processed_text


# Security function to validate URLs
func _is_safe_url(url: String) -> bool:
	# Only allow HTTPS URLs
	if not url.begins_with("https://"):
		return false

	# Check for common XSS patterns
	var dangerous_patterns = [
		"javascript:",
		"data:",
		"vbscript:",
		"file:",
		"about:",
		"<script",
		"onclick",
		"onerror",
		"onload",
		"%3Cscript",  # URL encoded <script
		"&#",  # HTML entity encoding
	]

	var lower_url = url.to_lower()
	for pattern in dangerous_patterns:
		if pattern in lower_url:
			return false

	# Check for suspicious characters that might indicate injection attempts
	var suspicious_chars = ["<", ">", '"', "'", "`", "{", "}", "[", "]"]
	for char in suspicious_chars:
		if char in url:
			return false

	# Validate URL structure (basic check)
	if url.length() > 2048:  # Reasonable max URL length
		return false

	# Check for double slashes after the protocol (except the initial https://)
	var after_protocol = url.substr(8)  # Skip "https://"
	if "//" in after_protocol:
		return false

	return true


# Sanitize URL for use in href attribute
func _sanitize_url(url: String) -> String:
	# Remove any potential BBCode injections
	var sanitized = url

	# Remove BBCode tags if any slipped through
	sanitized = sanitized.replace("[", "%5B")
	sanitized = sanitized.replace("]", "%5D")

	# Ensure URL hasn't been tampered with
	if not sanitized.begins_with("https://"):
		return ""

	return sanitized


# Sanitize text for display (the visible part of the link)
func _sanitize_for_display(text: String) -> String:
	# Escape BBCode characters to prevent injection in the display text
	var sanitized = text
	sanitized = sanitized.replace("[", "\\[")
	sanitized = sanitized.replace("]", "\\]")
	return sanitized


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

	var my_name = me.get_name()
	if not me.has_claimed_name():
		my_name = my_name + "#" + me.get_user_id().right(4)

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
		Global.modal_manager.show_external_link_modal(meta_str)
	if Global.is_mobile():
		DisplayServer.virtual_keyboard_hide()


func _handle_coordinate_click(coord_str: String):
	var coords = coord_str.split(",")
	if coords.size() == 2:
		var x = int(coords[0])
		var y = int(coords[1])
		Global.modal_manager.async_show_teleport_modal(Vector2i(x, y))


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
					Global.get_explorer()._async_open_profile_by_address(avatar.avatar_id)
					break


func async_adjust_panel_size():
	_adjust_panel_size()


func _adjust_panel_size():
	var margin = 40
	if compact_view:
		margin = 20
	# Calculate required width based on text for extended chat
	var rich_text_label = rich_text_label_message
	if compact_view:
		rich_text_label = rich_text_label_compact_chat

	var parsed_text = rich_text_label.get_parsed_text()
	var font = rich_text_label.get_theme_default_font()
	var font_size = rich_text_label.get_theme_font_size("normal_font_size")
	if font_size == -1:
		font_size = 16  # default size
	var text_width = font.get_string_size(parsed_text, HORIZONTAL_ALIGNMENT_LEFT, -1, font_size).x

	# Minimum and maximum width
	var min_width = 25
	var desired_width = max(min_width, min(text_width + margin, max_panel_width))
	# Set custom size
	panel_container_compact.custom_minimum_size.x = desired_width
	panel_container_extended.custom_minimum_size.x = desired_width

	# If text is too long, allow RichTextLabel to wrap
	if text_width > desired_width:
		rich_text_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	else:
		rich_text_label.autowrap_mode = TextServer.AUTOWRAP_OFF


func escape_bbcode(text: String) -> String:
	# Insert zero-width space after [ to break BBCode parsing
	text = text.replace("[", "[\u200b")  # Zero-width space
	return text
