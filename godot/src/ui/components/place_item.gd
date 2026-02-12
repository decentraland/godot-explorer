class_name PlaceItem
extends Control

signal item_pressed(data)
signal event_pressed(data)
signal jump_in(position: Vector2i, realm: String)
signal jump_in_world(realm: String)
signal close

enum DragState { HIDDEN, HALF, FULL }
enum DragGesture { IDLE, UP, DOWN }

const _CALENDAR_BUTTON_SCENE: PackedScene = preload(
	"res://src/ui/components/calendar_button/calendar_button.tscn"
)
const _TWEEN_DURATION := 0.2

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
@export var is_draggable := false

var event_id: String
var event_status: String
var event_tags: String
var event_start_timestamp: int = 0
var texture_placeholder = load("res://assets/ui/placeholder.png")
var start_pos: Vector2
var initial_pos: Vector2
var drag_tween: Tween
var drag_state := DragState.HALF
var dragging: bool = false

var _data: Dictionary = {}
var _node_cache: Dictionary = {}
var _tween_callback: Callable
var _tween_header_visible: bool


# gdlint:ignore = async-function-name
func _ready():
	mouse_filter = Control.MOUSE_FILTER_PASS
	UiSounds.install_audio_recusirve(self)
	_connect_signals()

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
		# NOTE must call deferred
		# otherwise the UI is broken
		card.set_anchors_and_offsets_preset.call_deferred(Control.PRESET_FULL_RECT)
		card.set_position.call_deferred(Vector2(0, _get_card_hidden_position()))

	var description_container = _get_hide_from_here()
	if description_container:
		description_container.show()

	if is_draggable and card:
		var header = _get_header()
		if header:
			header.self_modulate = Color.TRANSPARENT
			header.hide()

		await get_tree().process_frame
		await get_tree().process_frame
		card.position.y = _get_card_hidden_position()
		tween_to(_get_card_half_position())


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


func _get_hide_from_here() -> HSeparator:
	return _get_node_safe("HSeparator_HideFromHere")


func _get_description_scroll() -> VBoxContainer:
	return _get_node_safe("ScrollDescription")


func _get_card() -> PanelContainer:
	return _get_node_safe("PanelContainer_Card")


func _get_header() -> PanelContainer:
	return _get_node_safe("PanelContainer_Header")


func _get_footer() -> PanelContainer:
	return _get_node_safe("PanelContainer_Footer")


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


func _get_container_location() -> HBoxContainer:
	return _get_node_safe("HBoxContainer_Location")


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


func set_server_or_location(unlimited: bool = false) -> void:
	var server = _data.get("server", null)
	var world_name = _data.get("world_name", null)
	var is_world = _data.get("world", false)
	if server and str(server) != "main":
		var realm_id = str(server)
		if not realm_id.ends_with(".dcl.eth"):
			realm_id = realm_id + ".dcl.eth"
		realm = realm_id
		set_world(server, unlimited)
	elif is_world and world_name:
		var wn = str(world_name)
		if wn.ends_with(".dcl.eth"):
			realm = wn
		else:
			realm = wn + ".dcl.eth"
		set_world(world_name, unlimited)
	else:
		realm = DclUrls.main_realm()

	location = _parse_position_from_item(_data)
	var is_world_place = (server and str(server) != "main") or is_world
	if not is_world_place:
		set_location(location)


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


func set_world(world: String, unlimited: bool = false):
	var label = _get_label_location()
	var texture_rect_location = _get_texture_rect_location()
	var texture_rect_server = _get_texture_rect_server()
	var event_location_coords = _get_label_event_location_coords()
	if label:
		if unlimited:
			label.text = world
		else:
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
		max_size_mb = 100
	else:
		var positions = item_data.get("positions", [])
		var parcel_count = positions.size() if positions is Array else 1
		if parcel_count == 0:
			parcel_count = 20
		max_size_mb = mini(parcel_count * 15, 300)

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
	set_fav_button_data(item_data.get("id", "-"))
	set_engagement_bar_data(item_data.get("id", "-"))
	var next_start_at = item_data.get("next_start_at", "")
	var live = item_data.get("live", false)
	event_status = "live" if live else "upcoming"
	if next_start_at != "":
		var timestamp = _parse_iso_timestamp(next_start_at)
		if timestamp > 0:
			event_start_timestamp = timestamp

	set_server_or_location()

	var reminder_btn = _get_reminder_button()
	if reminder_btn:
		reminder_btn.set_data(_data)
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


