class_name PlaceItem
extends Control

signal item_pressed(data)
signal event_pressed(data)
signal jump_in(position: Vector2i, realm: String)
signal close

const _CALENDAR_BUTTON_SCENE: PackedScene = preload(
	"res://src/ui/components/calendar_button/calendar_button.tscn"
)

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
		set_online(onlines)
		set_title(title)
		set_event_name(event_name)
		set_description(description)
		set_likes_percent(likes_percent)
		set_location(location)
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


func _get_header() -> PanelContainer:
	return _get_node_safe("PanelContainer_Header")


func _get_show_more_container() -> Button:
	return _get_node_safe("MarginContainer_ShowMore")


func _get_image_container() -> PanelContainer:
	return _get_node_safe("Panel_Container_Image")


func _get_no_image_container() -> PanelContainer:
	return _get_node_safe("Panel_Container_NoImage")


func _get_button_close() -> Button:
	return _get_node_safe("Button_Close")


func _get_texture_button_close() -> TextureButton:
	return _get_node_safe("TextureButton_Close")


func _get_button_jump_in() -> Button:
	return _get_node_safe("Button_JumpIn")


func _get_button_jump_to_event() -> Button:
	return _get_node_safe("Button_JumpToEvent")


func _get_button_jump_to_event_small() -> Button:
	return _get_node_safe("Button_JumpToEventSmall")


func _get_label_event_location_name() -> Label:
	return _get_node_safe("Label_EventLocationName")


func _get_label_event_location_coords() -> Label:
	return _get_node_safe("Label_EventLocationCoords")


func _get_button_calendar() -> CalendarButton:
	return _get_node_safe("Button_Calendar")


func _get_recurrent_dates_separator() -> Node:
	return _get_node_safe("HSeparator_RecurrentDates")


func _get_recurrent_dates_container() -> Node:
	return _get_node_safe("VBoxContainer_RecurrentDates")


func _get_button_share() -> Button:
	return _get_node_safe("Button_Share")


func _get_download_warning() -> Button:
	return _get_node_safe("DownloadWarning")


func _get_label_location() -> Label:
	return _get_node_safe("Label_Location")


func _get_texture_rect_location() -> TextureRect:
	return _get_node_safe("TextureRect_Location")


func _get_texture_rect_server() -> TextureRect:
	return _get_node_safe("TextureRect_Server")


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


func _get_rich_label_event_name() -> TrimmedRichTextLabel:
	return _get_node_safe("RichTextLabel_EventName")


func _get_rich_label_title() -> TrimmedRichTextLabel:
	return _get_node_safe("RichTextLabel_Title")


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


func _get_separator_recurrent() -> VSeparator:
	return _get_node_safe("VSeparator_Recurrent")


func _get_separator_duration() -> VSeparator:
	return _get_node_safe("VSeparator_Duration")


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

	var texture_button_close = _get_texture_button_close()
	if texture_button_close:
		if not texture_button_close.pressed.is_connected(_on_texture_button_close_pressed):
			texture_button_close.pressed.connect(_on_texture_button_close_pressed)

	var button_jump_in = _get_button_jump_in()
	if button_jump_in:
		if not button_jump_in.pressed.is_connected(_on_button_jump_in_pressed):
			button_jump_in.pressed.connect(_on_button_jump_in_pressed)

	var button_jump_to_event = _get_button_jump_to_event()
	var button_jump_to_event_small = _get_button_jump_to_event_small()
	if button_jump_to_event:
		if not button_jump_to_event.pressed.is_connected(_on_button_jump_to_event_pressed):
			button_jump_to_event.pressed.connect(_on_button_jump_to_event_pressed)
	if button_jump_to_event_small:
		if not button_jump_to_event_small.pressed.is_connected(_on_button_jump_to_event_pressed):
			button_jump_to_event_small.pressed.connect(_on_button_jump_to_event_pressed)

	var button_share = _get_button_share()
	if button_share:
		if not button_share.pressed.is_connected(_on_button_share_pressed):
			button_share.pressed.connect(_on_button_share_pressed)


