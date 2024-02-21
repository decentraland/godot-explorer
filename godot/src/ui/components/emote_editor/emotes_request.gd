class_name EmotesRequest


class EmoteData:
	var urn: String = ""
	var amount: int = 0
	var emote_id: String = ""
	var token_id: String = ""
	var transferet_at: int = 0
	var emote_name: String = ""
	var emote_category: String = ""
	var emote_rarity: String = ""

	static func from_dictionary(dictionary: Dictionary) -> EmoteData:
		var response = EmoteData.new()
		response.urn = dictionary.get("urn", "")
		response.amount = dictionary.get("amount", 0)
		var individual_data = dictionary.get("individualData", [{}])
		var first_item = individual_data[0]
		response.emote_id = first_item.get("id", "")
		response.token_id = first_item.get("tokenId", "")
		response.transferet_at = int(first_item.get("transferredAt", "0"))

		response.emote_name = dictionary.get("name", "")
		response.emote_category = dictionary.get("category", "")
		response.emote_rarity = dictionary.get("rarity", "")
		return response


class EmotesResponse:
	var elements: Array[EmoteData]
	var total_amount: int = 0

	static func from_dictionary(dictionary: Dictionary) -> EmotesResponse:
		var response = EmotesResponse.new()

		var emotes = dictionary.get("elements", [])
		for emote in emotes:
			response.elements.push_back(EmoteData.from_dictionary(emote))

		response.total_amount = dictionary.get("totalAmount", 0)
		return response


static func async_fetch_emote(emote_urn: String):
	var emote_data_promises = Global.content_provider.fetch_wearables(
		[emote_urn], Global.realm.get_profile_content_url()
	)
	await PromiseUtils.async_all(emote_data_promises)


static func async_request_emotes(page_number: int = 1, page_size: int = 10) -> EmotesResponse:
	var address = Global.player_identity.get_address_str()
	if address.is_empty():
		return

	var url = Global.player_identity.current_lambda_server_base_url + "users/" + address + "/emotes"

	url += "?pageNum=%d" % page_number
	url += "&pageSize=%d" % page_size

	var promise: Promise = Global.http_requester.request_json(url, HTTPClient.METHOD_GET, "", [])

	var result = await PromiseUtils.async_awaiter(promise)

	if result is PromiseError:
		printerr("Error request emotes: ", result.get_error())
		return

	var json: Dictionary = result.get_string_response_as_json()

	return EmotesResponse.from_dictionary(json)
