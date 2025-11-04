class_name PlaceItem
extends Control

signal item_pressed(data)
signal event_pressed(data)
signal jump_in(position: Vector2i, realm: String)
signal close

const TIME_PILL_BLACK = preload("res://src/ui/components/events/time_pill_black.tres")
const TIME_PILL_RED = preload("res://src/ui/components/events/time_pill_red.tres")

@export var texture: Texture2D = texture_placeholder
@export var title: String = "Scene Title"
@export var event_name: String = "Event Name"
@export var description: String = "Scene Description"
@export var views: int = 0
@export var onlines: int = 0
@export var likes_percent: float = 0.0
@export var metadata: Dictionary = {}
@export var location: Vector2i = Vector2i(0, 0)
@export var realm: String = Realm.MAIN_REALM
@export var realm_title: String = "Genesis City"

var event_id: String
var event_status: String
var event_tags: String
var engagement_bar: HBoxContainer
var texture_placeholder = load("res://assets/ui/placeholder.png")
var _data = null
var _node_cache: Dictionary = {}


func _ready():
	mouse_filter = Control.MOUSE_FILTER_PASS
	UiSounds.install_audio_recusirve(self)
	_connect_signals()

	engagement_bar = _get_engagement_bar()

	if metadata.is_empty():
		set_image(texture)
		set_views(views)
		set_online(onlines)
		set_title(title)
		set_event_name(event_name)
		set_description(description)
		set_likes_percent(likes_percent)
		set_location(location)
		set_event_location(location)
	else:
		set_data(metadata)


func _get_node_safe(node_name: String) -> Node:
	if not _node_cache.has(node_name):
		_node_cache[node_name] = get_node_or_null("%" + node_name)
	return _node_cache[node_name]


func _get_engagement_bar() -> HBoxContainer:
	return _get_node_safe("EngagementBar")


func _get_button_close() -> Button:
	return _get_node_safe("Button_Close")


func _get_button_jump_in() -> Button:
	return _get_node_safe("Button_JumpIn")


func _get_button_jump_to_event() -> Button:
	return _get_node_safe("Button_JumpToEvent")


func _get_label_location() -> Label:
	return _get_node_safe("Label_Location")


func _get_label_event_location() -> Label:
	return _get_node_safe("Label_EventLocation")


func _get_label_realm() -> Label:
	return _get_node_safe("Label_Realm")


func _get_label_creator() -> Label:
	return _get_node_safe("Label_Creator")


func _get_label_user_name() -> Label:
	return _get_node_safe("Label_UserName")


func _get_container_user_name() -> Label:
	return _get_node_safe("HBoxContainer_UserName")


func _get_container_creator() -> Control:
	return _get_node_safe("HBoxContainer_Creator")


func _get_label_title() -> Label:
	return _get_node_safe("Label_Title")


func _get_rich_label_event_name() -> RichTextLabel:
	return _get_node_safe("RichTextLabel_EventName")


func _get_label_event_name() -> Label:
	return _get_node_safe("Label_EventName")


func _get_trending_pill() -> Control:
	return _get_node_safe("TrendingPill")


func _get_duration_label() -> Label:
	return _get_node_safe("Label_Duration")


func _get_recurrent_label() -> Label:
	return _get_node_safe("Label_Recurrent")


func _get_label_time_pill() -> Label:
	return _get_node_safe("Label_TimePill")


func _get_label_live_pill() -> Label:
	return _get_node_safe("Label_LivePill")


func _get_border() -> Control:
	return _get_node_safe("Border")


func _get_label_attendees_number() -> Label:
	return _get_node_safe("Label_AttendeesNumber")


func _get_reminder_button() -> Button:
	return _get_node_safe("Button_Reminder")


func _get_label_description() -> Label:
	return _get_node_safe("Label_Description")


func _get_label_online() -> Label:
	return _get_node_safe("Label_Online")


func _get_container_online() -> Control:
	return _get_node_safe("Container_Online")


func _get_label_views() -> Label:
	return _get_node_safe("Label_Views")


func _get_container_views() -> Control:
	return _get_node_safe("HBoxContainer_Views")