func set_location(_location: Vector2i):
	var label = _get_label_location()
	var texture_rect_location = _get_texture_rect_location()
	var texture_rect_server = _get_texture_rect_server()
	var event_location_coords = _get_label_event_location_coords()
	if label:
		location = _location
		label.text = "%s, %s" % [_location.x, _location.y]
	if event_location_coords:
		event_location_coords.text = "(%s,%s)" % [_location.x, _location.y]
	if texture_rect_location and texture_rect_server:
		texture_rect_location.show()
		texture_rect_server.hide()


func set_scene_event_name(scene_name: String) -> void:
	var event_location_name = _get_label_event_location_name()
	if event_location_name:
		event_location_name.text = scene_name


func set_world(world: String):
	# Solo actualiza la UI (etiqueta, icono). El realm para teleport se fija en set_data desde server.
	var label = _get_label_location()
	var texture_rect_location = _get_texture_rect_location()
	var texture_rect_server = _get_texture_rect_server()
	var event_location_coords = _get_label_event_location_coords()
	if label:
		label.text = format_name(world, 7)
	if event_location_coords:
		event_location_coords.text = format_name(world, 30)
	if texture_rect_location and texture_rect_server:
		texture_rect_location.hide()
		texture_rect_server.show()


func format_name(full: String, max_len := 7) -> String:
	var base := full.trim_suffix(".dcl.eth")
	if base.length() > max_len:
		return base.substr(0, max_len) + "..."
	return base


func set_image(_texture: Texture2D):
	show_image_container(true)
	var texture_rect = _get_texture_image()
	if texture_rect:
		texture_rect.texture = _texture


func set_title(_title: String):
	var label = _get_label_title()
	var rtl_title = _get_rich_label_title()
	if label:
		label.text = _title
	if rtl_title:
		rtl_title.text = _title


func set_description(_description: String):
	description = _description
	var label = _get_label_description()
	if label:
		label.text = _description


func set_likes_percent(_likes: float):
	var label = _get_label_likes()
	var container = _get_container_likes()
	if label and container:
		container.set_visible(_likes > 0.0)
		label.text = str(int(round(_likes * 100))) + "%"
		_update_separators()


func set_online(_online: int):
	var label = _get_label_online()
	var container = _get_container_online()
	if label and container:
		container.set_visible(_online > 0)
		label.text = _format_number(_online)
		_update_separators()


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

	var event_scene_name = _get_or_empty_string(item_data, "scene_name")
	set_scene_event_name(event_scene_name)
	set_description(_get_or_empty_string(item_data, "description"))

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

	# Set location and realm for Jump In (teleport: mismo realm = solo posición; otro realm = cambiar realm + posición)
	var server = item_data.get("server", null)
	var world_name = item_data.get("world_name", null)
	var is_world = item_data.get("world", false)

	# Realm: "main"/null → Genesis City; si no, world (ej. fractilians.dcl.eth). Events usan "server", places pueden usar "world_name".
	if server and str(server) != "main":
		var realm_id = str(server)
		if not realm_id.ends_with(".dcl.eth"):
			realm_id = realm_id + ".dcl.eth"
		realm = realm_id
		set_world(server)
	elif is_world and world_name:
		var wn = str(world_name)
		if wn.ends_with(".dcl.eth"):
			realm = wn
		else:
			realm = wn + ".dcl.eth"
		set_world(world_name)
	else:
		realm = DclUrls.main_realm()

	# Coordenadas: para Jump In siempre; para UI solo si no es world (world muestra icono map + nombre trimmeado)
	var parsed_loc: Vector2i = Vector2i.ZERO
	var coordinates = item_data.get("coordinates", null)
	var position = item_data.get("position", null)
	var base_position = item_data.get("base_position", null)
	if coordinates is Array and coordinates.size() >= 2:
		parsed_loc = Vector2i(int(coordinates[0]), int(coordinates[1]))
	elif position is Array and position.size() >= 2:
		parsed_loc = Vector2i(int(position[0]), int(position[1]))
	elif item_data.get("x") != null and item_data.get("y") != null:
		parsed_loc = Vector2i(int(item_data.x), int(item_data.y))
	elif base_position:
		var location_vector = str(base_position).split(",")
		if location_vector.size() >= 2:
			parsed_loc = Vector2i(int(location_vector[0]), int(location_vector[1]))
	location = parsed_loc
	var is_world_place = (server and str(server) != "main") or is_world
	if not is_world_place:
		set_location(parsed_loc)
	# Si es world, la UI ya quedó con icono map + nombre trimmeado en set_world()

	set_attending(item_data.get("attending", false), event_id, event_tags)
	_update_reminder_and_jump_buttons()
	set_user_name(item_data.get("user_name", ""))
	var like_score = item_data.get("like_score", 0.0)
	set_likes_percent(like_score if like_score is float else 0.0)
	set_online(item_data.get("user_count", 0))
	set_duration(item_data.get("duration", 0))
	set_recurrent(_get_or_empty_string(item_data, "recurrent_frequency"))
	set_recurrent_dates(item_data)
	if _get_texture_image():
		var image_url = item_data.get("image", "")
		if not image_url.is_empty():
			_async_download_image(image_url)
		else:
			show_image_container(false)

	set_creator(_get_or_empty_string(item_data, "contact_name"))

	if engagement_bar:
		engagement_bar.update_data(_data)

	set_download_warning(item_data)

	var calendar_btn = _get_button_calendar()
	if calendar_btn:
		calendar_btn.set_data(item_data)
		calendar_btn.next_event = true


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
	_do_jump_in()


