extends Control

## When true (e.g., notifications with closed chat), the message body is shortened.
const MAX_CHARS_REDUCED: int = 80
const TAG_COLOR := "#716B7C"

## Design specs — landscape
const LANDSCAPE_MARGIN_H := 10
const LANDSCAPE_MARGIN_V := 8
const LANDSCAPE_FONT_SIZE := 20
const LANDSCAPE_BOLD_SIZE := 24

## Design specs — portrait
const PORTRAIT_MARGIN := 16
const PORTRAIT_FONT_SIZE := 29
const PORTRAIT_BOLD_SIZE := 33

## Bold header ~22pt (theme). Small separator: nick<->tag, tag<->icon, nick<->icon (~4px).
const SEP_HEADER_SMALL := "[font_size=22]\u200a\u200a[/font_size]"
## Large separator: always between the header block (nick / nick+tag / nick+icon / ...) and the message (~8px).
const SEP_HEADER_TO_MESSAGE := "[font_size=22]\u2004\u200a[/font_size]"

const MY_MENTION_COLOR = "#FC03EC"
const MY_MENTION_BORDER := Color(0.988235, 0.011765, 0.92549)
const MENTION_COLOR = "#FFD700"

const BG_OTHER := Color(0, 0, 0, 0.4)
const BG_OWN := Color(0, 0, 0, 0.7)
const BG_SYSTEM := Color(0, 0, 0, 0.4)

var nickname: String = "Unknown"
var tag: String = ""
var nickname_color_hex: String = "ffffff"
var is_own_message: bool = false
var has_claimed_name: bool = false
var reduce_text: bool = false

var _address: String = ""

@onready var rich_text: RichTextLabel = %RichTextLabel_Message
@onready var message_panel: PanelContainer = %PanelContainer_Message
@onready var content_margin: MarginContainer = %MarginContainer_Content


func _ready() -> void:
	rich_text.meta_clicked.connect(_on_url_clicked)
	_configure_richtext_theme(rich_text)


func _configure_richtext_theme(richtext_label: RichTextLabel) -> void:
	var custom_theme = Theme.new()
	var link_style = StyleBoxFlat.new()
	link_style.bg_color = Color.TRANSPARENT
	link_style.set_border_width_all(0)
	custom_theme.set_stylebox("normal", "LinkButton", link_style)
	richtext_label.theme = custom_theme

	const MULTILINGUAL_REGULAR := preload(
		"res://assets/themes/fonts/multilanguage/multilanguage_font-Regular.tres"
	)
	const MULTILINGUAL_SEMIBOLD := preload(
		"res://assets/themes/fonts/multilanguage/multilanguage_font-SemiBold.tres"
	)

	richtext_label.add_theme_font_override("normal_font", MULTILINGUAL_REGULAR)
	richtext_label.add_theme_font_override("bold_font", MULTILINGUAL_SEMIBOLD)
	richtext_label.add_theme_font_override("bold_italics_font", MULTILINGUAL_SEMIBOLD)
	richtext_label.add_theme_font_override("italics_font", MULTILINGUAL_REGULAR)
	richtext_label.add_theme_font_override("mono_font", MULTILINGUAL_REGULAR)


func set_chat(address: String, message: String, _timestamp: float) -> void:
	_address = address
	var new_text: String = message
	if address != "system":
		new_text = escape_bbcode(new_text)

	if reduce_text and new_text.length() > MAX_CHARS_REDUCED:
		new_text = new_text.substr(0, MAX_CHARS_REDUCED) + "..."

	var own_address: String = Global.player_identity.get_address_str()
	is_own_message = own_address == address

	if address == "system":
		_set_system_sender()
	elif address == own_address:
		set_avatar(Global.scene_runner.player_avatar_node)
	else:
		set_avatar(Global.avatars.get_avatar_by_address(address))

	_apply_message_background()

	var processed_message = make_urls_clickable(new_text)
	rich_text.text = _build_chat_rich_text(processed_message)
	_update_layout.call_deferred()


func _build_chat_rich_text(processed_message: String) -> String:
	# Header: Nick (small sep) Tag? (small sep) Icon?  ->  (large sep)  Message
	var chunks: PackedStringArray = []
	chunks.append("[b][color=#%s]%s[/color][/b]" % [nickname_color_hex, nickname])
	if not tag.is_empty():
		chunks.append(SEP_HEADER_SMALL)
		chunks.append("[color=%s]%s[/color]" % [TAG_COLOR, tag])
	if has_claimed_name:
		chunks.append(SEP_HEADER_SMALL)
		chunks.append("[img=22]res://assets/check-mark.svg[/img]")
	chunks.append(SEP_HEADER_TO_MESSAGE)
	# System already includes its own BBCode ([color], [b], ...); don't wrap in [b] or nesting breaks.
	if _address == "system":
		chunks.append(processed_message)
	else:
		chunks.append("[b]%s[/b]" % processed_message)
	return "".join(chunks)