static func _parse_position_from_item(item_data: Dictionary) -> Vector2i:
	var coords = item_data.get("coordinates", null)
	var pos_arr = item_data.get("position", null)
	var base_pos = item_data.get("base_position", null)
	if coords is Array and coords.size() >= 2:
		return Vector2i(int(coords[0]), int(coords[1]))
	if pos_arr is Array and pos_arr.size() >= 2:
		return Vector2i(int(pos_arr[0]), int(pos_arr[1]))
	if item_data.get("x") != null and item_data.get("y") != null:
		return Vector2i(int(item_data.x), int(item_data.y))
	if base_pos:
		var parts = str(base_pos).split(",")
		if parts.size() >= 2:
			return Vector2i(int(parts[0]), int(parts[1]))
	return Vector2i.ZERO


func _get_jump_in_position_and_realm_from_data(item_data: Dictionary) -> Array:
	var server = item_data.get("server", null)
	var world_name = item_data.get("world_name", null)
	var r: String
	if server and str(server) != "main":
		r = str(server)
		if not r.ends_with(".dcl.eth"):
			r = r + ".dcl.eth"
	elif item_data.get("world", false) and world_name:
		r = str(world_name)
		if not r.ends_with(".dcl.eth"):
			r = r + ".dcl.eth"
	else:
		r = DclUrls.main_realm()
	var pos := _parse_position_from_item(item_data)
	return [pos, r]


static func _is_event_in_world(item_data: Dictionary) -> bool:
	if not item_data is Dictionary or item_data.is_empty():
		return false
	if not item_data.has("duration"):
		return false
	var server = item_data.get("server", null)
	if server == null:
		return false
	var s = str(server).strip_edges()
	return s != "" and s != "main"


func _do_jump_in() -> void:
	if _data is Dictionary and not _data.is_empty() and _is_event_in_world(_data):
		var pos_realm = _get_jump_in_position_and_realm_from_data(_data)
		var world_realm: String = pos_realm[1]
		jump_in_world.emit(world_realm)
		return

	var jump_pos := location
	var jump_realm := realm
	if _data is Dictionary and not _data.is_empty():
		var pos_realm = _get_jump_in_position_and_realm_from_data(_data)
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


func _update_reminder_and_jump_buttons() -> void:
	var reminder_btn = _get_reminder_button()
	var jump_btn = _get_button_jump_to_event()
	if not reminder_btn or not jump_btn:
		return

	var now = Time.get_unix_time_from_system()
	var is_live = _data.get("live", false) if _data is Dictionary else false
	if event_start_timestamp > 0 and now >= event_start_timestamp:
		is_live = true
	var time_until_start_sec: int = (
		int(event_start_timestamp - now) if event_start_timestamp > 0 else 0
	)
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
	if iso_string.is_empty():
		return 0

	var date_parts = iso_string.split("T")
	if date_parts.size() != 2:
		return 0

	var date_part = date_parts[0]
	var time_part = date_parts[1].replace("Z", "").split(".")[0]

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

	var date_dict = {
		"year": year, "month": month, "day": day, "hour": hour, "minute": minute, "second": second
	}

	return Time.get_unix_time_from_datetime_dict(date_dict)


func _parse_iso_timestamp_utc(iso_string: String) -> int:
	if iso_string.is_empty():
		return 0
	var local_unix: int = _parse_iso_timestamp(iso_string)
	var tz: Dictionary = Time.get_time_zone_from_system()
	var bias_minutes: int = tz.get("bias", 0)
	return local_unix - (bias_minutes * 60)


func _format_duration(duration: int) -> String:
	var hours: int = duration / (1000 * 60 * 60)
	if hours < 1:
		var minutes: int = duration / (1000 * 60)
		if minutes == 1:
			return "1 MIN"
		return str(minutes) + " MINS"

	if hours < 72:
		if hours == 1:
			return "1 HR"
		return str(hours) + " HRS"

	var days: int = hours / 24
	return str(days) + " DAYS"


