@tool
extends Control

const EMOTE: String = "␐"
const REQUEST_PING: String = "␑"
const ACK: String = "␆"

@export var compact_view := false:
	set(value):
		compact_view = value
		h_box_container_extended_chat.visible = !value
		rich_text_label_compact_chat.visible = value
var nickname: String = "Unknown"
var tag: String = ""
var nickname_color_hex: String = "#FFFFFF"

@onready var rich_text_label_compact_chat: RichTextLabel = %RichTextLabel_CompactChat
@onready var h_box_container_extended_chat: HBoxContainer = %HBoxContainer_ExtendedChat
@onready var label_nickname: Label = %Label_Nickname
@onready var label_tag: Label = %Label_Tag
@onready var rich_text_label_message: RichTextLabel = %RichTextLabel_Message
@onready var label_timestamp: Label = %Label_Timestamp
@onready var claimed_checkmark: MarginContainer = %ClaimedCheckmark
@onready var profile_picture: ProfilePicture = %ProfilePicture
@onready var panel_container: PanelContainer = %PanelContainer


func _ready() -> void:
	Global.chat_compact_changed.connect(_on_chat_compact_changed)
	compact_view = Global.is_chat_compact

	# Connect signals for clickable URLs
	rich_text_label_message.meta_clicked.connect(_on_url_clicked)
	rich_text_label_compact_chat.meta_clicked.connect(_on_url_clicked)


func set_chat(chat) -> void:
	var own_address: String = Global.player_identity.get_address_str()
	var address: String = chat[0]

	if own_address == address:
		# Own messages: avatar on the right, content aligned to the right
		h_box_container_extended_chat.layout_direction = Control.LAYOUT_DIRECTION_RTL
		#h_box_container_extended_chat.set_end()
	else:
		# Other messages: avatar on the left, content aligned to the left
		h_box_container_extended_chat.layout_direction = Control.LAYOUT_DIRECTION_LTR
		#h_box_container_extended_chat.alignment = 0  # ALIGNMENT_BEGIN

	var timestamp: float = chat[1]
	var message: String = chat[2]

	var datetime = Time.get_datetime_dict_from_unix_time(int(timestamp))
	var time_string = "%02d:%02d" % [datetime.hour, datetime.minute]

	# Process message to make URLs clickable
	var processed_message = make_urls_clickable(message)
	rich_text_label_message.text = processed_message
	label_timestamp.text = time_string

	# Adjust panel size dynamically
	async_adjust_panel_size.call_deferred()
	var avatar = Global.avatars.get_avatar_by_address(address)

	if avatar == null:
		if address == Global.player_identity.get_address_str():
			avatar = Global.scene_runner.player_avatar_node

	if avatar != null and is_instance_valid(avatar):
		set_avatar(avatar)
	else:
		set_default_avatar(address)

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
		var text = (
			"[b][color=#%s]%s[/color][color=#a9a9a9]%s[/color] [color=#fff]%s[/color]"
			% [nickname_color_hex, nickname, tag, processed_message_compact]
		)
		rich_text_label_compact_chat.append_text(text)


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

	profile_picture.async_update_profile_picture(avatar)


func set_default_avatar(address: String) -> void:
	if address.length() > 32:
		nickname = DclEther.shorten_eth_address(address)
	else:
		nickname = "Unknown"

	tag = ""
	nickname_color_hex = "ffffff"

	label_nickname.text = nickname
	label_tag.text = ""
	label_nickname.add_theme_color_override("font_color", Color.WHITE)
	claimed_checkmark.hide()


func _on_chat_compact_changed(is_compact: bool) -> void:
	compact_view = is_compact
	h_box_container_extended_chat.visible = !is_compact
	rich_text_label_compact_chat.visible = is_compact

	# Readjust size if in extended view
	if not is_compact:
		async_adjust_panel_size.call_deferred()


func make_urls_clickable(text: String) -> String:
	var processed_text = text

	# First, detect and process coordinates (#,# format)
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
			# Replace with clickable BBCode format for coordinates
			var clickable_coord = "[url=coord:%s]%s[/url]" % [coord, coord]
			processed_text = (
				processed_text.substr(0, start_pos)
				+ clickable_coord
				+ processed_text.substr(end_pos)
			)

	# Then, detect and process URLs (http/https/www)
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

		# Replace with clickable BBCode format for URLs
		var clickable_url = "[url=%s]%s[/url]" % [full_url, url]
		processed_text = (
			processed_text.substr(0, start_pos) + clickable_url + processed_text.substr(end_pos)
		)

	return processed_text


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
		# Handle coordinate click
		var coord_str = meta_str.substr(6)  # Remove "coord:" prefix
		_handle_coordinate_click(coord_str)
	else:
		# Handle regular URL click
		Global.show_url_popup(meta_str)


func _handle_coordinate_click(coord_str: String):
	# Parse coordinates from string like "52,-56"
	var coords = coord_str.split(",")
	if coords.size() == 2:
		var x = int(coords[0])
		var y = int(coords[1])
		# Show jump in popup for coordinate confirmation
		Global.show_jump_in_popup(Vector2i(x, y))


func async_adjust_panel_size():
	# Wait one frame for content to render
	await get_tree().process_frame

	# Get available width from parent container
	var parent_width = get_parent().size.x if get_parent() else 400.0

	# Maximum panel width (leaving space for avatar and margins)
	var max_panel_width = parent_width - 100.0  # 100px for avatar and spacing

	## Get RichTextLabel content size
	#var content_size = rich_text_label_message.get_content_height()

	# Calculate required width based on text
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
	panel_container.custom_minimum_size.x = desired_width

	# If text is too long, allow RichTextLabel to wrap
	if text_width > max_panel_width - 40:
		rich_text_label_message.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	else:
		rich_text_label_message.autowrap_mode = TextServer.AUTOWRAP_OFF
