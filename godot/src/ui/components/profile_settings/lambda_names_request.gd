class_name NamesRequest

const NAMES_PAGE_SIZE = 100


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

		var elements = dictionary.get("elements", [])
		for element in elements:
			response.elements.push_back(LambdaNameItemResponse.from_dictionary(element))

		response.total_amount = dictionary.get("totalAmount", 0)
		return response


static func _async_request(
	url: String, page_number: int = 1, page_size: int = 10
) -> LambdaNamesResponse:
	url += "?pageNum=%d" % page_number
	url += "&pageSize=%d" % page_size

	var promise: Promise = Global.http_requester.request_json(url, HTTPClient.METHOD_GET, "", [])

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

	var url = Global.player_identity.current_lambda_server_base_url + "users/" + address + "/names"

	return await _async_request(url, page_number, page_size)


static func async_request_all_names() -> LambdaNamesResponse:
	var response: LambdaNamesResponse = LambdaNamesResponse.new()
	var page_number = 1
	while true:
		var names = await async_request_names(page_number, NAMES_PAGE_SIZE)
		if not is_instance_valid(names):
			return null
		response.total_amount = names.total_amount
		response.elements.append_array(names.elements)
		var loaded_elements = page_number * NAMES_PAGE_SIZE
		if loaded_elements >= response.total_amount:
			break
		page_number += 1

	return response