func _get_label_likes() -> Label:
	return _get_node_safe("Label_Likes")


func _get_container_likes() -> Control:
	return _get_node_safe("HBoxContainer_Likes")


func _get_texture_image() -> TextureRect:
	return _get_node_safe("TextureRect_Image")


func _connect_signals():
	var button_close = _get_button_close()
	if button_close:
		if not button_close.pressed.is_connected(_on_button_close_pressed):
			button_close.pressed.connect(_on_button_close_pressed)

	var button_jump_in = _get_button_jump_in()
	if button_jump_in:
		if not button_jump_in.pressed.is_connected(_on_button_jump_in_pressed):
			button_jump_in.pressed.connect(_on_button_jump_in_pressed)

	var button_jump_to_event = _get_button_jump_to_event()
	if button_jump_to_event:
		if not button_jump_to_event.pressed.is_connected(_on_button_jump_to_event_pressed):
			button_jump_to_event.pressed.connect(_on_button_jump_to_event_pressed)


func set_location(_location: Vector2i):
	var label = _get_label_location()
	if label:
		location = _location
		label.text = "%s, %s" % [_location.x, _location.y]


func set_event_location(_location: Vector2i):
	var label = _get_label_event_location()
	if label:
		location = _location
		label.text = "%s, %s" % [_location.x, _location.y]


func set_image(_texture: Texture2D):
	var texture_rect = _get_texture_image()
	if texture_rect:
		texture_rect.texture = _texture


func set_title(_title: String):
	var label = _get_label_title()
	if label:
		label.text = _title


func set_description(_description: String):
	description = _description
	var label = _get_label_description()
	if label:
		label.text = _description


func set_views(_views: int):
	var label = _get_label_views()
	var container = _get_container_views()
	if label and container:
		container.set_visible(_views > 0)
		label.text = _format_number(_views)


func set_likes_percent(_likes: float):
	var label = _get_label_likes()
	var container = _get_container_likes()
	if label and container:
		container.set_visible(_likes > 0.0)
		label.text = str(int(round(_likes * 100))) + "%"


func set_online(_online: int):
	var label = _get_label_online()
	var container = _get_container_online()
	if label and container:
		container.set_visible(_online > 0)
		label.text = _format_number(_online)


func set_realm(_realm: String, _realm_title: String):
	realm = _realm
	var label = _get_label_realm()
	if label:
		label.text = _realm_title


func set_user_name(_user_name: String):
	var label = _get_label_user_name()
	var container = _get_container_user_name()
	if label and container:
		container.set_visible(not _user_name.is_empty())
		label.text = _user_name


func set_creator(_creator: String):
	var label = _get_label_creator()
	var container = _get_container_creator()
	if label and container:
		container.set_visible(not _creator.is_empty())
		label.text = _creator


func set_data(item_data):
	_data = item_data

	set_title(item_data.get("title", "Unknown place"))
	set_description(_get_or_empty_string(item_data, "description"))
	set_attendees_number(item_data.get("total_attendees", 0))
	set_trending(item_data.get("trending", false))
	event_id = item_data.get("id", "id")
	set_attending(item_data.get("attending", false), event_id, event_tags)
	set_event_name(item_data.get("name", "Event Name"), item_data.get("user_name", ""))
	set_user_name(item_data.get("user_name", ""))
	set_views(item_data.get("user_visits", 0))
	var like_score = item_data.get("like_score", 0.0)
	set_likes_percent(like_score if like_score is float else 0.0)
	set_online(item_data.get("user_count", 0))
	set_duration(item_data.get("duration", 0))
	set_recurrent(item_data.get("recurrent", false))

	# Handle start_at for events (Unix timestamp)
	var next_start_at = item_data.get("next_start_at", "")
	var live = item_data.get("live", false)
	event_status = "live" if live else "upcoming"
	if next_start_at != "":
		# Convert ISO string to Unix timestamp
		var timestamp = _parse_iso_timestamp(next_start_at)
		if timestamp > 0:
			set_time(timestamp, live)

	if _get_texture_image():
		var image_url = item_data.get("image", "")
		if not image_url.is_empty():
			_async_download_image(image_url)
		else:
			set_image(texture_placeholder)

	var location_vector = item_data.get("base_position", "0,0").split(",")
	if location_vector.size() == 2:
		set_location(Vector2i(int(location_vector[0]), int(location_vector[1])))

	var event_location_vector = item_data.get("coordinates", [0, 0])
	if event_location_vector.size() == 2:
		set_event_location(Vector2i(int(event_location_vector[0]), int(event_location_vector[1])))

	set_creator(_get_or_empty_string(item_data, "contact_name"))
	var world = item_data.get("world", false)
	if world:
		var world_name = item_data.get("world_name")
		if world_name:
			set_realm(world_name, world_name)
	else:
		set_realm(Realm.MAIN_REALM, "Genesis City")

	if engagement_bar:
		engagement_bar.update_data(_data.get("id", null))


