class_name PresetAvatarCard
extends Button

# Distance in pixels a finger must travel before we treat it as a scroll.
const TAP_THRESHOLD = 20.0
const PROFILES_BASE_URL = "https://peer.decentraland.org/lambdas/profiles/"

@export var style_normal: StyleBox
@export var style_selected: StyleBox
@export var style_tapping: StyleBox

var preset_data: Dictionary = {}
var _touch_start = Vector2.ZERO
var _touch_active = false  # touch that started inside this card's rect
var _scroll_detected = false

@onready var _texture_rect: TextureRect = $TextureRect
@onready var _skeleton: RectangleSkeleton = $RectangleSkeleton


func _ready() -> void:
	toggle_mode = true
	# All interaction is handled manually in _input so the parent ScrollContainer
	# receives GUI events unobstructed.
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	self_modulate = Color.TRANSPARENT
	disabled = true
	_skeleton.show()
	UiSounds.install_audio(self)
	toggled.connect(_on_toggled)


func setup(image: Texture2D, data: Dictionary) -> void:
	preset_data = data
	_texture_rect.texture = image
	_skeleton.hide()
	self_modulate = Color.WHITE
	disabled = false
	_apply_style()


func _input(event: InputEvent) -> void:
	if disabled:
		return
	if event is InputEventScreenTouch:
		if event.pressed:
			_scroll_detected = false
			_touch_active = get_global_rect().has_point(event.position)
			if _touch_active:
				_touch_start = event.position
				_set_tap_style(true)
		elif _touch_active:
			_touch_active = false
			_set_tap_style(false)
			if not _scroll_detected and not button_pressed:
				# Confirmed tap: select this card and notify the ButtonGroup.
				button_pressed = true
				if button_group:
					for btn in button_group.get_buttons():
						if btn != self:
							btn.set_pressed_no_signal(false)
					button_group.pressed.emit(self)
			_scroll_detected = false
	elif event is InputEventScreenDrag:
		if _touch_active and not _scroll_detected:
			if event.position.distance_to(_touch_start) >= TAP_THRESHOLD:
				_scroll_detected = true
				_set_tap_style(false)


## Fetches the stock profile behind `preset_id` and fills this card. A method of the
## card, so the fetch dies with it when the carousel is rebuilt or closed mid-load.
func async_load_preset(preset_id: String, avatar_id: int) -> void:
	var url = PROFILES_BASE_URL + preset_id
	var headers = {"Content-Type": "application/json"}
	var promise = Global.http_requester.request_json(url, HTTPClient.METHOD_GET, "", headers)
	var result = await PromiseUtils.async_awaiter(promise)

	if result is PromiseError:
		push_error("PresetAvatarCarousel: Failed to fetch " + preset_id + ": " + result.get_error())
		return

	var json = result.get_string_response_as_json()
	if json is not Dictionary:
		push_error("PresetAvatarCarousel: Unexpected response for " + preset_id)
		return

	var avatars = json.get("avatars", [])
	if avatars.is_empty():
		return

	var avatar_data: Dictionary = avatars[0].get("avatar", {})
	var data = _extract_preset_data(avatar_data)
	data["avatar_id"] = avatar_id
	preset_data = data

	var snapshot_url = _get_body_snapshot_url(json)
	if snapshot_url.is_empty():
		return

	var url_hash = snapshot_url.md5_text()
	var img_promise = Global.content_provider.fetch_texture_by_url(url_hash, snapshot_url)
	var img_result = await PromiseUtils.async_awaiter(img_promise)

	if img_result is PromiseError:
		push_error("PresetAvatarCarousel: Failed to fetch thumbnail: " + img_result.get_error())
		return

	setup(img_result.texture, data)


func _extract_preset_data(avatar_data: Dictionary) -> Dictionary:
	return {
		"body_shape": avatar_data.get("bodyShape", ""),
		"wearables": avatar_data.get("wearables", []),
		"skin_color": avatar_data.get("skin", {}).get("color", {}),
		"hair_color": avatar_data.get("hair", {}).get("color", {}),
		"eye_color": avatar_data.get("eyes", {}).get("color", {}),
	}


func _get_body_snapshot_url(profile_data: Dictionary) -> String:
	var avatars = profile_data.get("avatars", [])
	if avatars.is_empty():
		return ""
	var avatar = avatars[0].get("avatar", {})
	var snapshots = avatar.get("snapshots", {})
	var body_url = snapshots.get("body", "")
	if body_url.is_empty():
		return ""
	if body_url.begins_with("http"):
		return body_url
	var base_url = profile_data.get("baseUrl", "https://peer.decentraland.org/content/contents/")
	return base_url + body_url


func _on_toggled(_pressed: bool) -> void:
	_apply_style()


func _apply_style() -> void:
	var style = style_selected if button_pressed else style_normal
	if style:
		add_theme_stylebox_override("normal", style)
		add_theme_stylebox_override("pressed", style)
		add_theme_stylebox_override("hover", style)
		add_theme_stylebox_override("hover_pressed", style)


func _set_tap_style(tapping: bool) -> void:
	if tapping:
		if style_tapping:
			add_theme_stylebox_override("normal", style_tapping)
			add_theme_stylebox_override("pressed", style_tapping)
			add_theme_stylebox_override("hover", style_tapping)
			add_theme_stylebox_override("hover_pressed", style_tapping)
	else:
		_apply_style()