func _get_jump_in_position_and_realm_from_data(item_data: Dictionary) -> Array:
	# Devuelve [Vector2i posición, String realm] para Jump In (eventos y places).
	var pos: Vector2i = Vector2i.ZERO
	var r: String = ""
	var server = item_data.get("server", null)
	var world_name = item_data.get("world_name", null)
	var is_world = item_data.get("world", false)
	if server and str(server) != "main":
		r = str(server)
		if not r.ends_with(".dcl.eth"):
			r = r + ".dcl.eth"
	elif is_world and world_name:
		r = str(world_name)
		if not r.ends_with(".dcl.eth"):
			r = r + ".dcl.eth"
	else:
		r = DclUrls.main_realm()
	var coords = item_data.get("coordinates", null)
	var pos_arr = item_data.get("position", null)
	var base_pos = item_data.get("base_position", null)
	if coords is Array and coords.size() >= 2:
		pos = Vector2i(int(coords[0]), int(coords[1]))
	elif typeof(coords) == TYPE_STRING:
		var parts = str(coords).split(",")
		if parts.size() >= 2:
			pos = Vector2i(int(parts[0]), int(parts[1]))
	elif pos_arr is Array and pos_arr.size() >= 2:
		pos = Vector2i(int(pos_arr[0]), int(pos_arr[1]))
	elif item_data.get("x") != null and item_data.get("y") != null:
		pos = Vector2i(int(item_data.x), int(item_data.y))
	elif base_pos:
		var parts = str(base_pos).split(",")
		if parts.size() >= 2:
			pos = Vector2i(int(parts[0]), int(parts[1]))
	return [pos, r]


func _do_jump_in() -> void:
	# Misma lógica para Jump In y Jump to event: teleport a position + realm. Resolver desde _data al hacer click por si set_data no aplicó (ej. panel detalles desde tarjeta).
	var jump_pos := location
	var jump_realm := realm
	if _data is Dictionary and not _data.is_empty():
		var pos_realm = _get_jump_in_position_and_realm_from_data(_data)
		if pos_realm[0] != Vector2i.ZERO or not pos_realm[1].is_empty():
			jump_pos = pos_realm[0]
			jump_realm = pos_realm[1]
	jump_in.emit(jump_pos, jump_realm)


func _on_button_close_pressed() -> void:
	close.emit()


func _on_texture_button_close_pressed() -> void:
	var parent = get_parent()
	if parent.has_method("_close"):
		parent._close()
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
		rtl.set_text_trimmed(_event_name)

	var label_user_name = _get_label_user_name()
	if label_user_name:
		label_user_name.text = _user_name


func set_duration(_duration: int) -> void:
	var duration_label = _get_duration_label()
	if duration_label:
		duration_label.text = _format_duration(_duration)


func set_recurrent(_recurrent_frequency: String) -> void:
	var label = _get_recurrent_label()
	var separator_recurrent = _get_separator_recurrent()
	var separator_duration = _get_separator_duration()
	if label:
		if _recurrent_frequency != "":
			label.get_parent().show()
			separator_recurrent.show()
			separator_duration.show()
			label.text = _recurrent_frequency.capitalize()
		else:
			label.get_parent().hide()
			separator_recurrent.hide()
			separator_duration.hide()