func _async_download_image(url: String):
	var url_hash = get_hash_from_url(url)
	var promise = Global.content_provider.fetch_texture_by_url(url_hash, url)
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		set_image(texture_placeholder)
		printerr("places_generator::_async_download_image promise error: ", result.get_error())
		return
	set_image(result.texture)


func _on_button_jump_in_pressed():
	jump_in.emit(location, realm)


func _on_button_close_pressed() -> void:
	close.emit()


func _on_pressed():
	item_pressed.emit(_data)


func _format_number(num: int) -> String:
	if num < 1e3:
		return str(num)
	if num < 1e6:
		return str(int(ceil(num / 1000.0))) + "k"
	return str(int(floor(num / 1000000.0))) + "M"


func get_hash_from_url(url: String) -> String:
	if url.contains("/content/contents/"):
		var parts = url.split("/")
		return parts[parts.size() - 1]

	var context := HashingContext.new()
	if context.start(HashingContext.HASH_SHA256) == OK:
		context.update(url.to_utf8_buffer())
		var url_hash: PackedByteArray = context.finish()
		return url_hash.hex_encode()

	return "temp-file"


func _get_or_empty_string(dict: Dictionary, key: String) -> String:
	var value = dict.get(key, null)
	if value is String:
		return value
	return ""


func set_event_name(_event_name: String, _user_name: String = "") -> void:
	event_name = _event_name
	var limit_to_trim = 90
	var modified_event_name: String
	if _user_name.length() > 0:
		limit_to_trim = limit_to_trim - 4 - _user_name.length()
		if event_name.length() > limit_to_trim:
			modified_event_name = "[font_size=28][b]" + _event_name.left(limit_to_trim) + "...[/b] "  #[font_size=15]by [color='#ff2d55'][b]" + _user_name
		else:
			modified_event_name = "[font_size=28][b]" + _event_name.left(limit_to_trim) + "[/b]"  # [font_size=15]by [color='#ff2d55'][b]" + _user_name
	else:
		if _event_name.length() > limit_to_trim:
			modified_event_name = "[font_size=28][b]" + _event_name.left(limit_to_trim) + "...[/b]"
		else:
			modified_event_name = "[font_size=28][b]" + _event_name.left(limit_to_trim) + "[/b]"
	var rich_text_label = _get_rich_label_event_name()
	if rich_text_label:
		rich_text_label.text = modified_event_name
	var label = _get_label_event_name()
	if label:
		label.text = _event_name


func set_trending(_trending: bool) -> void:
	event_tags = "trending" if _trending else "none"
	var trending_pill = _get_trending_pill()
	if trending_pill:
		trending_pill.set_visible(_trending)


func set_duration(_duration: int) -> void:
	var duration_label = _get_duration_label()
	if duration_label:
		duration_label.text = _format_duration(_duration)


func set_recurrent(_recurrent: bool) -> void:
	var label = _get_recurrent_label()
	if label:
		if _recurrent:
			label.text = "YES"
		else:
			label.text = "NO"


