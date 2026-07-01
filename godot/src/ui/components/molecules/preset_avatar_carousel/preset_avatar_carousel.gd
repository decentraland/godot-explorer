class_name PresetAvatarCarousel
extends Control

signal preset_selected(preset_data: Dictionary)

const PRESET_CARD_SCENE = preload(
	"res://src/ui/components/atoms/controls/preset_avatar_card/preset_avatar_card.tscn"
)
const PRESET_COUNT = 12
const PROFILES_BASE_URL = "https://peer.decentraland.org/lambdas/profiles/"

var _button_group: ButtonGroup
var _loaded = false

@onready var h_box_container_cards: HBoxContainer = %HBoxContainerCards
@onready var scroll_container: Container = %ScrollContainer


func _ready() -> void:
	_button_group = ButtonGroup.new()
	_button_group.allow_unpress = false
	_button_group.pressed.connect(_on_button_group_pressed)
	visibility_changed.connect(_on_visibility_changed)
	if is_visible_in_tree():
		_async_load_presets()


func _on_visibility_changed() -> void:
	if is_visible_in_tree() and not _loaded:
		_async_load_presets()


func _async_load_presets() -> void:
	_loaded = true
	var cards: Array[PresetAvatarCard] = []
	for i in range(1, PRESET_COUNT + 1):
		var card: PresetAvatarCard = PRESET_CARD_SCENE.instantiate()
		card.button_group = _button_group
		h_box_container_cards.add_child(card)
		if i == 1:
			card.button_pressed = true
		cards.append(card)
		_async_load_preset_into_card("default%d" % i, card)


func _async_load_preset_into_card(preset_id: String, card: PresetAvatarCard) -> void:
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
	var preset_data = _extract_preset_data(avatar_data)
	card.preset_data = preset_data

	var snapshot_url = _get_body_snapshot_url(json)
	if snapshot_url.is_empty():
		return

	var url_hash = snapshot_url.md5_text()
	var img_promise = Global.content_provider.fetch_texture_by_url(url_hash, snapshot_url)
	var img_result = await PromiseUtils.async_awaiter(img_promise)

	if img_result is PromiseError:
		push_error("PresetAvatarCarousel: Failed to fetch thumbnail: " + img_result.get_error())
		return

	card.setup(img_result.texture, preset_data)


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


func _on_button_group_pressed(button: BaseButton) -> void:
	var card = button as PresetAvatarCard
	if card:
		preset_selected.emit(card.preset_data)
