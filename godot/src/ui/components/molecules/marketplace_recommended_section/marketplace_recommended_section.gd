class_name MarketplaceRecommendedSection
extends VBoxContainer

signal item_equip(urn: String)
signal item_unequip(urn: String)
signal item_selected(urn: String, item_name: String)

const CATALOG_API_URL = "https://marketplace-api.decentraland.org/v2/catalog"
const WEARABLE_ITEM_SCENE = preload(
	"res://src/ui/components/molecules/wearable_item/wearable_item.tscn"
)

## Backpack subcategories that map directly to wearableCategory param values.
const WEARABLE_CATEGORIES: Array = [
	"facial_hair",
	"hair",
	"eyes",
	"eyebrows",
	"mouth",
	"upper_body",
	"hands",
	"lower_body",
	"feet",
	"earring",
	"eyewear",
	"hat",
	"helmet",
	"mask",
	"tiara",
	"top_head",
	"skin",
]

## Categories that have no marketplace suggestions.
const HIDDEN_CATEGORIES: Array = ["body_shape", "all", "all_extras"]

@export var credits_balance: int = 0:
	set(value):
		credits_balance = value
		if _button_cta:
			_button_cta.credits_balance = value
@export var asset_type: String = "wearables"

var _current_category: String = ""
var _card_button_group: ButtonGroup
var _card_prices: Dictionary = {}  # urn -> price
var _card_names: Dictionary = {}  # urn -> display name

@onready var _grid: GridContainer = %GridContainer_Recommended
@onready var _button_cta: MarketplaceCtaCard = %Button_CTA


func _ready():
	_button_cta.credits_balance = credits_balance
	_button_cta.marketplace_section = asset_type
	_card_button_group = ButtonGroup.new()
	_card_button_group.allow_unpress = true
	# Start hidden — update_category will show when there are results
	visible = false


func set_columns(columns: int):
	if _grid:
		_grid.columns = columns
		_update_visible_cards()


func refresh():
	if not _current_category.is_empty():
		_load_category(_current_category)


func clear_selection():
	if _card_button_group:
		var pressed = _card_button_group.get_pressed_button()
		if pressed:
			pressed.set_pressed(false)
	_button_cta.update_selection(-1)


func update_category(category: String):
	if category == _current_category:
		return
	_current_category = category
	_load_category(category)


func _load_category(category: String):
	if category in HIDDEN_CATEGORIES:
		visible = false
		return

	# Show skeleton placeholders immediately while fetching
	_reset_to_placeholders()
	visible = true
	_async_fetch_items(category)


func _build_catalog_url(category: String, skip: int = 0, first: int = 3) -> String:
	if asset_type == "emotes":
		return (
			CATALOG_API_URL
			+ (
				"?first=%d&skip=%d&category=emote&isOnSale=true&minPrice=1&onlyMinting=true&sortBy=recently_listed"
				% [first, skip]
			)
		)
	var url = (
		CATALOG_API_URL
		+ (
			"?first=%d&skip=%d&category=wearable&isOnSale=true&minPrice=1&onlyMinting=true&sortBy=recently_listed"
			% [first, skip]
		)
	)
	var wearable_cat = category if category in WEARABLE_CATEGORIES else ""
	if category == "handwear":
		wearable_cat = "hands"
	if not wearable_cat.is_empty():
		url += "&wearableCategory=%s" % wearable_cat
	return url


## Shows only as many cards as there are columns (one row).
func _update_visible_cards():
	if not _grid:
		return
	var cols = _grid.columns
	var children = _grid.get_children()
	for i in range(children.size()):
		children[i].visible = i < cols


func _add_placeholder_cards():
	for i in range(3):
		var card = WEARABLE_ITEM_SCENE.instantiate()
		_grid.add_child(card)


func _reset_to_placeholders():
	for child in _grid.get_children():
		_grid.remove_child(child)
		child.queue_free()
	_add_placeholder_cards()
	_update_visible_cards()