func set_avatar(avatar: DclAvatar) -> void:
	if avatar == null or not is_instance_valid(avatar):
		nickname = "Unknown"
		tag = ""
		has_claimed_name = false
		nickname_color_hex = "ffffff"
		return

	var full_name: String = avatar.get_avatar_name()
	nickname = full_name
	var color = DclAvatar.get_nickname_color(full_name)
	nickname_color_hex = color.to_html(false) if color != null else "ffffff"

	var parts: PackedStringArray = full_name.split("#", false)
	if parts.size() > 1:
		nickname = parts[0]
		tag = "#" + parts[1]
		has_claimed_name = false
	else:
		tag = ""
		has_claimed_name = true


func _set_system_sender() -> void:
	nickname = "System"
	tag = ""
	nickname_color_hex = "00ff00"
	has_claimed_name = false


func _apply_message_background() -> void:
	var box := StyleBoxFlat.new()
	box.set_corner_radius_all(5)
	if _address == "system":
		box.bg_color = BG_SYSTEM
	elif is_own_message:
		box.bg_color = BG_OWN
	else:
		box.bg_color = BG_OTHER
	message_panel.add_theme_stylebox_override("panel", box)


func make_urls_clickable(text: String) -> String:
	var processed_text = text

	var mention_regex = RegEx.new()
	mention_regex.compile(r"@([^#\s]+)(?:#([0-9a-fA-F]{4}))?")

	var mention_results = mention_regex.search_all(processed_text)
	for i in range(mention_results.size() - 1, -1, -1):
		var mention_match = mention_results[i]
		var full_mention = mention_match.get_string()
		var start_pos = mention_match.get_start()
		var end_pos = mention_match.get_end()

		var username = mention_match.get_string(1)

		var user_tag = ""
		if mention_match.get_group_count() > 1:
			var tag_match = mention_match.get_string(2)
			if tag_match != null and tag_match != "":
				user_tag = tag_match

		var unique_name = ("@" + username) if user_tag.is_empty() else full_mention

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
			var mention_colored = "[color=%s]%s[/color]" % [MY_MENTION_COLOR, full_mention]
			processed_text = (
				processed_text.substr(0, start_pos)
				+ mention_colored
				+ processed_text.substr(end_pos)
			)
			_apply_mention_style()

	var coord_regex = RegEx.new()
	coord_regex.compile(r"(-?\d+,-?\d+)")

	var coord_results = coord_regex.search_all(processed_text)
	for i in range(coord_results.size() - 1, -1, -1):
		var coord_match = coord_results[i]
		var coord = coord_match.get_string()
		var start_pos = coord_match.get_start()
		var end_pos = coord_match.get_end()

		if _is_valid_coordinate(coord):
			var clickable_coord = "[url=coord:%s][color=#66B3FF]%s[/color][/url]" % [coord, coord]
			processed_text = (
				processed_text.substr(0, start_pos)
				+ clickable_coord
				+ processed_text.substr(end_pos)
			)

	var url_regex = RegEx.new()
	# Match https://, http://, www., or bare domains (e.g., amazon.com, example.co.uk)
	url_regex.compile(r"(https?://[^\s]+|www\.[^\s]+|[a-zA-Z0-9][-a-zA-Z0-9]*\.[a-zA-Z]{2,}[^\s]*)")

	var url_results = url_regex.search_all(processed_text)
	for i in range(url_results.size() - 1, -1, -1):
		var url_match = url_results[i]
		var url = url_match.get_string()
		var start_pos = url_match.get_start()
		var end_pos = url_match.get_end()

		# Strip trailing punctuation that's likely not part of the URL
		var trailing_punct = [".", ",", "!", "?", ")", ";", ":"]
		while url.length() > 0 and url[-1] in trailing_punct:
			url = url.substr(0, url.length() - 1)
			end_pos -= 1

		var full_url = Realm.ensure_starts_with_https(url)

		if _is_safe_url(full_url):
			var sanitized_url = _sanitize_url(full_url)
			var display_url = _sanitize_for_display(url)

			var clickable_url = (
				"[url=%s][color=#66B3FF]%s[/color][/url]" % [sanitized_url, display_url]
			)
			processed_text = (
				processed_text.substr(0, start_pos) + clickable_url + processed_text.substr(end_pos)
			)

	return processed_text


func _is_safe_url(url: String) -> bool:
	if not url.begins_with("https://"):
		return false

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
		"%3Cscript",
		"&#",
	]

	var lower_url = url.to_lower()
	for pattern in dangerous_patterns:
		if pattern in lower_url:
			return false

	var suspicious_chars = ["<", ">", '"', "'", "`", "{", "}", "[", "]"]
	for c in suspicious_chars:
		if c in url:
			return false

	if url.length() > 2048:
		return false

	var after_protocol = url.substr(8)
	if "//" in after_protocol:
		return false

	return true


func _sanitize_url(url: String) -> String:
	var sanitized = url
	sanitized = sanitized.replace("[", "%5B")
	sanitized = sanitized.replace("]", "%5D")

	if not sanitized.begins_with("https://"):
		return ""

	return sanitized