func set_time(_start_at: int, live: bool) -> void:
	var time_pill = _get_label_time_pill()
	var live_pill = _get_label_live_pill()
	var border = _get_border()
	var jump_to_event = _get_button_jump_to_event()
	var reminder_button = _get_reminder_button()

	if time_pill and live_pill:
		if live:
			live_pill.text = "LIVE"
			if jump_to_event and reminder_button:
				jump_to_event.show()
				reminder_button.hide()
			if border:
				border.self_modulate = "#FFFFFF"
			live_pill.get_parent().show()
			time_pill.get_parent().hide()
			return
		var time_text = _format_timestamp(_start_at)
		time_pill.text = time_text
		live_pill.text = time_text


func set_attendees_number(_attendees: int) -> void:
	var label = _get_label_attendees_number()
	if label:
		label.text = str(_attendees)


func set_attending(_attending: bool, _id: String, _event_tags: String) -> void:
	var reminder_button = _get_reminder_button()
	if reminder_button:
		reminder_button.event_id_value = _id
		reminder_button.event_tags = _event_tags
		reminder_button.set_pressed_no_signal(_attending)
		reminder_button.update_styles(_attending)


func _parse_iso_timestamp(iso_string: String) -> int:
	# Convert ISO string (e.g. "2025-10-06T12:00:00.000Z") to Unix timestamp
	if iso_string.is_empty():
		return 0

	# Parse ISO date
	var date_parts = iso_string.split("T")
	if date_parts.size() != 2:
		return 0

	var date_part = date_parts[0]  # "2025-10-06"
	var time_part = date_parts[1].replace("Z", "").split(".")[0]  # "12:00:00"

	var date_components = date_part.split("-")
	var time_components = time_part.split(":")

	if date_components.size() != 3 or time_components.size() != 3:
		return 0

	var year = int(date_components[0])
	var month = int(date_components[1])
	var day = int(date_components[2])
	var hour = int(time_components[0])
	var minute = int(time_components[1])
	var second = int(time_components[2])

	# Create date dictionary and convert to timestamp
	var date_dict = {
		"year": year, "month": month, "day": day, "hour": hour, "minute": minute, "second": second
	}

	return Time.get_unix_time_from_datetime_dict(date_dict)


func _format_timestamp_for_calendar(timestamp: int) -> String:
	# Convert Unix timestamp to ISO format for Google Calendar (YYYYMMDDTHHMMSSZ)
	var time_dict = Time.get_datetime_dict_from_unix_time(timestamp)
	return (
		"%04d%02d%02dT%02d%02d%02dZ"
		% [
			time_dict.year,
			time_dict.month,
			time_dict.day,
			time_dict.hour,
			time_dict.minute,
			time_dict.second
		]
	)


func _format_timestamp(timestamp: int) -> String:
	var now = Time.get_unix_time_from_system()
	var time_diff = timestamp - now
	var live_pill_parent = _get_label_live_pill().get_parent()
	var time_pill = _get_label_time_pill()
	var time_pill_parent = time_pill.get_parent()
	var border = _get_border()
	var jump_in_button = _get_button_jump_to_event()
	var reminder_button = _get_reminder_button()
	# Create unique styles for this instance
	var time_pill_parent_style = time_pill_parent.get_theme_stylebox("panel")
	if time_pill_parent_style:
		var unique_style = time_pill_parent_style.duplicate()
		unique_style.border_color = Color("#161518")
		time_pill_parent.add_theme_stylebox_override("panel", unique_style)

	if time_pill:
		time_pill.label_settings = TIME_PILL_BLACK

	# If event has passed, show date
	if time_diff <= 0:
		var time_dict = Time.get_datetime_dict_from_unix_time(timestamp)
		var month_names = [
			"", "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC"
		]
		return month_names[time_dict.month] + " " + str(time_dict.day)

	# Calculate time differences
	var minutes_diff = time_diff / 60
	var hours_diff = time_diff / 3600
	var days_diff = time_diff / 86400

	if minutes_diff < 5:
		live_pill_parent.show()
		time_pill_parent.hide()
		if jump_in_button and reminder_button:
			jump_in_button.show()
			reminder_button.hide()
		if border:
			border.self_modulate = "#FFFFFF"
		return "IN " + str(int(minutes_diff)) + " MINS"

	live_pill_parent.hide()
	time_pill_parent.show()
	if jump_in_button and reminder_button:
		jump_in_button.hide()
		reminder_button.show()
	if border:
		border.self_modulate = "#FFFFFF00"

	# If less than 1 hour remaining: IN XX MINUTES
	if hours_diff < 1:
		# Create unique styles for this instance with red color
		if time_pill_parent:
			var original_style = time_pill_parent.get_theme_stylebox("panel")
			if original_style:
				var red_style = original_style.duplicate()
				red_style.border_color = Color("#ff2d55")
				time_pill_parent.add_theme_stylebox_override("panel", red_style)

		if time_pill:
			time_pill.label_settings = TIME_PILL_RED

		return "IN " + str(int(minutes_diff)) + " MINS"

	# If less than 48 hours remaining: IN XX HOURS
	if hours_diff < 48:
		if hours_diff > 2:
			return "IN " + str(int(hours_diff)) + " HRS"
		return "IN " + str(int(hours_diff)) + " HR"

	# If 7 days or less remaining: IN X DAYS
	if days_diff <= 7:
		return "IN " + str(int(days_diff)) + " DAYS"

	# If more than 7 days remaining: Show date in SEPT 31 format
	var time_dict = Time.get_datetime_dict_from_unix_time(timestamp)
	var month_names = [
		"", "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC"
	]
	return month_names[time_dict.month] + " " + str(time_dict.day)


