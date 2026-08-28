class_name MarketplaceRecommendedSection
extends VBoxContainer

signal item_equip(urn: String)
signal item_unequip(urn: String)
signal item_selected(urn: String, item_name: String)

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

## Hide items priced above the largest credits pack currently OFFERED (`credits_tier_b3`
## = 260 credits): they can't be afforded with a single In-App Purchase, so surfacing
## them is misleading (#2298). Keep this in sync with the top card in
## `credits_option.tscn` — swapping the storefront to another price point changes it.
## Sent as `maxPriceCredits`, in the same whole credits the IAP balance is counted in.
const _MAX_PRICE_CREDITS := "260"

## Cards in one row of suggestions.
const _CARDS_PER_ROW := 3

## Entries pulled per row. The catalog can't filter by the body shape the player
## wears (see `_matches_player_gender`) and doesn't always carry enough to build a
## urn, so ask for spares and drop what doesn't fit instead of showing a short row.
const _FETCH_COUNT := _CARDS_PER_ROW * 3

## Chain segment of a collections-v2 urn, keyed by the `chainId` the catalog reports.
## Only the Polygon-family chains are listed on purpose: they are the ones that mint
## collections-v2 items, whose urn IS `<contract>:<itemId>`. Ethereum's collections-v1
## urns are named (`collections-v1:<collection>:<item>`) and can't be rebuilt from the
## ids, so an entry on another chain is skipped rather than given a plausible-looking
## urn that resolves to nothing.
const _URN_CHAIN_BY_ID: Dictionary = {137: "matic", 80002: "amoy"}

@export var asset_type: String = "wearables"

var _current_category: String = ""
var _card_button_group: ButtonGroup
var _card_prices: Dictionary = {}  # urn -> price
var _card_names: Dictionary = {}  # urn -> display name

@onready var _grid: GridContainer = %GridContainer_Recommended
@onready var _button_cta: MarketplaceCtaCard = %Button_CTA


func _ready():
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
	# Prices are bounded in CREDITS (`minPriceCredits`/`maxPriceCredits`). The v2
	# endpoint's `minPrice`/`maxPrice` are MANA and the unified endpoint ignores
	# them outright, so sending those would silently drop the ceiling and offer
	# items no single credits pack can pay for.
	#
	# `listingType=primary` is what `onlyMinting=true` used to say: without it the
	# feed also carries resales, which are listings of ONE token. A card links to
	# the item page, so a resale would quote the seller's price next to a page that
	# doesn't sell at it. Zone already returns a few.
	if asset_type == "emotes":
		return (
			DclUrls.marketplace_catalog_api()
			+ (
				"?first=%d&skip=%d&category=emote&isOnSale=true&listingType=primary&minPriceCredits=1&maxPriceCredits=%s&sortBy=recently_listed"
				% [first, skip, _MAX_PRICE_CREDITS]
			)
		)
	var url = (
		DclUrls.marketplace_catalog_api()
		+ (
			"?first=%d&skip=%d&category=wearable&isOnSale=true&listingType=primary&minPriceCredits=1&maxPriceCredits=%s&sortBy=recently_listed"
			% [first, skip, _MAX_PRICE_CREDITS]
		)
	)
	var wearable_cat = category if category in WEARABLE_CATEGORIES else ""
	if category == "handwear":
		wearable_cat = "hands"
	if not wearable_cat.is_empty():
		url += "&wearableCategory=%s" % wearable_cat
	# No `wearableGender`: the unified handler never reads that param (it isn't in the
	# filter set marketplace-server builds for this feed), so sending it looks like a
	# body-shape filter and is one more unfiltered page. The check moved onto the
	# response instead — `_matches_player_gender`.
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
	for i in range(_CARDS_PER_ROW):
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
	var max_skip = maxi(total - _FETCH_COUNT, 0)
	var skip = randi_range(0, max_skip)
	var url = _build_catalog_url(category, skip, _FETCH_COUNT)
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
	# `first=0` asks for the count without the page. The unified endpoint doesn't reject
	# it and reports the same `total` the paged call does — verified on org and zone, for
	# both wearables and emotes — it just clamps the page to one row, which is thrown
	# away here. `total` is what gates the whole section, so this contract is a QA case.
	var url = _build_catalog_url(category, 0, 0)
	var promise = Global.http_requester.request_json(url, HTTPClient.METHOD_GET, "", {})
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		return 0
	var json = result.get_string_response_as_json()
	return _int_field(json, "total")


