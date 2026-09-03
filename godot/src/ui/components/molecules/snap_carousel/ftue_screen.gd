extends Control

signal ftue_completed
signal jump_in(parcel_position: Vector2i, realm_str: String)
signal jump_in_world(realm_str: String)

## Font sizes tried, largest first, until the greeting's longest line fits on one line.
## 48 is the scene's design size, so English with a short name renders exactly as before.
## FTUE_WELCOME is deliberately two lines ("Welcome @name" / "Let's get you started") and the
## name is user data of unbounded length, so ES/PT push the first line to wrap (#2825).
const WELCOME_FONT_SIZES: PackedInt32Array = [48, 42, 36]

## Strips bbcode tags so the measured width is the drawn text, not the markup.
const BBCODE_TAG_RE := "\\[[^\\]]*\\]"

var _places: Array[Dictionary] = []

## Kept so the greeting can be recomposed on a language change — text assigned from GDScript
## does not re-translate itself.
var _display_name: String = ""
var _bbcode_regex: RegEx = null

# How the campaign token resolved (#2670). Metrics only: a resolved campaign never reaches
# this screen, so this just names which failure sent the launch here.
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
	# The fit depends on the width the label is finally given, which is not known in _ready.
	label_welcome.resized.connect(_fit_welcome_text)


func _notification(what: int) -> void:
	if what == NOTIFICATION_TRANSLATION_CHANGED and not _display_name.is_empty():
		_update_welcome()


func set_username(display_name: String) -> void:
	_display_name = display_name
	_update_welcome()


func _update_welcome() -> void:
	label_welcome.text = tr("FTUE_WELCOME").format({"name": _display_name})
	_fit_welcome_text()


## Steps the greeting's font down until its longest line fits the available width, so a long
## display name or a longer translation does not wrap the name onto a second line.
func _fit_welcome_text() -> void:
	var available: float = label_welcome.size.x
	if available <= 0.0:
		return

	var font: Font = label_welcome.get_theme_font("normal_font")
	if font == null:
		return

	if _bbcode_regex == null:
		_bbcode_regex = RegEx.new()
		_bbcode_regex.compile(BBCODE_TAG_RE)
	var plain: String = _bbcode_regex.sub(label_welcome.text, "", true)

	var longest: String = ""
	for line in plain.split("\n"):
		if font.get_string_size(line).x > font.get_string_size(longest).x:
			longest = line
	if longest.is_empty():
		return

	var chosen: int = WELCOME_FONT_SIZES[WELCOME_FONT_SIZES.size() - 1]
	for size in WELCOME_FONT_SIZES:
		if font.get_string_size(longest, HORIZONTAL_ALIGNMENT_LEFT, -1, size).x <= available:
			chosen = size
			break

	# Guard against re-entry: this runs from `resized`, and changing the font size resizes.
	if label_welcome.get_theme_font_size("normal_font_size") != chosen:
		label_welcome.add_theme_font_size_override("normal_font_size", chosen)
		label_welcome.add_theme_font_size_override("bold_font_size", chosen)


## Records how the campaign token resolved, for the metrics this screen already emits.
func set_campaign_context(resolution: Dictionary) -> void:
	_campaign_resolution = resolution


func load_places() -> void:
	carousel.fetch()


func _on_items_loaded(places: Array[Dictionary]) -> void:
	_places.assign(places)
	_track_screen_view()


func _on_card_tapped(_index: int) -> void:
	_on_button_jump_in_pressed()


func _on_button_jump_in_pressed() -> void:
	if _places.is_empty():
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
