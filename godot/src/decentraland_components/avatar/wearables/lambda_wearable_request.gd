class_name WearableRequest

const WEARABLE_PAGE_SIZE = 100


class LambdaWearableItemResponse:
	var urn: String = ""
	var token_id: String = ""
	var transferet_at: int = 0
	var wearable_name: String = ""
	var wearable_category: String = ""
	var wearable_rarity: String = ""

	static func from_dictionary(dictionary: Dictionary) -> Array[LambdaWearableItemResponse]:
		var response: Array[LambdaWearableItemResponse] = []
		var individual_data = dictionary.get("individualData", [{}])
		for data in individual_data:
			var item = LambdaWearableItemResponse.new()
			item.urn = data.get("id", "")
			item.token_id = data.get("tokenId", "")
			item.transferet_at = int(data.get("transferredAt", "0"))

			item.wearable_name = dictionary.get("name", "")
			item.wearable_category = dictionary.get("category", "")
			item.wearable_rarity = dictionary.get("rarity", "")
			response.push_back(item)
		return response


class LambdaWearableResponse:
	var elements: Array[LambdaWearableItemResponse]
	var total_amount: int = 0

	static func from_dictionary(dictionary: Dictionary) -> LambdaWearableResponse:
		var response = LambdaWearableResponse.new()

		var emotes = dictionary.get("elements", [])
		for emote in emotes:
			response.elements.append_array(LambdaWearableItemResponse.from_dictionary(emote))

		response.total_amount = dictionary.get("totalAmount", 0)
		return response


static func async_fetch_emote(emote_urn: String):
	var emote_data_promises = Global.content_provider.fetch_wearables(
		[emote_urn], Global.realm.get_profile_content_url()
	)
	await PromiseUtils.async_all(emote_data_promises)


static func _async_request(
	url: String, page_number: int = 1, page_size: int = 10
) -> LambdaWearableResponse:
	url += "?pageNum=%d" % page_number
	url += "&pageSize=%d" % page_size

	var promise: Promise = Global.http_requester.request_json(url, HTTPClient.METHOD_GET, "", [])

	var result = await PromiseUtils.async_awaiter(promise)

	if result is PromiseError:
		printerr("Error request emotes: ", result.get_error())
		return

	var json: Dictionary = result.get_string_response_as_json()

	return LambdaWearableResponse.from_dictionary(json)


static func async_request_emotes(
	page_number: int = 1, page_size: int = 10
) -> LambdaWearableResponse:
	var address = Global.player_identity.get_address_str()
	if address.is_empty():
		return

	var url = Global.realm.get_lambda_server_base_url() + "users/" + address + "/emotes"

	return await _async_request(url, page_number, page_size)


static func async_request_all_wearables() -> LambdaWearableResponse:
	var response: LambdaWearableResponse = LambdaWearableResponse.new()
	var page_number = 1
	while true:
		var wearables = await async_request_wearables(page_number, WEARABLE_PAGE_SIZE)
		if not is_instance_valid(wearables):
			return null
		response.total_amount = wearables.total_amount
		response.elements.append_array(wearables.elements)
		var loaded_elements = page_number * WEARABLE_PAGE_SIZE
		if loaded_elements >= response.total_amount:
			break
		page_number += 1

	return response


static func async_request_wearables(
	page_number: int = 1, page_size: int = 10
) -> LambdaWearableResponse:
	var address = Global.player_identity.get_address_str()
	if address.is_empty():
		return null

	var url = Global.realm.get_lambda_server_base_url() + "users/" + address + "/wearables"

	return await _async_request(url, page_number, page_size)
