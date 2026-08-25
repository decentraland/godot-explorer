extends Control

signal ftue_completed
signal jump_in(parcel_position: Vector2i, realm_str: String)
signal jump_in_world(realm_str: String)

const DEFAULT_SUBTITLE := "Let's get you started"
const FeaturedDataProvider = preload(
	"res://src/ui/components/molecules/snap_carousel/featured_data_provider.gd"
)

var _places: Array[Dictionary] = []

# Campaign resolution for this launch (issue #2670): {token, campaign, fallback_reason}.
# Empty campaign means the default FTUE, which is every path that did not resolve.
var _campaign_resolution: Dictionary = {}

@onready var carousel: Control = %SnapCarousel
@onready var label_welcome: RichTextLabel = %Label_Welcome
@onready var button_jump_in: Button = %Button_JumpIn_FTUE
@onready var button_skip: Button = %Button_Skip


func _ready() -> void:
	carousel.card_tapped.connect(_on_card_tapped)
	carousel.items_loaded.connect(_on_items_loaded)
	button_jump_in.pressed.connect(_on_button_jump_in_pressed)
	button_skip.pressed.connect(_on_button_skip_pressed)


func set_username(display_name: String) -> void:
	label_welcome.text = (
		"Welcome [color=#B18AFF]@" + display_name + "[/color]\n" + _get_subtitle()
	)


## Applies the campaign resolved for this launch. Must run before set_username /
## load_places; an empty resolution (the default) leaves the screen exactly as it was.
func set_campaign(resolution: Dictionary) -> void:
	_campaign_resolution = resolution
	var cta := String(_get_campaign().get("cta", ""))
	if not cta.is_empty() and is_instance_valid(button_jump_in):
		button_jump_in.text = cta


func load_places() -> void:
	var place_ids = _get_campaign().get("placeIds", [])
	if typeof(place_ids) == TYPE_ARRAY and not place_ids.is_empty():
		_async_load_campaign_places(place_ids)
		return
	carousel.fetch()


func _get_campaign() -> Dictionary:
	var campaign = _campaign_resolution.get("campaign", {})
	return campaign if typeof(campaign) == TYPE_DICTIONARY else {}


func _get_subtitle() -> String:
	var title := String(_get_campaign().get("title", ""))
	return title if not title.is_empty() else DEFAULT_SUBTITLE


# gdlint:ignore = async-function-name
func _async_load_campaign_places(place_ids: Array) -> void:
	var places := await FeaturedDataProvider.async_fetch_places_by_ids(place_ids)
	if not is_instance_valid(self):
		return
	if places.is_empty():
		# The curated set resolved to nothing — the campaign copy still applies, but the
		# carousel falls back to the default featured list rather than showing an empty rail.
		carousel.fetch()
		return
	carousel.set_items(places)


func _on_items_loaded(places: Array[Dictionary]) -> void:
	_places.assign(places)
	_track_screen_view()


func _on_card_tapped(_index: int) -> void:
	_on_button_jump_in_pressed()


func _on_button_jump_in_pressed() -> void:
	if _places.is_empty():
		# Nothing in the carousel. A campaign launch still has somewhere to go — the scene
		# the ad sold — so the button works instead of doing nothing.
		_jump_to_campaign_target()
		return
	var index = carousel.get_current_index()
	var place: Dictionary = _places[index]
	var payload := {"place_id": place.get("id", ""), "position": index}
	payload.merge(CampaignResolution.metrics_context(_campaign_resolution))
	Global.metrics.track_click_button("JUMP_IN", "DISCOVER_FTUE", JSON.stringify(payload))
	ftue_completed.emit()
	_do_jump_in(place)


func _on_button_skip_pressed() -> void:
	Global.metrics.track_click_button(
		"SKIP",
		"DISCOVER_FTUE",
		JSON.stringify(CampaignResolution.metrics_context(_campaign_resolution))
	)
	ftue_completed.emit()


func _jump_to_campaign_target() -> void:
	var campaign := _get_campaign()
	if campaign.is_empty():
		return
	var position_and_realm := CampaignResolution.target_position_and_realm(campaign)
	if position_and_realm.is_empty():
		return

	var payload := {"place_id": "", "position": -1}
	payload.merge(CampaignResolution.metrics_context(_campaign_resolution))
	Global.metrics.track_click_button("JUMP_IN", "DISCOVER_FTUE", JSON.stringify(payload))
	ftue_completed.emit()

	if CampaignResolution.is_world_target(campaign):
		jump_in_world.emit(position_and_realm[1])
	else:
		jump_in.emit(position_and_realm[0], position_and_realm[1])


func _do_jump_in(place_data: Dictionary) -> void:
	if PlacesHelper.is_world(place_data):
		var pos_realm := PlacesHelper.get_position_and_realm(place_data)
		jump_in_world.emit(pos_realm[1])
		return
	var pos_realm := PlacesHelper.get_position_and_realm(place_data)
	jump_in.emit(pos_realm[0], pos_realm[1])


func _track_screen_view() -> void:
	var carousel_items = []
	for i in _places.size():
		carousel_items.append({"position": i, "place_id": _places[i].get("id", "")})
	var payload := {"carousel": carousel_items}
	payload.merge(CampaignResolution.metrics_context(_campaign_resolution))
	Global.metrics.track_screen_viewed("DISCOVER_FTUE", JSON.stringify(payload))
