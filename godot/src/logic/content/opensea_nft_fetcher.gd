# Fetcher NFTs for OpenSea...
# It will batch all the request in a period of time, and then it will request those
class_name OpenSeaFetcher

# DCL OpenSea
const RETRIEVE_ASSETS_ENDPOINT := "https://opensea.decentraland.org/api/v1/asset"

# OpenSea
#const RETRIEVE_ASSETS_ENDPOINT := "https://api.opensea.io/api/v1/asset"
const API_KEY := ""


class Asset:
	var valid: bool = false
	var endpoint: String = ""
	var name: String = ""
	var description: String = ""
	var permalink: String = ""
	var contract_address: String = ""
	var token_id: String = ""
	var image_url: String = ""
	var background_color: Color
	var texture: ImageTexture = null
	var last_sell_erc20: Erc20
	var number_of_offers: int = 0
	var username: String = ""
	var address: String = ""
	var average_price: float = 0.0
	var average_price_in_dollars: float = 0.0

	func _get_or_empty_string(dict: Dictionary, key: String) -> String:
		var value = dict.get(key, null)
		if value is String:
			return value
		return ""

	func _init(_endpoint: String, asset: Dictionary):
		self.endpoint = _endpoint
		contract_address = asset.get("asset_contract", {}).get("address", "")
		token_id = asset.get("token_id")

		self.name = _get_or_empty_string(asset, "name")
		self.description = _get_or_empty_string(asset, "description")
		self.permalink = _get_or_empty_string(asset, "permalink")

		var color = asset.get("background_color")
		if color is String:
			background_color = Color("#" + color)
		else:
			background_color = Color.TRANSPARENT

		# average price
		average_price = asset.get("collection", {}).get("stats", {}).get("average_price", 0.0)

		# last sell
		var last_sale = asset.get("last_sale", {})
		if last_sale != null:
			var total_price = last_sale.get("total_price", "0")
			var payment_token = last_sale.get("payment_token", {})
			var decimals = int(payment_token.get("decimals", 18))
			var symbol = payment_token.get("symbol", "")
			var usd_price = float(payment_token.get("usd_price"))
			var eth_price = float(payment_token.get("eth_price"))
			var value = DclEther.format_units(total_price, decimals)
			last_sell_erc20 = Erc20.new(value, symbol, usd_price, eth_price)

			# average price
			if last_sell_erc20 != null:
				average_price_in_dollars = average_price * eth_price * usd_price

		# image
		image_url = asset.get("image_url", "")
		# force format to png
		image_url = image_url.replace("&auto=format", "")
		image_url += "&format=png"

		# top ownership
		var owner = asset.get("top_ownerships", [{}])[0].get("owner", {})
		var user = owner.get("user", {})
		if user:
			self.username = _get_or_empty_string(user, "username")
		self.address = _get_or_empty_string(owner, "address")

		if !token_id.is_empty() and !image_url.is_empty() and !contract_address.is_empty():
			valid = true

	func average_price_to_string():
		var eth = "ETH " + str(snappedf(average_price, 0.0001))
		if average_price_in_dollars > 0.1:
			var usd = "US$" + str(snappedf(average_price_in_dollars, 0.01))
			return eth + " (" + usd + ")"

		return eth

	func async_load_offers() -> int:
		# Request
		var url = endpoint + "/offers"
		var headers = [
			"Content-Type: application/json",
			"X-API-KEY: " + API_KEY,
		]
		var offers_promise: Promise = Global.http_requester.request_json(
			url, HTTPClient.METHOD_GET, "", headers
		)
		var offers_result = await offers_promise.async_awaiter()
		if offers_result is Promise.Error:
			printerr("Asset::async_load_offers error loading offers: ", offers_result.get_error())
			return 0
		# Parsing
		var result = offers_result.get_string_response_as_json()
		var seaport_offers: Array = result.get("seaport_offers", [])
		number_of_offers = seaport_offers.size()
		return number_of_offers

	func get_owner_name():
		var short_address = DclEther.shorten_eth_address(self.address)
		if self.username.is_empty():
			return short_address

		return self.username + " (" + short_address + ")"

# Commented code if in the future we want to search for best_offers...
# 		if seaport_offers.is_empty():
#			return
#		var best_offer: BigNumber = BigNumber.new(seaport_offers[0].get("current_price", "0"))
#		for i in range(1, seaport_offers.size() - 1):
#			var offer = BigNumber.new(seaport_offers[i].get("current_price", "0"))
#			if offer.is_larger_than(best_offer):
#				best_offer = offer

	func async_download_image():
		var texture_hash = get_hash()
		var promise = Global.content_manager.fetch_texture_by_url(texture_hash, image_url)
		var result = await promise.async_awaiter()
		if result is Promise.Error:
			printerr(
				"open_sea_nft_fetcher::asset::download_image promise error: ", result.get_error()
			)
			return
		self.texture = result

	func get_hash() -> String:
		return contract_address + ":" + token_id


var cached_promises: Dictionary


func fetch_nft(urn: DclUrn) -> Promise:
	var cached_promise: Promise = cached_promises.get(urn.get_hash(), null)
	if cached_promise != null:
		return cached_promise

	var promise = Promise.new()
	cached_promises[urn.get_hash()] = promise
	_async_request_nft(promise, urn)

	return promise


func _async_request_nft(completed_promise: Promise, urn: DclUrn):
	var url = RETRIEVE_ASSETS_ENDPOINT + "/" + urn.contract_address + "/" + urn.token_id
	var headers = [
		"Content-Type: application/json",
		"X-API-KEY: " + API_KEY,
	]
	var asset_promise: Promise = Global.http_requester.request_json(
		url, HTTPClient.METHOD_GET, "", headers
	)
	var asset_result = await asset_promise.async_awaiter()

	if asset_result is Promise.Error:
		printerr(
			"Error fetching asset result for opensea, url: ",
			url,
			" error:",
			asset_result.get_error()
		)
		return

	var asset_json: Dictionary = asset_result.get_string_response_as_json()
	var asset: Asset = Asset.new(url, asset_json)
	if not asset.valid:
		printerr("Error on opensea nft asset: ", asset.get_hash())
		completed_promise.reject("Error on opensea nft asset: " + asset.get_hash())
		return

	await asset.async_download_image()

	completed_promise.resolve_with_data(asset)
