class_name NearbySceneNotifier
extends Node

var _scene_fetcher: SceneFetcher = null

var _home_scene_entity_id: String = ""
var _home_scene_parcels: Array[Vector2i] = []
var _pending_toast_parcel: Vector2i = SceneFetcher.INVALID_PARCEL
var _pending_toast_entity_id: String = ""
var _pending_toast_scene_parcels: Array[Vector2i] = []
var _toast_shown_scene_parcels: Array[Vector2i] = []
var _scene_toast_timer: Timer = null
var _scene_modal_timer: Timer = null
var _scene_toast_cache: Dictionary = {}


func setup(scene_fetcher: SceneFetcher) -> void:
	_scene_fetcher = scene_fetcher


func _ready() -> void:
	_scene_toast_timer = Timer.new()
	_scene_toast_timer.one_shot = true
	_scene_toast_timer.wait_time = 3.0
	_scene_toast_timer.timeout.connect(_async_on_scene_toast_timer_timeout)
	add_child(_scene_toast_timer)

	_scene_modal_timer = Timer.new()
	_scene_modal_timer.one_shot = true
	_scene_modal_timer.wait_time = 8.0
	_scene_modal_timer.timeout.connect(_async_on_scene_modal_timer_timeout)
	add_child(_scene_modal_timer)

	Global.notification_clicked.connect(_on_notification_clicked)


func on_position_changed(new_position: Vector2i) -> void:
	_async_check_nearby_scene(new_position)


func on_teleport() -> void:
	_home_scene_entity_id = ""
	_home_scene_parcels = []
	_pending_toast_parcel = SceneFetcher.INVALID_PARCEL
	_pending_toast_entity_id = ""
	_pending_toast_scene_parcels = []
	_toast_shown_scene_parcels = []
	_scene_toast_timer.stop()
	_scene_modal_timer.stop()


func on_realm_changed() -> void:
	_scene_toast_cache = {}


func _on_notification_clicked(notification_d: Dictionary) -> void:
	if notification_d.get("type", "") != "nearby_scene":
		return
	var pos: Vector2i = notification_d.get("parcel_position", Vector2i.ZERO)
	if Global.modal_manager != null:
		Global.modal_manager.async_show_teleport_modal(pos)


func _async_on_scene_toast_timer_timeout() -> void:
	_pending_toast_parcel = SceneFetcher.INVALID_PARCEL
	if _home_scene_entity_id.is_empty() or _scene_fetcher.current_position in _home_scene_parcels:
		return
	await _async_show_scene_toast(_scene_fetcher.current_position)


func _async_on_scene_modal_timer_timeout() -> void:
	if _home_scene_entity_id.is_empty() or _scene_fetcher.current_position in _home_scene_parcels:
		return
	if Global.modal_manager != null:
		Global.modal_manager.async_show_teleport_modal(_scene_fetcher.current_position)


func _async_check_nearby_scene(new_position: Vector2i) -> void:
	var loaded_scene_id: int = Global.scene_runner.get_current_parcel_scene_id()
	var loaded_entity_id: String = Global.scene_runner.get_scene_entity_id(loaded_scene_id)

	# Track home as the first loaded scene after spawn/teleport and cache its parcel list
	if _home_scene_entity_id.is_empty() and not loaded_entity_id.is_empty():
		_home_scene_entity_id = loaded_entity_id
		var home_data: SceneFetcher.SceneItem = _scene_fetcher.loaded_scenes.get(
			_home_scene_entity_id
		)
		if home_data != null:
			_home_scene_parcels = home_data.parcels.duplicate()

	# Player stepped back into home — cancel any pending toast/modal
	if new_position in _home_scene_parcels:
		_pending_toast_parcel = SceneFetcher.INVALID_PARCEL
		_pending_toast_entity_id = ""
		_pending_toast_scene_parcels = []
		_toast_shown_scene_parcels = []
		_scene_toast_timer.stop()
		_scene_modal_timer.stop()
		return

	# Home not established yet — can't determine context
	if _home_scene_entity_id.is_empty():
		return

	# Coordinator maps parcels to entity IDs for new_position (correct source for new parcel)
	var new_entity_id := _scene_fetcher.scene_entity_coordinator.get_scene_entity_id(new_position)

	# Timer is running — check if we're still in the same scene
	if _scene_toast_timer.time_left > 0:
		# Lazily resolve pending entity/parcels in case coordinator/scene loaded since timer started
		if _pending_toast_entity_id.is_empty():
			var resolved := _scene_fetcher.scene_entity_coordinator.get_scene_entity_id(
				_pending_toast_parcel
			)
			if not resolved.is_empty():
				_pending_toast_entity_id = resolved
		if _pending_toast_scene_parcels.is_empty() and not _pending_toast_entity_id.is_empty():
			var scene_data: SceneFetcher.SceneItem = _scene_fetcher.loaded_scenes.get(
				_pending_toast_entity_id
			)
			if scene_data != null:
				_pending_toast_scene_parcels = scene_data.parcels.duplicate()

		if not new_entity_id.is_empty() and new_entity_id == _pending_toast_entity_id:
			return  # same known scene
		if new_position in _pending_toast_scene_parcels:
			return  # same scene by parcel list
		# Either entity is unknown — can't confirm scene change, keep timer running
		if new_entity_id.is_empty() or _pending_toast_entity_id.is_empty():
			return
		# Both known and different — fall through to restart

	# Still within the parcels of the last shown scene — no need to re-notify
	if new_position in _toast_shown_scene_parcels:
		return

	# New scene — clear state and (re)start the timer
	_toast_shown_scene_parcels = []
	_pending_toast_scene_parcels = []
	_pending_toast_parcel = new_position
	_pending_toast_entity_id = new_entity_id
	if not new_entity_id.is_empty():
		var scene_data: SceneFetcher.SceneItem = _scene_fetcher.loaded_scenes.get(new_entity_id)
		if scene_data != null:
			_pending_toast_scene_parcels = scene_data.parcels.duplicate()
	_scene_toast_timer.start()
	_scene_modal_timer.start()


func _async_show_scene_toast(parcel: Vector2i) -> void:
	var entity_id := _scene_fetcher.scene_entity_coordinator.get_scene_entity_id(parcel)

	var place: Dictionary = {}
	if not entity_id.is_empty() and _scene_toast_cache.has(entity_id):
		place = _scene_toast_cache[entity_id]
	else:
		var result = await PlacesHelper.async_get_by_position(parcel)
		if result is PromiseError:
			return
		var json: Dictionary = result.get_string_response_as_json()
		if json.get("data", []).is_empty():
			return
		place = json["data"][0]
		if not entity_id.is_empty():
			_scene_toast_cache[entity_id] = place

	# Store parcels of shown scene so uncached steps within it don't re-trigger
	_toast_shown_scene_parcels = []
	for pos_str in place.get("positions", []):
		var parts: PackedStringArray = str(pos_str).split(",")
		if parts.size() == 2:
			_toast_shown_scene_parcels.append(Vector2i(int(parts[0]), int(parts[1])))

	var title: String = "You're entering " + place.get("title", "Unknown place")
	var creator: String = place.get("contact_name", "")
	var description: String = "By " + creator if not creator.is_empty() else "Tap to explore"

	NotificationsManager.show_system_toast(
		title, description, "nearby_scene", "default", {"parcel_position": parcel}
	)
