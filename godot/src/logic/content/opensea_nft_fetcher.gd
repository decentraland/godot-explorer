# Fetcher NFTs for OpenSea...
# It will batch all the request in a period of time, and then it will request those
class_name OpenSeaFetcher

# DCL OpenSea
const RETRIEVE_ASSETS_ENDPOINT := "https://opensea.decentraland.org/api/v2/chain/%s/contract/%s/nfts/%s"

# OpenSea
#const RETRIEVE_ASSETS_ENDPOINT := "https://api.opensea.io/api/v2/chain/%s/contract/%s/nfts/%s"
const API_KEY := ""


class Asset:
	var valid: bool = false
	var endpoint: String = ""
	var name: String = ""
	var description: String = ""
	var opensea_url: String = ""
	var contract_address: String = ""
	var token_id: String = ""
	var image_url: String = ""
	var background_color: Color
	var texture: ImageTexture = null
	var username: String = ""  # TODO: Need to fetch to users
	var address: String = ""

	func _get_or_empty_string(dict: Dictionary, key: String) -> String:
		var value = dict.get(key, null)
		if value is String:
			return value
		return ""

	func _init(_endpoint: String, asset: Dictionary):
		self.endpoint = _endpoint
		var nft = asset.get("nft")
		contract_address = nft.get("contract", "")
		token_id = nft.get("identifier", "")

		self.name = _get_or_empty_string(nft, "name")
		self.description = _get_or_empty_string(nft, "description")
		self.opensea_url = _get_or_empty_string(nft, "opensea_url")

		background_color = Color.TRANSPARENT

		# image
		image_url = nft.get("image_url", "")

		# top ownership
		var owner = nft.get("owners", [{}])[0]
		self.address = _get_or_empty_string(owner, "address")

		if !token_id.is_empty() and !image_url.is_empty() and !contract_address.is_empty():
			valid = true

	func get_owner_name():
		var short_address = DclEther.shorten_eth_address(self.address)
		if self.username.is_empty():
			return short_address

		return self.username + " (" + short_address + ")"

	func async_download_image():
		var texture_hash = get_hash()
		var promise = Global.content_provider.fetch_texture_by_url(texture_hash, image_url)
		var result = await PromiseUtils.async_awaiter(promise)
		if result is PromiseError:
			printerr(
				"open_sea_nft_fetcher::asset::download_image promise error: ", result.get_error()
			)
			return
		self.texture = result.texture

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
	var url = RETRIEVE_ASSETS_ENDPOINT % [urn.chain, urn.contract_address, urn.token_id]
	var headers = [
		"Content-Type: application/json",
		"X-API-KEY: " + API_KEY,
	]
	var asset_promise: Promise = Global.http_requester.request_json(
		url, HTTPClient.METHOD_GET, "", headers
	)
	var asset_result = await PromiseUtils.async_awaiter(asset_promise)

	if asset_result is PromiseError:
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