func _format_duration(duration: int) -> String:
	# Convert milliseconds to hours
	var hours = duration / (1000 * 60 * 60)

	# If less than 1 hour, show minutes
	if hours < 1:
		var minutes = duration / (1000 * 60)
		if minutes == 1:
			return "1 MIN"
		return str(int(minutes)) + " MINS"

	# If less than 72 hours, show hours
	if hours < 72:
		if hours == 1:
			return "1 HR"
		return str(int(hours)) + " HRS"

	# If more than 72 hours, show days
	var days = hours / 24
	return str(int(days)) + " DAYS"


func schedule_event() -> void:
	if not _data:
		return

	# Get event data
	var next_start_at = _data.get("next_start_at", "")
	var next_finish_at = _data.get("next_finish_at", "")

	# Create jump in URL with location coordinates
	var jump_in_url = (
		"https://decentraland.org/jump/events?position=%d%%2C%d&realm=main"
		% [location.x, location.y]
	)

	# Combine description with jump in URL
	var details = description
	if not description.is_empty():
		details += "\n\n"
	details += "jump in: " + jump_in_url

	# Create dates for Google Calendar
	#var dates_param = ""
	if not next_start_at.is_empty() and not next_finish_at.is_empty():
		var start_timestamp = _parse_iso_timestamp(next_start_at)
		var finish_timestamp = _parse_iso_timestamp(next_finish_at)
		var start_time_millis = start_timestamp * 1000
		var end_time_millis = finish_timestamp * 1000
		var event_location: String = "Decentraland at " + str(location.x) + "," + str(location.y)
		if DclGodotAndroidPlugin.is_available():
			DclGodotAndroidPlugin.add_calendar_event(
				event_name, details, start_time_millis, end_time_millis, event_location
			)
		elif DclGodotAndroidPlugin.is_available():
			DclIosPlugin.add_calendar_event(
				event_name, details, start_time_millis, end_time_millis, event_location
			)


func _on_event_pressed() -> void:
	event_pressed.emit(event_id)


func _on_button_share_pressed() -> void:
	if not _data or not _data.has("id"):
		printerr("No event data available to share")
		return

	if event_id.is_empty():
		printerr("Event ID not available")
		return

	var event_url = "https://decentraland.org/events/event/?id=" + event_id

	var event_title = _data.get("name", "Decentraland Event")

	var text = "Visit the event '" + event_title + "' following this link " + event_url

	if Global.is_android():
		DclGodotAndroidPlugin.share_text(text)
	elif Global.is_ios():
		DclIosPlugin.share_text(text)


func _on_button_calendar_pressed() -> void:
	schedule_event()


func _on_button_jump_to_event_pressed() -> void:
	Global.metrics.track_click_button(
		"jump_to", "EVENT_DETAILS", JSON.stringify({"event_id": event_id, "event_tag": event_tags})
	)
	jump_in.emit(location, realm)
