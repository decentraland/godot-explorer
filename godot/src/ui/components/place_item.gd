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
@export var realm: String = Realm.MAIN_REALM
@export var realm_title: String = "Genesis City"

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
		label.text = str(round(_likes * 100)) + "%"


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
	set_attending(item_data.get("attending", false), item_data.get("id", "id"))
	set_event_name(item_data.get("name", "Event Name"))
	set_views(item_data.get("user_visits", 0))
	var like_score = item_data.get("like_score", 0.0)
	set_likes_percent(like_score if like_score is float else 0.0)
	set_online(item_data.get("user_count", 0))
	set_trending(item_data.get("trending", false))
	set_duration(item_data.get("duration", 0))
	set_recurrent(item_data.get("recurrent", false))
	
	# Manejar start_at para eventos (timestamp Unix)
	var next_start_at = item_data.get("next_start_at", "")
	var live = item_data.get("live", false)
	if next_start_at != "":
		# Convertir ISO string a timestamp Unix
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
		
	var event_location_vector = item_data.get("coordinates", [0,0])
	if event_location_vector.size() == 2:
		set_event_location(Vector2i(int(event_location_vector[0]), int(event_location_vector[1])))

	set_creator(_get_or_empty_string(item_data, "contact_name"))
	set_user_name(_get_or_empty_string(item_data, "user_name"))
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
		return str(ceil(num / 1000.0)) + "k"
	return str(floor(num / 1000000.0)) + "M"


func get_hash_from_url(url: String) -> String:
	if url.contains("/content/contents/"):
		var parts = url.split("/")
		return parts[parts.size() - 1]  # Return the last part

	# Convert URL to a hexadecimal
	var context := HashingContext.new()
	if context.start(HashingContext.HASH_SHA256) == OK:
		# Convert the URL string to UTF-8 bytes and update the context with this data
		context.update(url.to_utf8_buffer())
		# Finalize the hashing process and get the hash as a PackedByteArray
		var url_hash: PackedByteArray = context.finish()
		# Encode the hash as hexadecimal
		return url_hash.hex_encode()

	return "temp-file"


func _get_or_empty_string(dict: Dictionary, key: String) -> String:
	var value = dict.get(key, null)
	if value is String:
		return value
	return ""


func set_event_name(_event_name: String) -> void:
	event_name = _event_name
	var label = _get_label_event_name()
	if label:
		label.text = _event_name


func set_trending(_trending: bool) -> void:
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
	var jump_in_button = _get_button_jump_in()
	var reminder_button = _get_reminder_button()
	
	if time_pill and live_pill:
		if live:
			live_pill.text = "LIVE"
			if jump_in_button and reminder_button:
				jump_in_button.show()
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


func set_attending(_attending: bool, _id: String) -> void:
	var reminder_button = _get_reminder_button()
	if reminder_button:
		reminder_button.button_pressed = _attending
		reminder_button.event_id_value = _id

func _parse_iso_timestamp(iso_string: String) -> int:
	# Convertir ISO string (ej: "2025-10-06T12:00:00.000Z") a timestamp Unix
	if iso_string.is_empty():
		return 0

	# Parsear la fecha ISO
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

	# Crear diccionario de fecha y convertir a timestamp
	var date_dict = {
		"year": year, "month": month, "day": day, "hour": hour, "minute": minute, "second": second
	}

	return Time.get_unix_time_from_datetime_dict(date_dict)


func _format_timestamp_for_calendar(timestamp: int) -> String:
	# Convertir timestamp Unix a formato ISO para Google Calendar (YYYYMMDDTHHMMSSZ)
	var time_dict = Time.get_datetime_dict_from_unix_time(timestamp)
	return "%04d%02d%02dT%02d%02d%02dZ" % [
		time_dict.year,
		time_dict.month,
		time_dict.day,
		time_dict.hour,
		time_dict.minute,
		time_dict.second
	]