func _sanitize_for_display(t: String) -> String:
	var sanitized = t
	sanitized = sanitized.replace("[", "\\[")
	sanitized = sanitized.replace("]", "\\]")
	return sanitized


func _is_valid_mention(mention: String) -> bool:
	if not Global.avatars:
		return false

	if not mention.begins_with("@"):
		return false

	var mention_without_at = mention.substr(1)

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

	var mention_without_at = mention.substr(1)

	var me = Global.player_identity.get_profile_or_null()
	if not me:
		return false

	var my_name = me.get_name()
	if not me.has_claimed_name():
		my_name = my_name + "#" + me.get_user_id().right(4)

	if my_name == mention_without_at:
		return true
	return false


func _apply_mention_style() -> void:
	var base = message_panel.get_theme_stylebox("panel")
	if base == null:
		return
	var stylebox = base.duplicate() as StyleBoxFlat
	if stylebox == null:
		return
	stylebox.set_border_width_all(2)
	stylebox.border_color = MY_MENTION_BORDER
	message_panel.add_theme_stylebox_override("panel", stylebox)


func _is_valid_coordinate(coord_str: String) -> bool:
	var coords = coord_str.split(",")
	if coords.size() != 2:
		return false

	var x = int(coords[0])
	var y = int(coords[1])

	return x >= -150 and x <= 150 and y >= -150 and y <= 150


func _on_url_clicked(meta) -> void:
	var meta_str = str(meta)
	if meta_str.begins_with("coord:"):
		var coord_str = meta_str.substr(6)
		_handle_coordinate_click(coord_str)
	elif meta_str.begins_with("mention:"):
		var mention_str = meta_str.substr(8)
		_handle_mention_click(mention_str)
	else:
		Global.modal_manager.async_show_external_link_modal(meta_str)
	if Global.is_mobile():
		DisplayServer.virtual_keyboard_hide()


func _handle_coordinate_click(coord_str: String) -> void:
	var coords = coord_str.split(",")
	if coords.size() == 2:
		var x = int(coords[0])
		var y = int(coords[1])
		Global.modal_manager.async_show_teleport_modal(Vector2i(x, y))


func _handle_mention_click(mention_str: String) -> void:
	if not mention_str.begins_with("@"):
		return

	var mention_without_at = mention_str.substr(1)

	if Global.avatars:
		var avatars = Global.avatars.get_avatars()
		for avatar in avatars:
			if avatar and avatar.has_method("get_avatar_name"):
				var avatar_name = avatar.get_avatar_name()
				if avatar_name == mention_without_at:
					Global.get_explorer()._async_open_profile_by_address(avatar.avatar_id)
					break


func _update_layout() -> void:
	await get_tree().process_frame
	if rich_text.size.x <= 0:
		return  # Chat hidden, will be re-layouted on open

	# Reset to full width so we measure against the real available space
	message_panel.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	message_panel.custom_minimum_size = Vector2.ZERO
	rich_text.custom_minimum_size = Vector2.ZERO
	await get_tree().process_frame

	var content_width := rich_text.get_content_width()
	var available_width := int(rich_text.size.x)
	var h_padding := (content_margin.get_theme_constant("margin_left")
		+ content_margin.get_theme_constant("margin_right"))

	if content_width < available_width:
		message_panel.size_flags_horizontal = Control.SIZE_SHRINK_BEGIN
		message_panel.custom_minimum_size.x = content_width + h_padding

	rich_text.custom_minimum_size.y = rich_text.get_content_height()


func set_portrait(is_portrait: bool) -> void:
	if is_portrait:
		content_margin.add_theme_constant_override("margin_left", PORTRAIT_MARGIN)
		content_margin.add_theme_constant_override("margin_top", PORTRAIT_MARGIN)
		content_margin.add_theme_constant_override("margin_right", PORTRAIT_MARGIN)
		content_margin.add_theme_constant_override("margin_bottom", PORTRAIT_MARGIN)
		rich_text.add_theme_font_size_override("normal_font_size", PORTRAIT_FONT_SIZE)
		rich_text.add_theme_font_size_override("bold_font_size", PORTRAIT_BOLD_SIZE)
	else:
		content_margin.add_theme_constant_override("margin_left", LANDSCAPE_MARGIN_H)
		content_margin.add_theme_constant_override("margin_top", LANDSCAPE_MARGIN_V)
		content_margin.add_theme_constant_override("margin_right", LANDSCAPE_MARGIN_H)
		content_margin.add_theme_constant_override("margin_bottom", LANDSCAPE_MARGIN_V)
		rich_text.add_theme_font_size_override("normal_font_size", LANDSCAPE_FONT_SIZE)
		rich_text.add_theme_font_size_override("bold_font_size", LANDSCAPE_BOLD_SIZE)


func escape_bbcode(text: String) -> String:
	return text.replace("[", "[\u200b")
