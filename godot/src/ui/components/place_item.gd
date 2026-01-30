class_name PlaceItem
extends Control

signal item_pressed(data)
signal event_pressed(data)
signal jump_in(position: Vector2i, realm: String)
signal close

@export var texture: Texture2D = texture_placeholder
@export var title: String = "Scene Title"
@export var event_name: String = "Event Name"
@export var description: String = "Scene Description"
@export var views: int = 0
@export var onlines: int = 0
@export var likes_percent: float = 0.0
@export var metadata: Dictionary = {}
@export var location: Vector2i = Vector2i(0, 0)
@export var realm: String = DclUrls.main_realm()
@export var realm_title: String = "Genesis City"
@export var categories: Array = []

var event_id: String
var event_status: String
var event_tags: String
var event_start_timestamp: int = 0  # Unix timestamp (seconds) when event starts
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
		set_categories(categories)
	else:
		set_data(metadata)
		
	var card = _get_card()
	if card:
		card.set_anchors_and_offsets_preset(Control.PRESET_BOTTOM_WIDE)
		
	var description_container = _get_description()
	if description_container:
		description_container.hide()


func _get_node_safe(node_name: String) -> Node:
	if not _node_cache.has(node_name):
		_node_cache[node_name] = get_node_or_null("%" + node_name)
	return _node_cache[node_name]


func _get_engagement_bar() -> HBoxContainer:
	return _get_node_safe("EngagementBar")
	
	
func _get_event_pills_bar() -> HBoxContainer:
	return _get_node_safe("EventPillsBar")


func _get_categories_bar() -> CategoriesBar:
	return _get_node_safe("CategoriesBar")


func _get_description() -> VBoxContainer:
	return _get_node_safe("VBoxContainer_Description")


func _get_card() -> PanelContainer:
	return _get_node_safe("PanelContainer_Card")
	
	
func _get_image_container() -> PanelContainer:
	return _get_node_safe("Panel_Container_Image")
	
	
func _get_no_image_container() -> PanelContainer:
	return _get_node_safe("Panel_Container_NoImage")


func _get_button_close() -> Button:
	return _get_node_safe("Button_Close")


func _get_button_jump_in() -> Button:
	return _get_node_safe("Button_JumpIn")


func _get_button_jump_to_event() -> Button:
	return _get_node_safe("Button_JumpToEvent")


func _get_download_warning() -> Button:
	return _get_node_safe("DownloadWarning")


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


func _get_label_event_name() -> Label:
	return _get_node_safe("Label_EventName")

func _get_rich_label_event_name() -> RichTextLabel:
	return _get_node_safe("RichTextLabel_EventName")



func _get_duration_label() -> Label:
	return _get_node_safe("Label_Duration")


func _get_recurrent_label() -> Label:
	return _get_node_safe("Label_Recurrent")


func _get_label_time_pill() -> Label:
	return _get_node_safe("Label_TimePill")


func _get_label_live_pill() -> Label:
	return _get_node_safe("Label_LivePill")


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


func _get_separator_online() -> VSeparator:
	return _get_node_safe("VSeparator_Online")


func _get_separator_likes() -> VSeparator:
	return _get_node_safe("VSeparator_Likes")


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


func _get_fav_button() -> FavButton:
	return _get_node_safe("FavButton")


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
	show_image_container(true)
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
	var separator_online = _get_separator_online()
	var separator_likes = _get_separator_likes()
	if label and container:
		container.set_visible(_likes > 0.0)
		if separator_online:
			separator_online.set_visible(_likes > 0)
		if separator_likes:
			separator_likes.set_visible(_likes > 0)
		label.text = str(int(round(_likes * 100))) + "%"


func set_online(_online: int):
	var label = _get_label_online()
	var container = _get_container_online()
	var separator_online = _get_separator_online()
	var separator_likes = _get_separator_likes()
	if label and container:
		container.set_visible(_online > 0)
		if separator_online:
			separator_online.set_visible(_online > 0)
		if separator_likes:
			separator_likes.set_visible(_online > 0)
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


func set_download_warning(item_data: Dictionary) -> void:
	var download_warning = _get_download_warning()
	if not download_warning:
		return

	var is_world = item_data.get("world", false)
	var max_size_mb: int

	if is_world:
		max_size_mb = 100  # Worlds have ~100MB dynamic capacity
	else:
		var positions = item_data.get("positions", [])
		var parcel_count = positions.size() if positions is Array else 1
		if parcel_count == 0:
			parcel_count = 20  # Fallback to worst-case scenario (300mb)
		max_size_mb = mini(parcel_count * 15, 300)  # 15MB per parcel, max 300MB

	download_warning.set_warning_text("May download up to %dMB of data" % max_size_mb)


