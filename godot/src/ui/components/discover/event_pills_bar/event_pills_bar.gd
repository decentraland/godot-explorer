extends HBoxContainer

const TIME_PILL_BLACK = preload("res://src/ui/components/events/time_pill_black.tres")
const TIME_PILL_RED = preload("res://src/ui/components/events/time_pill_red.tres")

var live: bool = false
var next_start_at = ""
var event_status = ""

@onready var live_pill: PanelContainer = %LivePill
@onready var label_live_pill: Label = %Label_LivePill
@onready var time_pill: PanelContainer = %TimePill
@onready var label_time_pill: Label = %Label_TimePill
@onready var users_pill: PanelContainer = %UsersPill
@onready var label_users_in_event: Label = %Label_UsersInEvent
@onready var featured_pill: PanelContainer = %FeaturedPill
@onready var trending_pill: PanelContainer = %TrendingPill


func set_data(item_data) -> void:
	live = item_data.get("live", false)
	next_start_at = item_data.get("next_start_at", "")

	event_status = "live" if live else "upcoming"
	# Usar el flag "live" del API como fuente de verdad para mostrar LIVE (unifica card y detalles)
	if live:
		_show_live_state()
		set_trending(item_data.get("trending", false))
		set_featured(item_data.get("highlighted", false))
		set_users_in_event(item_data.get("user_count", 0))
		return
	if next_start_at != "":
		var timestamp = _parse_iso_timestamp(next_start_at)
		if timestamp > 0:
			set_time(timestamp)
	set_trending(item_data.get("trending", false))
	set_featured(item_data.get("highlighted", false))
	set_users_in_event(item_data.get("user_count", 0))


func set_trending(_trending: bool) -> void:
	trending_pill.set_visible(_trending)


func set_featured(_featured: bool) -> void:
	featured_pill.set_visible(_featured)


func set_users_in_event(_users: int = 0) -> void:
	if _users == 0:
		users_pill.hide()
		return
	label_users_in_event.text = str(_users)


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


func _show_live_state() -> void:
	label_live_pill.text = "LIVE"
	live_pill.show()
	time_pill.hide()
	if users_pill:
		users_pill.show()
	live_pill.get_parent().show()


func set_time(_start_at: int) -> void:
	var now = Time.get_unix_time_from_system()
	# Si ya empezó (tiempo <= 0) → mostrar LIVE y USERS, ocultar TIME
	if _start_at <= now:
		_show_live_state()
		return
	# Si va a empezar en el futuro → mostrar TIME (borde rojo si <= 5 min, negro si > 5 min)
	live_pill.hide()
	time_pill.show()
	var time_text = _format_timestamp(_start_at)
	label_time_pill.text = time_text
	label_live_pill.text = time_text


func _format_timestamp(timestamp: int) -> String:
	var now = Time.get_unix_time_from_system()
	var time_diff = timestamp - now

	# Solo se llama con evento en el futuro (time_diff > 0). TIME pill visible; borde según tiempo restante.
	var time_pill_style = time_pill.get_theme_stylebox("panel")
	if time_pill_style:
		var unique_style = time_pill_style.duplicate()
		# Borde rojo si faltan 5 minutos o menos, negro si más de 5 min
		if time_diff <= 300:
			unique_style.border_color = Color("#ff2d55")
			label_time_pill.label_settings = TIME_PILL_RED
		else:
			unique_style.border_color = Color("#161518")
			label_time_pill.label_settings = TIME_PILL_BLACK
		time_pill.add_theme_stylebox_override("panel", unique_style)

	var minutes_diff = time_diff / 60
	var hours_diff = time_diff / 3600
	var days_diff = time_diff / 86400

	# Menos de 1 hora: IN XX MINUTES
	if hours_diff < 1:
		return "IN " + str(int(minutes_diff)) + " MINS"

	# Menos de 48 horas: IN XX HOURS
	if hours_diff < 48:
		if hours_diff > 2:
			return "IN " + str(int(hours_diff)) + " HRS"
		return "IN " + str(int(hours_diff)) + " HR"

	# 7 días o menos: IN X DAYS
	if days_diff <= 7:
		return "IN " + str(int(days_diff)) + " DAYS"

	# Más de 7 días: fecha tipo SEPT 31
	var time_dict = Time.get_datetime_dict_from_unix_time(timestamp)
	var month_names = [
		"", "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC"
	]
	return month_names[time_dict.month] + " " + str(time_dict.day)
