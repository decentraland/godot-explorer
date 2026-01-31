class_name CalendarButton
extends HBoxContainer

@export var next_event: bool = true

var data: Dictionary = {}
var _index: int = 0
var _start_iso: String = ""
var _duration_ms: int = 0
var _event_name: String = "Decentraland Event"
var _event_description: String = ""
var _event_location: Vector2i = Vector2i.ZERO
var _server: String = "main"

var _weekday_short: PackedStringArray = PackedStringArray(
	["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]
)
var _month_short: PackedStringArray = PackedStringArray(
	["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]
)

@onready var h_box_container_text: HBoxContainer = $HBoxContainer_Text
@onready var label_day: Label = $HBoxContainer_Text/Label_Day
@onready var label_time: Label = $HBoxContainer_Text/Label_Time
@onready var button: Button = $Button


func _ready() -> void:
	button.pressed.connect(_on_pressed)
	_update_labels()


func set_index(idx: int) -> void:
	_index = idx
	_update_labels()


func set_event(start_iso: String, duration_ms: int) -> void:
	_start_iso = start_iso
	_duration_ms = duration_ms
	_update_labels()


func set_event_metadata(
	event_name: String, event_description: String, event_location: Vector2i, server: String = "main"
) -> void:
	_event_name = event_name
	_event_description = event_description
	_event_location = event_location
	_server = server


func set_data(value: Dictionary) -> void:
	data = value
	_duration_ms = value.get("duration", 0)
	_event_name = value.get("name", "Decentraland Event")
	_event_description = value.get("description", "")
	_server = _realm_param_from_data(value)
	var coords: Array = value.get("coordinates", [0, 0])
	if coords.size() >= 2:
		_event_location = Vector2i(int(coords[0]), int(coords[1]))
	else:
		var base: String = value.get("base_position", "0,0")
		var parts: PackedStringArray = base.split(",")
		if parts.size() >= 2:
			_event_location = Vector2i(int(parts[0]), int(parts[1]))
	_update_labels()


func _realm_param_from_data(value: Dictionary) -> String:
	var server_val = value.get("server", null)
	if server_val == null:
		return "main"
	if server_val is String and (server_val == "" or server_val == "main"):
		return "main"
	return str(server_val)


func _get_start_iso() -> String:
	if not _start_iso.is_empty():
		return _start_iso
	if data.is_empty():
		return ""
	if next_event:
		return data.get("next_start_at", "")
	var recurrent_dates: Array = data.get("recurrent_dates", [])
	if _index >= 0 and _index < recurrent_dates.size():
		var val = recurrent_dates[_index]
		return val if val is String else str(val)
	return ""


func _update_labels() -> void:
	if not is_node_ready():
		call_deferred("_update_labels")
		return
	if not label_day or not label_time or not h_box_container_text:
		return
	h_box_container_text.visible = false
	if next_event:
		return

	var start_iso: String = _get_start_iso()
	if start_iso.is_empty():
		label_day.text = ""
		label_time.text = ""
		return
	var start_timestamp_sec: int = _parse_iso_timestamp(start_iso)
	if start_timestamp_sec <= 0:
		label_day.text = ""
		label_time.text = ""
		return
	var now_sec: int = int(Time.get_unix_time_from_system())
	if start_timestamp_sec < now_sec and _index != -1:
		queue_free()
		return
	h_box_container_text.visible = true
	label_day.text = _format_day(start_timestamp_sec)
	label_time.text = _format_time_range(start_timestamp_sec, _duration_ms)


func _format_day(unix_sec: int) -> String:
	var dt: Dictionary = Time.get_datetime_dict_from_unix_time(unix_sec)
	var wd: int = dt.get("weekday", 0)
	var month: int = dt.get("month", 1)
	var day: int = dt.get("day", 1)
	return (
		"%s, %s %d."
		% [_weekday_short[clampi(wd, 0, 6)], _month_short[clampi(month - 1, 0, 11)], day]
	)


func _format_time_range(start_unix_sec: int, duration_ms: int) -> String:
	var start_dt: Dictionary = Time.get_datetime_dict_from_unix_time(start_unix_sec)
	var end_unix_sec: int = start_unix_sec + (duration_ms / 1000)
	var end_dt: Dictionary = Time.get_datetime_dict_from_unix_time(end_unix_sec)
	var tz_str: String = _get_local_timezone_string()
	return (
		"%s to %s %s"
		% [
			_format_12h(start_dt.hour, start_dt.minute),
			_format_12h(end_dt.hour, end_dt.minute),
			tz_str
		]
	)


func _format_12h(hour: int, minute: int) -> String:
	var h: int = 12 if (hour == 0 or hour == 12) else (hour - 12 if hour >= 12 else hour)
	var am_pm: String = "pm" if hour >= 12 else "am"
	return "%02d:%02d%s" % [h, minute, am_pm]


func _get_local_timezone_string() -> String:
	var bias_minutes: int = Time.get_time_zone_from_system().get("bias", 0)
	var offset_hours: int = bias_minutes / 60
	if offset_hours >= 0:
		return "(UTC+%d)" % offset_hours
	return "(UTC%d)" % offset_hours


func _on_pressed() -> void:
	add_event_to_calendar()


func add_event_to_calendar() -> void:
	var start_iso: String = _get_start_iso()
	if start_iso.is_empty():
		return
	var start_timestamp_sec: int = _iso_to_utc_unix(start_iso)
	if start_timestamp_sec <= 0:
		return
	var start_time_millis: int = start_timestamp_sec * 1000
	var end_time_millis: int = start_time_millis + _duration_ms
	# El calendario del sistema trata el timestamp como hora local y aplica -bias para guardar UTC.
	# Compensamos pasando un timestamp +(-bias) segundos para que la hora mostrada sea la correcta.
	var bias_minutes: int = Time.get_time_zone_from_system().get("bias", 0)
	var compensation_ms: int = (-bias_minutes * 60) * 1000
	var start_time_millis_for_calendar: int = start_time_millis + compensation_ms
	var end_time_millis_for_calendar: int = end_time_millis + compensation_ms
	var description_str: String = _event_description
	if not description_str.is_empty():
		description_str += "\n\n"
	description_str += "jump in: " + _build_jump_in_url(_event_location)
	var event_location_str: String = (
		"Decentraland at %d,%d" % [_event_location.x, _event_location.y]
	)
	if DclAndroidPlugin.is_available():
		DclAndroidPlugin.add_calendar_event(
			_event_name,
			description_str,
			start_time_millis_for_calendar,
			end_time_millis_for_calendar,
			event_location_str
		)
	elif DclIosPlugin.is_available():
		DclIosPlugin.add_calendar_event(
			_event_name,
			description_str,
			start_time_millis_for_calendar,
			end_time_millis_for_calendar,
			event_location_str
		)


func _build_jump_in_url(loc: Vector2i) -> String:
	var realm_param: String = (
		"main" if (_server == "" or _server == "main") else _server.uri_encode()
	)
	return DclUrls.jump_events() + "?position=%d%%2C%d&realm=%s" % [loc.x, loc.y, realm_param]


func _parse_iso_timestamp(iso_string: String) -> int:
	var utc_unix: int = _iso_to_utc_unix(iso_string)
	if utc_unix <= 0:
		return 0
	return utc_unix


func _iso_to_utc_unix(iso_string: String) -> int:
	if iso_string.is_empty():
		return 0
	var date_parts: PackedStringArray = iso_string.split("T")
	if date_parts.size() != 2:
		return 0
	var date_part: String = date_parts[0]
	var time_part: String = date_parts[1].replace("Z", "").split(".")[0]
	var date_components: PackedStringArray = date_part.split("-")
	var time_components: PackedStringArray = time_part.split(":")
	if date_components.size() != 3 or time_components.size() != 3:
		return 0
	var date_dict: Dictionary = {
		"year": int(date_components[0]),
		"month": int(date_components[1]),
		"day": int(date_components[2]),
		"hour": int(time_components[0]),
		"minute": int(time_components[1]),
		"second": int(time_components[2])
	}
	var local_unix: int = Time.get_unix_time_from_datetime_dict(date_dict)
	var bias_minutes: int = Time.get_time_zone_from_system().get("bias", 0)
	return local_unix + (bias_minutes * 60)
