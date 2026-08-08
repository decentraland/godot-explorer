class_name NamesRequest

const NAMES_PAGE_SIZE = 100

static var _cache: LambdaNamesResponse = null
static var _is_fetching: bool = false


class LambdaNameItemResponse:
	var name: String = ""
	var contract_address: String = ""
	var token_id: String = ""

	static func from_dictionary(dictionary: Dictionary) -> LambdaNameItemResponse:
		var response = LambdaNameItemResponse.new()
		response.name = dictionary.get("name", "")
		response.contract_address = dictionary.get("contractAddress", "")
		response.token_id = dictionary.get("tokenId", "")
		return response


class LambdaNamesResponse:
	var elements: Array[LambdaNameItemResponse] = []
	var total_amount: int = 0

	static func from_dictionary(dictionary: Dictionary) -> LambdaNamesResponse:
		var response = LambdaNamesResponse.new()

		var el = dictionary.get("elements", [])
		for element in el:
			response.elements.push_back(LambdaNameItemResponse.from_dictionary(element))

		response.total_amount = dictionary.get("totalAmount", 0)
		return response


static func _async_request(
	url: String, page_number: int = 1, page_size: int = 10
) -> LambdaNamesResponse:
	url += "?pageNum=%d" % page_number
	url += "&pageSize=%d" % page_size

	var promise: Promise = Global.http_requester.request_json(url, HTTPClient.METHOD_GET, "", {})

	var result = await PromiseUtils.async_awaiter(promise)

	if result is PromiseError:
		printerr("Error request emotes: ", result.get_error())
		return

	var json: Dictionary = result.get_string_response_as_json()

	return LambdaNamesResponse.from_dictionary(json)


static func async_request_names(page_number: int = 1, page_size: int = 10) -> LambdaNamesResponse:
	var address = Global.player_identity.get_address_str()
	if address.is_empty():
		return

	var url = Global.realm.get_lambda_server_base_url() + "users/" + address + "/names"

	return await _async_request(url, page_number, page_size)


static func async_request_all_names() -> LambdaNamesResponse:
	if _cache != null:
		return _cache

	if _is_fetching:
		while _is_fetching:
			await (Engine.get_main_loop() as SceneTree).process_frame
		return _cache

	_is_fetching = true
	var response: LambdaNamesResponse = LambdaNamesResponse.new()
	var page_number = 1
	while true:
		var names = await async_request_names(page_number, NAMES_PAGE_SIZE)
		if not is_instance_valid(names):
			_is_fetching = false
			return response
		response.total_amount = names.total_amount
		response.elements.append_array(names.elements)
		var loaded_elements = page_number * NAMES_PAGE_SIZE
		if loaded_elements >= response.total_amount:
			break
		page_number += 1

	# Always clear the flag before returning, even if invalidate_cache() fired
	# mid-fetch (in that case we discard the result instead of re-caching it).
	_is_fetching = false
	if _cache == null:
		_cache = response
	return _cache


## Starts the names fetch in the background so the result is cached by the time
## the profile editor opens. Safe to call multiple times.
static func pre_fetch() -> void:
	if _cache != null or _is_fetching:
		return
	async_request_all_names()


## Clears the cache and breaks any in-flight waiter loop. Call on wallet change.
static func invalidate_cache() -> void:
	_cache = null
	_is_fetching = false