func _format_timestamp(timestamp: int) -> String:
	var now = Time.get_unix_time_from_system()
	var time_diff = timestamp - now

	# Si el evento ya pasó, mostrar fecha
	if time_diff <= 0:
		var time_dict = Time.get_datetime_dict_from_unix_time(timestamp)
		var month_names = [
			"", "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC"
		]
		return month_names[time_dict.month] + " " + str(time_dict.day)

	# Calcular diferencias
	var minutes_diff = time_diff / 60
	var hours_diff = time_diff / 3600
	var days_diff = time_diff / 86400

	var live_pill_parent = _get_label_live_pill().get_parent()
	var time_pill_parent = _get_label_time_pill().get_parent()
	var border = _get_border()
	var jump_in_button = _get_button_jump_in()
	var reminder_button = _get_reminder_button()

	if minutes_diff < 5:
		live_pill_parent.show()
		time_pill_parent.hide()
		if jump_in_button and reminder_button:
			jump_in_button.show()
			reminder_button.hide()
		if border:
			border.self_modulate = "#FFFFFF"
		return "IN " + str(int(minutes_diff)) + " MINS"
	else:
		live_pill_parent.hide()
		time_pill_parent.show()
		if jump_in_button and reminder_button:
			jump_in_button.hide()
			reminder_button.show()
		if border:
			border.self_modulate = "#FFFFFF00"
		
	# Si falta menos de 1 hora: IN XX MINUTES
	if hours_diff < 1:
		return "IN " + str(int(minutes_diff)) + " MINS"

	# Si faltan menos de 48 horas: IN XX HOURS
	if hours_diff < 48:
		if hours_diff > 2:
			return "IN " + str(int(hours_diff)) + " HRS"
		return "IN " + str(int(hours_diff)) + " HR"

	# Si faltan 7 días o menos: IN X DAYS
	if days_diff <= 7:
		return "IN " + str(int(days_diff)) + " DAYS"

	# Si faltan más de 7 días: Poner la fecha con formato SEPT 31
	var time_dict = Time.get_datetime_dict_from_unix_time(timestamp)
	var month_names = [
		"", "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC"
	]
	return month_names[time_dict.month] + " " + str(time_dict.day)


func _format_duration(duration: int) -> String:
	# Convertir milisegundos a horas
	var hours = duration / (1000 * 60 * 60)
	
	# Si es menor a 1 hora, mostrar minutos
	if hours < 1:
		var minutes = duration / (1000 * 60)
		if minutes == 1:
			return "1 MIN"
		else:
			return str(int(minutes)) + " MINS"
	
	# Si es menor a 72 horas, mostrar horas
	if hours < 72:
		if hours == 1:
			return "1 HR"
		else:
			return str(int(hours)) + " HRS"
	
	# Si es mayor a 72 horas, mostrar días
	var days = hours / 24
	return str(int(days)) + " DAYS"


func schedule_event() -> void:
	if not _data:
		return
	
	# Obtener datos del evento
	var next_start_at = _data.get("next_start_at", "")
	var next_finish_at = _data.get("next_finish_at", "")
	
	# Crear URL de jump in con las coordenadas de location
	var jump_in_url = "https://decentraland.org/jump/events?position=%d%%2C%d&realm=main" % [location.x, location.y]
	
	# Combinar descripción con URL de jump in
	var details = description
	if not description.is_empty():
		details += "\n\n"
	details += "jump in: " + jump_in_url
	
	# Crear fechas para Google Calendar
	var dates_param = ""
	if not next_start_at.is_empty() and not next_finish_at.is_empty():
		var start_timestamp = _parse_iso_timestamp(next_start_at)
		var finish_timestamp = _parse_iso_timestamp(next_finish_at)
		
		if start_timestamp > 0 and finish_timestamp > 0:
			var start_iso = _format_timestamp_for_calendar(start_timestamp)
			var finish_iso = _format_timestamp_for_calendar(finish_timestamp)
			dates_param = "&dates=%s/%s" % [start_iso, finish_iso]
	
	# Crear deep link de Google Calendar
	var calendar_url = "https://calendar.google.com/calendar/u/0/r/eventedit?text=%s&details=%s%s" % [
		event_name.uri_encode(),
		details.uri_encode(),
		dates_param
	]
	
	# Abrir el enlace en el navegador
	OS.shell_open(calendar_url)


func _on_event_pressed() -> void:
	event_pressed.emit(_data)


func _on_button_share_pressed() -> void:
	pass


func _on_button_calendar_pressed() -> void:
	schedule_event()