func _populate_cards(items: Array):
	for child in _grid.get_children():
		_grid.remove_child(child)
		child.queue_free()
	_card_prices.clear()
	_card_names.clear()
	_button_cta.update_selection(-1)
	for item_data in items:
		if _grid.get_child_count() >= _CARDS_PER_ROW:
			break
		if not _matches_player_gender(item_data):
			continue
		var urn := _item_urn(item_data)
		if urn.is_empty():
			continue
		var card = WEARABLE_ITEM_SCENE.instantiate()
		_grid.add_child(card)
		_setup_card(card, item_data, urn)
	_update_visible_cards()


func _setup_card(card: WearableItem, item_data: Dictionary, urn: String):
	_set_rarity_background(card, item_data.get("rarity", "common"))

	# `priceCredits` is the price in the credits an In-App Purchase buys — the same
	# unit as the balance the card compares it against to choose between DETAIL and
	# GET CREDITS. The entry also carries `manaWei`, a different currency: 1 MANA is
	# worth ~0.63 credits, so its figure reads about 1.6x too expensive.
	var price := _int_field(item_data, "priceCredits")
	# The link is built from the entry's ids for the same reason the urn above is:
	# the catalog carries neither. DclUrls picks the route table the current env
	# serves — the classic marketplace's "/contracts/{c}/items/{i}" is a 404 under
	# the new shop. Both ids are known to be present, having produced the urn.
	var full_url := str(
		DclUrls.marketplace_item(
			_string_field(item_data, "contractAddress"), _string_field(item_data, "itemId")
		)
	)
	card.setup_marketplace(price, full_url)

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


## The item urn (`urn:decentraland:<chain>:collections-v2:<contract>:<itemId>`), which
## the catalog never spells out — it identifies an entry by contract plus item id.
## Everything downstream of a card (equipping the preview, the selection signals, the
## price lookup) keys off the urn, so an entry that can't produce one is skipped.
func _item_urn(item_data: Dictionary) -> String:
	var chain := str(_URN_CHAIN_BY_ID.get(_int_field(item_data, "chainId"), ""))
	var contract_address := _string_field(item_data, "contractAddress")
	var item_id := _string_field(item_data, "itemId")
	if chain.is_empty() or contract_address.is_empty() or item_id.is_empty():
		return ""
	return "urn:decentraland:%s:collections-v2:%s:%s" % [chain, contract_address, item_id]


## Whether the entry has a representation for the body shape the player wears.
## Equipping a wearable that only ships the other one leaves that slot naked, so a
## mismatch is dropped here rather than asked of the server: `gender` is a derived
## column on this feed (unisex when the item declares both base shapes) and the
## unified query only ever SELECTs it — v2 was the one that could filter on it.
func _matches_player_gender(item_data: Dictionary) -> bool:
	var player_gender := MarketplaceUrl.current_player_gender()
	if player_gender.is_empty():
		return true
	var gender := _string_field(item_data, "gender")
	# Null for emotes, which have no body shape to mismatch — those are kept.
	return gender.is_empty() or gender == "unisex" or gender == player_gender


## Reads a string field, treating a JSON null as absent. `str(null)` is "<null>",
## a non-empty string that would pass an is_empty() guard and reach a URL.
func _string_field(item_data: Dictionary, key: String) -> String:
	var value = item_data.get(key)
	return str(value) if value != null else ""


## Same, for the numeric fields. `Dictionary.get(key, default)` only falls back when the
## key is MISSING, and this feed sends it present-and-null instead — `manaWei`, `tokenId`,
## `seller` and `issuedId` all arrive that way. `int(null)` doesn't quietly give 0; it
## pushes a Godot error (→ Sentry) on the way there.
func _int_field(item_data: Dictionary, key: String) -> int:
	var value = item_data.get(key)
	return int(value) if value != null else 0


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
