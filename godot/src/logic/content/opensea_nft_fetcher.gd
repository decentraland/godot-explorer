# Fetcher NFTs for OpenSea...
# It will batch all the request in a period of time, and then it will request those
class_name OpenSeaFetcher

# issue: opensea.decentraland.org endpoints is not working for multiple request...
const BATCH_QUERY := true

# DCL OpenSea
const RETRIEVE_ASSETS_ENDPOINT := "https://opensea.decentraland.org/api/v1/assets"

# OpenSea
#const RETRIEVE_ASSETS_ENDPOINT := "https://api.opensea.io/api/v1/assets"
const API_KEY := ""

const QUERY_MAX_LENGTH := 1085
const BATCH_TIMEOUT_IN_SECS := 1.0


class _Request:
	var urn: DclUrn
	var promise: Promise = Promise.new()

	func _init(urn: DclUrn):
		self.urn = urn


class Asset:
	var valid: bool = false
	var contract_address: String = ""
	var token_id: String = ""
	var image_url: String = ""
	var background_color: String = ""
	var texture: ImageTexture = null

	func _init(asset: Dictionary):
		contract_address = asset.get("asset_contract", {}).get("address", "")
		token_id = asset.get("token_id")
		var color = asset.get("background_color")
		if color is String:
			background_color = color
		image_url = asset.get("image_url", "")

		# force format to png
		image_url = image_url.replace("&auto=format", "")
		image_url += "&format=png"

		if !token_id.is_empty() and !image_url.is_empty() and !contract_address.is_empty():
			valid = true

	func download_image():
		var local_file = OS.get_user_data_dir() + "/content/" + get_hash() + ".png"
		if not FileAccess.file_exists(local_file):
			var promise = Global.http_requester.request_file(image_url, local_file)
			await promise.co_awaiter()

		# Maybe can we done in a thread using the content thread pool...
		var file = FileAccess.open(local_file, FileAccess.READ)
		if file == null:
			printerr("Opening texture `" + local_file + "` fails: ")
			return

		var buf = file.get_buffer(file.get_length())
		var image := Image.new()
		var err = image.load_png_from_buffer(buf)
		if err != OK:
			printerr("Texture " + image_url + " couldn't be loaded succesfully: ", err)
			return
		self.texture = ImageTexture.create_from_image(image)

	func get_hash() -> String:
		return contract_address + ":" + token_id


var pending_requests: Dictionary
var cached_requests: Dictionary


func fetch_nft(urn: DclUrn) -> Promise:
	var requested_request: _Request = pending_requests.get(urn.get_hash(), null)
	if requested_request != null:
		return requested_request.promise

	var cached_request: _Request = cached_requests.get(urn.get_hash(), null)
	if cached_request != null:
		return cached_request.promise

	var request = _Request.new(urn)
	pending_requests[request.urn.get_hash()] = request

	if pending_requests.size() == 1:
		_schedule_process_request()

	return request.promise


func _schedule_process_request():
	if BATCH_QUERY:
		Global.get_tree().create_timer(BATCH_TIMEOUT_IN_SECS).timeout.connect(
			self._process_requests, CONNECT_ONE_SHOT
		)
	else:
		self._process_requests()


func _process_requests():
	var requests: Dictionary = {}

	var query: String = ""
	for key in pending_requests.keys():
		var request = pending_requests[key]

		if !requests.has(request.urn.get_hash()):  # avoid duplicated requests
			query += (
				"asset_contract_addresses=%s&token_ids=%s&"
				% [request.urn.contract_address, request.urn.token_id]
			)
			requests[request.urn.get_hash()] = request
			if query.length() >= QUERY_MAX_LENGTH:
				pending_requests.erase(key)
				if !pending_requests.is_empty():
					_schedule_process_request()  # if we have more to process, we schedule another process
				break

		pending_requests.erase(key)

		if not BATCH_QUERY:  # one request per process
			break

	var url = RETRIEVE_ASSETS_ENDPOINT + "?" + query
	var headers = [
		"Content-Type: application/json",
		"X-API-KEY: " + API_KEY,
	]
	var promise: Promise = Global.http_requester.request_json(
		url, HTTPClient.METHOD_GET, "", headers
	)
	var result = await promise.co_awaiter()

	if result is Promise.Error:
		printerr("Error processing opensea nft fetch, url: ", url)
		return

	var json: Dictionary = result.get_string_response_as_json()
	for asset in json.get("assets", []):
		var opensea_asset: Asset = Asset.new(asset)
		if not opensea_asset.valid:
			printerr("Error on opensea nft asset: ", opensea_asset.get_hash())
			continue

		await opensea_asset.download_image()

		var request: _Request = requests.get(opensea_asset.get_hash())
		request.promise.resolve_with_data(opensea_asset)
		cached_requests[opensea_asset.get_hash()] = request