func set_data(item_data):
	_data = item_data

	set_title(item_data.get("title", "Unknown place"))
	set_description(_get_or_empty_string(item_data, "description"))
	set_attendees_number(item_data.get("total_attendees", 0))

	event_id = item_data.get("id", "id")
	set_event_name(item_data.get("name", "Event Name"), item_data.get("user_name", ""))
	set_event_pills(item_data)
	set_categories(item_data.get("categories", []))
	# Parse event timestamp BEFORE set_attending so it's available for notifications
	set_fav_button_data(item_data.get("id", "-"))
	var next_start_at = item_data.get("next_start_at", "")
	var live = item_data.get("live", false)
	event_status = "live" if live else "upcoming"
	if next_start_at != "":
		# Convert ISO string to Unix timestamp
		var timestamp = _parse_iso_timestamp(next_start_at)
		if timestamp > 0:
			event_start_timestamp = timestamp  # Store for notification scheduling


	
	
	# Set location before set_attending so event_coordinates is correct for notifications
	var location_vector = item_data.get("base_position", "0,0").split(",")
	if location_vector.size() == 2:
		set_location(Vector2i(int(location_vector[0]), int(location_vector[1])))

	var event_location_vector = item_data.get("coordinates", [0, 0])
	if event_location_vector.size() == 2:
		set_event_location(Vector2i(int(event_location_vector[0]), int(event_location_vector[1])))

	set_attending(item_data.get("attending", false), event_id, event_tags)
	_update_reminder_and_jump_buttons()
	set_user_name(item_data.get("user_name", ""))
	set_views(item_data.get("user_visits", 0))
	var like_score = item_data.get("like_score", 0.0)
	set_likes_percent(like_score if like_score is float else 0.0)
	set_online(item_data.get("user_count", 0))
	set_duration(item_data.get("duration", 0))
	set_recurrent(item_data.get("recurrent", false))

	if _get_texture_image():
		var image_url = item_data.get("image", "")
		if not image_url.is_empty():
			_async_download_image(image_url)
		else:
			show_image_container(false)

	set_creator(_get_or_empty_string(item_data, "contact_name"))
	var world = item_data.get("world", false)
	if world:
		var world_name = item_data.get("world_name")
		if world_name:
			set_realm(world_name, world_name)
	else:
		set_realm(DclUrls.main_realm(), "Genesis City")

	if engagement_bar:
		engagement_bar.update_data(_data.get("id", null))
	

	set_download_warning(item_data)
	
	

func _async_download_image(url: String):
	var url_hash = get_hash_from_url(url)
	var promise = Global.content_provider.fetch_texture_by_url(url_hash, url)
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		show_image_container(false)
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
	var rtl = _get_rich_label_event_name()
	if rtl:

		rtl.text = _event_name
		await get_tree().process_frame

		var font := rtl.get_theme_font("normal_font")
		var font_size := rtl.get_theme_font_size("normal_font_size")
		var line_height := font.get_height(font_size)

		var max_width = rtl.size.x
		if max_width <= 0:
			return

		var full_size := font.get_multiline_string_size(
			_event_name,
			HORIZONTAL_ALIGNMENT_LEFT,
			max_width,
			font_size
		)
		

		var one_line_h := line_height * 1.2
		var two_lines_h := line_height * 2.2
		print(full_size.y, one_line_h, two_lines_h)
		if full_size.y <= one_line_h:
			rtl.custom_minimum_size.y = one_line_h
			return

		# Caso 2: entra en dos líneas
		if full_size.y <= two_lines_h:
			rtl.custom_minimum_size.y = two_lines_h
			return

		# Caso 3: más de dos líneas → trim + altura fija a 2
		rtl.text = trim_to_two_lines_fill(rtl, _event_name)
		rtl.custom_minimum_size.y = two_lines_h

	var label = _get_label_event_name()
	if label:
		label.text = _event_name
	
	
func _apply_trim(text: String) -> void:
	var rich_text_label = _get_rich_label_event_name()
	rich_text_label.text = trim_to_two_lines_fill(
		rich_text_label,
		text
	)
	
	