func _on_event_pressed() -> void:
	if _data is Dictionary and not _data.is_empty():
		event_pressed.emit(_data)
	else:
		event_pressed.emit(event_id)


func _extract_short_realm_url(full_url: String) -> String:
	var url_trimmed = full_url.trim_suffix("/")
	var parts = url_trimmed.split("/")
	if parts.size() > 0:
		return parts[parts.size() - 1]
	return full_url


func _is_place_item_event(item_data: Dictionary) -> bool:
	return item_data.has("duration")


func _share_place_or_event() -> void:
	var share_title: String
	var url: String
	var is_event := false

	if _data is Dictionary and not _data.is_empty():
		is_event = _is_place_item_event(_data)
		var pos_realm = _get_jump_in_position_and_realm_from_data(_data)
		var share_pos: Vector2i = pos_realm[0]
		var share_realm: String = pos_realm[1]
		var is_main = share_realm == DclUrls.main_realm()

		if is_main:
			url = (
				"https://mobile.dclexplorer.com/open?position="
				+ str(share_pos.x)
				+ ","
				+ str(share_pos.y)
			)
		else:
			var short_realm = (
				share_realm
				if share_realm.ends_with(".dcl.eth")
				else _extract_short_realm_url(share_realm)
			)
			url = "https://mobile.dclexplorer.com/open?realm=" + short_realm

		share_title = (
			_data.get("name", _data.get("title", "Decentraland"))
			if is_event
			else _data.get("title", "Decentraland")
		)
	else:
		var is_main = realm == DclUrls.main_realm()
		if is_main:
			url = (
				"https://mobile.dclexplorer.com/open?position="
				+ str(location.x)
				+ ","
				+ str(location.y)
			)
		else:
			var short_realm = (
				realm if realm.ends_with(".dcl.eth") else _extract_short_realm_url(realm)
			)
			url = "https://mobile.dclexplorer.com/open?realm=" + short_realm
		share_title = event_name if not event_name.is_empty() else title

	if share_title.is_empty():
		share_title = "Decentraland"

	var msg: String
	if is_event:
		msg = "📍 Visit the event '" + share_title + "' following this link: " + url
	else:
		msg = "📍 Join me at " + share_title + " following this link: " + url

	if Global.is_android():
		DclAndroidPlugin.share_text(msg)
	elif Global.is_ios():
		DclIosPlugin.share_text(msg)


func _on_button_share_pressed() -> void:
	if not _data or _data.is_empty():
		if title.is_empty() and event_name.is_empty():
			printerr("No place or event data available to share")
			return
		_share_place_or_event()
		return

	_share_place_or_event()


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
	var hide_from_here = _get_hide_from_here()
	#var show_more = _get_show_more_container()
	var card = _get_card()
	var header = _get_header()

	if hide_from_here and header and card and show:
		if toggled_on:
			hide_from_here.show()
			_set_card_corner_radius(card, 0, 0)
		else:
			_set_card_corner_radius(card, 24, 24)
	# header and show_more visibility are animated via modulate in tween_to()


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


func set_engagement_bar_data(_id: String) -> void:
	var engagement_bar = _get_engagement_bar()
	if engagement_bar:
		engagement_bar.update_data(_id)


func _update_separators() -> void:
	var separator_likes = _get_separator_likes()
	var separator_online = _get_separator_online()
	var container_likes = _get_container_likes()
	var container_online = _get_container_online()
	var container_location = _get_container_location()
	if container_likes and container_online:
		if separator_likes and separator_online:
			if container_likes.visible and container_online.visible:
				separator_likes.show()
				separator_online.show()
			else:
				separator_likes.hide()
				separator_online.hide()
			if container_location:
				if !container_likes.visible and !container_online.visible:
					container_location.alignment = BoxContainer.ALIGNMENT_BEGIN
					set_server_or_location(true)
				else:
					container_location.alignment = BoxContainer.ALIGNMENT_END