func set_recurrent_dates(item_data: Dictionary) -> void:
	var recurrent_dates = item_data.get("recurrent_dates", [])
	if recurrent_dates.size() < 1:
		return
	var recurrent_dates_container = _get_recurrent_dates_container()
	if not recurrent_dates_container:
		return
	for child in recurrent_dates_container.get_children():
		recurrent_dates_container.remove_child(child)
		child.queue_free()

	for i in recurrent_dates.size():
		var btn: CalendarButton = _CALENDAR_BUTTON_SCENE.instantiate() as CalendarButton
		btn.set_index(i)
		btn.set_data(item_data)
		btn.next_event = false
		recurrent_dates_container.add_child(btn)


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
		event_start_timestamp > 0 and time_until_start_sec > 0 and time_until_start_sec < 300
	)
	var show_jump_hide_reminder = is_live or starts_in_less_than_5_mins

	if show_jump_hide_reminder:
		reminder_btn.hide()
		jump_btn.show()
	else:
		reminder_btn.show()
		jump_btn.hide()


func set_categories(_categories: Array) -> void:
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


func _parse_iso_timestamp_utc(iso_string: String) -> int:
	## Parse ISO string (UTC, e.g. "2025-05-31T19:00:00.000Z") and return Unix timestamp (UTC).
	if iso_string.is_empty():
		return 0
	var local_unix: int = _parse_iso_timestamp(iso_string)
	var tz: Dictionary = Time.get_time_zone_from_system()
	var bias_minutes: int = tz.get("bias", 0)
	return local_unix - (bias_minutes * 60)


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


func _on_event_pressed() -> void:
	# Emitir datos completos del evento para que card y detalles muestren lo mismo (LIVE, IN X DAYS)
	if _data is Dictionary and not _data.is_empty():
		event_pressed.emit(_data)
	else:
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
	var btn = _get_button_calendar()
	if btn is CalendarButton:
		btn.add_event_to_calendar()


func _on_button_jump_to_event_pressed() -> void:
	Global.metrics.track_click_button(
		"jump_to", "EVENT_DETAILS", JSON.stringify({"event_id": event_id, "event_tag": event_tags})
	)
	_do_jump_in()


func _on_show_more_toggled(toggled_on: bool) -> void:
	var description_container = _get_description()
	var show_more = _get_show_more_container()
	var card = _get_card()
	var header = _get_header()

	if description_container and header and card and show:
		if toggled_on:
			description_container.show()
			header.show()
			card.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
			_set_card_corner_radius(card, 0, 0)
			show_more.hide()
		else:
			description_container.hide()
			header.hide()
			card.set_anchors_and_offsets_preset(Control.PRESET_BOTTOM_WIDE)
			_set_card_corner_radius(card, 24, 24)
			show_more.show()


func _set_card_corner_radius(card: PanelContainer, top_left: int, top_right: int) -> void:
	var current_style = card.get_theme_stylebox("panel")
	if current_style is StyleBoxFlat:
		var style_box := current_style.duplicate() as StyleBoxFlat
		style_box.corner_radius_top_left = top_left
		style_box.corner_radius_top_right = top_right
		card.add_theme_stylebox_override("panel", style_box)


func show_image_container(toggle: bool) -> void:
	var image_container = _get_image_container()
	var no_image_container = _get_no_image_container()
	if image_container and no_image_container:
		if toggle:
			image_container.show()
			no_image_container.hide()
		else:
			image_container.hide()
			no_image_container.show()


func set_fav_button_data(_id: String) -> void:
	var fav_button = _get_fav_button()
	if fav_button:
		fav_button.update_data(_id)


func _update_separators() -> void:
	var separator_likes = _get_separator_likes()
	var separator_online = _get_separator_online()
	var container_likes = _get_container_likes()
	var container_online = _get_container_online()
	if container_likes and container_online:
		if separator_likes and separator_online:
			if container_likes.visible and container_online.visible:
				separator_likes.show()
				separator_online.show()
			else:
				separator_likes.hide()
				separator_online.hide()