func trim_to_two_lines_fill(rtl: RichTextLabel, text: String) -> String:
	var font := rtl.get_theme_font("normal_font")
	var font_size := rtl.get_theme_font_size("normal_font_size")
	var max_width := rtl.size.x

	var ellipsis := "…"
	var best := ""
	var current := ""

	for i in text.length():
		current += text[i]

		var size := font.get_multiline_string_size(
			current + ellipsis,
			HORIZONTAL_ALIGNMENT_LEFT,
			max_width,
			font_size
		)

		if size.y > font.get_height(font_size) * 2.2:
			break

		best = current

	return best.rstrip(" ") + ellipsis

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





func set_attendees_number(_attendees: int) -> void:
	var label = _get_label_attendees_number()
	if label:
		label.text = str(_attendees)


func set_attending(_attending: bool, _id: String, _event_tags: String) -> void:
	var reminder_button = _get_reminder_button()
	if reminder_button:
		reminder_button.event_id_value = _id
		reminder_button.event_tags = _event_tags
		reminder_button.event_start_timestamp = event_start_timestamp
		reminder_button.event_name = event_name
		reminder_button.event_coordinates = location
		reminder_button.event_cover_image_url = _data.get("image", "") if _data else ""
		reminder_button.set_pressed_no_signal(_attending)
		reminder_button.update_styles(_attending)


func _update_reminder_and_jump_buttons() -> void:
	var reminder_btn = _get_reminder_button()
	var jump_btn = _get_button_jump_to_event()
	if not reminder_btn or not jump_btn:
		return

	var now = Time.get_unix_time_from_system()
	var is_live = _data.get("live", false) if _data is Dictionary else false
	if event_start_timestamp > 0 and now >= event_start_timestamp:
		is_live = true
	var time_until_start_sec = event_start_timestamp - now if event_start_timestamp > 0 else 0
	var starts_in_less_than_5_mins = (
		event_start_timestamp > 0
		and time_until_start_sec > 0
		and time_until_start_sec < 300
	)
	var show_jump_hide_reminder = is_live or starts_in_less_than_5_mins

	if show_jump_hide_reminder:
		reminder_btn.hide()
		jump_btn.show()
	else:
		reminder_btn.show()
		jump_btn.hide()


func set_categories(_categories:Array) -> void:
	var categories_bar = _get_categories_bar()
	if categories_bar:
		categories_bar.set_categories(_categories)


func set_event_pills(_item_data) -> void:
	var event_pills_bar = _get_event_pills_bar()
	if event_pills_bar:
		event_pills_bar.set_data(_item_data)


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
		DclUrls.jump_events() + "?position=%d%%2C%d&realm=main" % [location.x, location.y]
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
		if DclAndroidPlugin.is_available():
			DclAndroidPlugin.add_calendar_event(
				event_name, details, start_time_millis, end_time_millis, event_location
			)
		elif DclIosPlugin.is_available():
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

	var event_url = DclUrls.host() + "/events/event/?id=" + event_id

	var event_title = _data.get("name", "Decentraland Event")

	var text = "Visit the event '" + event_title + "' following this link " + event_url

	if Global.is_android():
		DclAndroidPlugin.share_text(text)
	elif Global.is_ios():
		DclIosPlugin.share_text(text)


func _on_button_calendar_pressed() -> void:
	schedule_event()


func _on_button_jump_to_event_pressed() -> void:
	Global.metrics.track_click_button(
		"jump_to", "EVENT_DETAILS", JSON.stringify({"event_id": event_id, "event_tag": event_tags})
	)
	jump_in.emit(location, realm)


func _on_panel_toggled(toggled_on: bool) -> void:
	var description_container = _get_description()
	var card = _get_card()
	
	if description_container and card:
		if toggled_on:
			description_container.show()
			card.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
		else:
			description_container.hide()
			card.set_anchors_and_offsets_preset(Control.PRESET_BOTTOM_WIDE)

func show_image_container(toggle:bool) -> void:
	var image_container = _get_image_container()
	var no_image_container = _get_no_image_container()
	if image_container and no_image_container:
		if toggle:
			image_container.show()
			no_image_container.hide()
		else:
			image_container.hide()
			no_image_container.show()
	
func set_fav_button_data(_id:String) -> void:
	var fav_button = _get_fav_button()
	if fav_button:
		fav_button.update_data(_id)