func _input(event: InputEvent) -> void:
	if not visible or not is_draggable:
		return
	if event is InputEventScreenTouch:
		if event.pressed:
			start_pos = event.position
			initial_pos = position
			dragging = true
		elif dragging:
			dragging = false
			var drag_distance = event.position.y - start_pos.y
			var gesture := DragGesture.IDLE
			if drag_distance > 50:
				gesture = DragGesture.DOWN
			elif drag_distance < -50:
				gesture = DragGesture.UP

			if _is_scrolling(event.position, gesture):
				return

			match gesture:
				DragGesture.UP:
					match drag_state:
						DragState.HALF:
							_on_show_more_toggled(true)
							drag_state = DragState.FULL
							tween_to(0.0, func(): return, true)
				DragGesture.DOWN:
					match drag_state:
						DragState.FULL:
							# Header must close immediately
							_get_header().hide()
							# Prevents jump glitch when Header closes
							_get_card().position.y += _get_header().size.y
							tween_to(_get_card_half_position())
							drag_state = DragState.HALF
						DragState.HALF:
							drag_state = DragState.HIDDEN
							tween_to(
								_get_card_hidden_position(), _on_texture_button_close_pressed, false
							)


## Is the user scrolling trough the description?
func _is_scrolling(pos: Vector2, gesture: DragGesture) -> bool:
	var description_rect := _get_description_scroll().get_global_rect()
	if not description_rect.has_point(pos):
		return false
	var v_scroll: float = _get_description_scroll().get_v_scroll()
	if gesture == DragGesture.DOWN:
		if v_scroll > 50.0:
			return true
	return false


func _get_offset_y_in_ancestor(node: Node, ancestor: Node) -> float:
	var y: float = 0.0
	var n: Node = node
	while n != null and n != ancestor:
		if n is Control:
			y += (n as Control).position.y
		n = n.get_parent()
	return y


func _get_card_half_position() -> float:
	var card := _get_card()
	var hide_from_here := _get_hide_from_here()
	if not card or not hide_from_here:
		return 0.0
	# Layout-independent offset: bottom of hide_from_here relative to card, so half is consistent (places/events)
	var offset_to_separator_top: float = _get_offset_y_in_ancestor(hide_from_here, card)
	var offset_to_separator_bottom: float = offset_to_separator_top + hide_from_here.size.y * 0.2
	offset_to_separator_bottom += _get_footer().size.y
	var full_height: float = get_rect().size.y
	return full_height - offset_to_separator_bottom


func _get_card_hidden_position() -> float:
	return get_rect().size.y


func _on_tween_to_finished() -> void:
	var header := _get_header()
	var show_more := _get_show_more_container()
	if header and is_draggable and not _tween_header_visible:
		header.hide()
	if show_more and is_draggable and _tween_header_visible:
		show_more.hide()
	if _tween_callback.is_valid():
		_tween_callback.call()


func tween_to(
	y_position: float, callback: Callable = Callable(), header_visible: bool = false
) -> void:
	var card := _get_card()
	if not card:
		return
	if drag_tween and drag_tween.is_running():
		drag_tween.stop()
		drag_tween = null
	_tween_callback = callback if callback.is_valid() else Callable()
	_tween_header_visible = header_visible
	var header := _get_header()
	var show_more := _get_show_more_container()
	if header and is_draggable and header_visible:
		header.show()
		header.self_modulate = Color.TRANSPARENT
		card.position.y -= _get_header().size.y
	if show_more and is_draggable and not header_visible:
		show_more.show()
		show_more.self_modulate = Color.TRANSPARENT
	drag_tween = create_tween().set_trans(Tween.TRANS_QUART)
	drag_tween.set_parallel(true)
	drag_tween.tween_property(card, "position:y", y_position, _TWEEN_DURATION)
	if header and is_draggable:
		var header_target := Color.WHITE if header_visible else Color.TRANSPARENT
		drag_tween.tween_property(header, "self_modulate", header_target, _TWEEN_DURATION)
	if show_more and is_draggable:
		var show_more_target := Color.TRANSPARENT if header_visible else Color.WHITE
		drag_tween.tween_property(show_more, "self_modulate", show_more_target, _TWEEN_DURATION)
	drag_tween.set_parallel(false)
	drag_tween.tween_callback(_on_tween_to_finished)