func _async_fetch_items(category: String):
	var total = await _async_fetch_total(category)
	if category != _current_category:
		return
	if total <= 0:
		visible = false
		return
	visible = true
	var max_skip = maxi(total - 3, 0)
	var skip = randi_range(0, max_skip)
	var url = _build_catalog_url(category, skip)
	var promise = Global.http_requester.request_json(url, HTTPClient.METHOD_GET, "", {})
	var result = await PromiseUtils.async_awaiter(promise)
	if category != _current_category:
		return
	if result is PromiseError:
		printerr("[MarketplaceRecommended] Error fetching items: ", result.get_error())
		return
	var json = result.get_string_response_as_json()
	var items = json.get("data", [])
	_populate_cards(items)


func _async_fetch_total(category: String) -> int:
	var url = _build_catalog_url(category, 0, 0)
	var promise = Global.http_requester.request_json(url, HTTPClient.METHOD_GET, "", {})
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		return 0
	var json = result.get_string_response_as_json()
	return json.get("total", 0)


func _populate_cards(items: Array):
	for child in _grid.get_children():
		_grid.remove_child(child)
		child.queue_free()
	_card_prices.clear()
	_card_names.clear()
	_button_cta.update_selection(-1)
	for item_data in items:
		var card = WEARABLE_ITEM_SCENE.instantiate()
		_grid.add_child(card)
		_setup_card(card, item_data)
	_update_visible_cards()


func _setup_card(card: WearableItem, item_data: Dictionary):
	var urn = item_data.get("urn", "")

	_set_rarity_background(card, item_data.get("rarity", "common"))

	var price = _parse_price(item_data.get("minPrice", item_data.get("price", "0")))
	var item_url = item_data.get("url", "")
	var full_url = str(DclUrls.marketplace()) + item_url if not item_url.is_empty() else ""
	card.setup_marketplace(price, full_url, credits_balance)

	card.wearable_id = urn
	card.button_group = _card_button_group
	card.equip.connect(_on_card_equip.bind(urn))
	card.unequip.connect(_on_card_unequip.bind(urn))
	_card_prices[urn] = price
	_card_names[urn] = item_data.get("name", "")

	var thumbnail_url = item_data.get("thumbnail", "")
	if not thumbnail_url.is_empty():
		_async_load_thumbnail(card, thumbnail_url)


func _on_card_equip(urn: String):
	_button_cta.update_selection(_card_prices.get(urn, 0))
	item_selected.emit(urn, _card_names.get(urn, ""))
	item_equip.emit(urn)


func _on_card_unequip(urn: String):
	_button_cta.update_selection(-1)
	item_unequip.emit(urn)


func _set_rarity_background(card: WearableItem, rarity: String):
	match rarity:
		"common":
			card.texture_rect_background.texture = card.common_thumbnail
		"uncommon":
			card.texture_rect_background.texture = card.uncommon_thumbnail
		"rare":
			card.texture_rect_background.texture = card.rare_thumbnail
		"epic":
			card.texture_rect_background.texture = card.epic_thumbnail
		"legendary":
			card.texture_rect_background.texture = card.legendary_thumbnail
		"exotic":
			card.texture_rect_background.texture = card.exotic_thumbnail
		"mythic":
			card.texture_rect_background.texture = card.mythic_thumbnail
		"unique":
			card.texture_rect_background.texture = card.unique_thumbnail
		_:
			card.texture_rect_background.texture = card.base_thumbnail
	card.texture_rect_background.show()


## Converts wei price string to integer credits (1 MANA = 10^18 wei).
func _parse_price(price_str: String) -> int:
	if price_str.is_empty() or price_str == "0":
		return 0
	if price_str.length() <= 18:
		return 1
	var mana_part = price_str.substr(0, price_str.length() - 18)
	return mana_part.to_int()


func _async_load_thumbnail(card: WearableItem, url: String):
	var http = HTTPRequest.new()
	add_child(http)
	http.request(url)
	var result = await http.request_completed
	http.queue_free()

	var response_code = result[1]
	var body = result[3]
	if response_code != 200:
		return

	var image = Image.new()
	var error = image.load_png_from_buffer(body)
	if error != OK:
		error = image.load_jpg_from_buffer(body)
	if error != OK:
		error = image.load_webp_from_buffer(body)
	if error != OK:
		return

	if not is_instance_valid(card):
		return

	var texture = ImageTexture.create_from_image(image)
	card.texture_rect_preview.texture = texture
	card.texture_progress_bar_loading.hide()
