class_name MarketplaceRecommendedSection
extends VBoxContainer

signal item_equip(urn: String)
signal item_unequip(urn: String)

const CATALOG_API_URL = "https://marketplace-api.decentraland.org/v2/catalog"
const WEARABLE_ITEM_SCENE = preload(
	"res://src/ui/components/molecules/wearable_item/wearable_item.tscn"
)

## Maps backpack subcategory names to the v2/catalog query parameter.
const CATEGORY_FILTER_MAP: Dictionary = {
	"head": "isWearableHead",
	"upper_body": "isWearableUpperBody",
	"lower_body": "isWearableLowerBody",
	"feet": "isWearableFeet",
	"hair": "isWearableHair",
	"facial_hair": "isWearableFacialHair",
	"eyes": "isWearableEyes",
	"eyebrows": "isWearableEyebrows",
	"mouth": "isWearableMouth",
	"hat": "isWearableHat",
	"helmet": "isWearableHelmet",
	"tiara": "isWearableTiara",
	"top_head": "isWearableTopHead",
	"eyewear": "isWearableEyewear",
	"mask": "isWearableMask",
	"earring": "isWearableEarring",
	"skin": "isWearableSkin",
	"handwear": "isWearableHandwear",
}

@export var credits_balance: int = 0
@export var asset_type: String = "wearables"

var _current_category: String = ""
var _card_button_group: ButtonGroup

@onready var _grid: GridContainer = %GridContainer_Recommended
@onready var _button_cta: MarketplaceCtaCard = %Button_CTA


func _ready():
	_button_cta.has_credits = credits_balance > 0
	_button_cta.marketplace_section = asset_type
	_card_button_group = ButtonGroup.new()
	_card_button_group.allow_unpress = true
	_add_placeholder_cards()


func set_columns(columns: int):
	if _grid:
		_grid.columns = columns


func update_category(category: String):
	if category == _current_category:
		return
	_current_category = category
	_async_fetch_items(category)


func _build_catalog_url(category: String) -> String:
	var skip = randi_range(0, 10)
	var url = (
		CATALOG_API_URL
		+ (
			"?first=3&skip=%d&category=wearable&isOnSale=true&minPrice=1&onlyMinting=true&sortBy=recently_listed"
			% skip
		)
	)
	var filter_key = CATEGORY_FILTER_MAP.get(category, "")
	if not filter_key.is_empty():
		url += "&%s=true" % filter_key
	return url


func _add_placeholder_cards():
	for i in range(3):
		var card = WEARABLE_ITEM_SCENE.instantiate()
		_grid.add_child(card)


func _async_fetch_items(category: String):
	var url = _build_catalog_url(category)
	var promise = Global.http_requester.request_json(url, HTTPClient.METHOD_GET, "", {})
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		printerr("[MarketplaceRecommended] Error fetching items: ", result.get_error())
		return
	var json = result.get_string_response_as_json()
	var items = json.get("data", [])
	_populate_cards(items)


func _populate_cards(items: Array):
	for child in _grid.get_children():
		child.queue_free()
	for item_data in items:
		var card = WEARABLE_ITEM_SCENE.instantiate()
		_grid.add_child(card)
		_setup_card(card, item_data)


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

	var thumbnail_url = item_data.get("thumbnail", "")
	if not thumbnail_url.is_empty():
		_async_load_thumbnail(card, thumbnail_url)


func _on_card_equip(urn: String):
	item_equip.emit(urn)


func _on_card_unequip(urn: String):
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
