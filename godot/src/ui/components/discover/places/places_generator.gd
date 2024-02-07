extends CarrouselGenerator

enum OrderBy {
	None,
	MostActive,
	LikeScore,
}

@export var order_by: OrderBy = OrderBy.None
@export var categories: String = "all"
@export var only_favorites: bool = false
@export var only_highlighted: bool = false
@export var only_worlds: bool = false

var loaded_elements: int = 0
var no_more_elements: bool = false

# Test
const DISCOVER_CARROUSEL_ITEM = preload(
	"res://src/ui/components/discover/carrousel/discover_carrousel_item.tscn"
)


func on_request(offset: int, limit: int) -> void:
	if no_more_elements:
		return # we reach the capacity...

	var url = "https://places.decentraland.org/api/"
	url += "worlds" if only_worlds else "places"
	
	url += "?offset=%d&limit=%d" % [offset, limit]

	if only_favorites:
		url += "&only_favorites=true"
		
	if only_highlighted:
		url += "&only_highlighted=true"
	
	if order_by != OrderBy.None:
		url += "&order_by=" + ("like_score" if order_by == OrderBy.LikeScore else "most_active")
		
	if categories != "all":
		var categories_array = categories.split(",")
		for category in categories_array:
			url += "&categories=" + category

	prints("url", url)
	var headers = ["Content-Type: application/json"]
	var promise: Promise = Global.http_requester.request_json(
		url, HTTPClient.METHOD_GET, "", headers
	)
	var result = await PromiseUtils.async_awaiter(promise)
	
	if result is PromiseError:
		set_consumer_visible.emit(false)
		printerr('Error request places', result.get_error())
		return
	
	var json: Dictionary = result.get_string_response_as_json()

	if json.data.is_empty():
		if loaded_elements == 0:
			set_consumer_visible.emit(false)
		return

	loaded_elements += json.data.size()
	
	if json.data.size() != limit:
		no_more_elements = true

	for item_data in json.data:
		var item = DISCOVER_CARROUSEL_ITEM.instantiate()
		item_container.add_child(item)
		
		item.set_data(item_data)
		item.item_pressed.connect(discover.on_item_pressed)

	set_consumer_visible.emit(true)

func get_hash_from_url(url: String) -> String:
	if url.contains("/content/contents/"):
		var parts = url.split("/")
		return parts[parts.size() - 1] # Return the last part
	else:
		# Convert URL to a hexadecimal
		var context := HashingContext.new()
		if context.start(HashingContext.HASH_SHA256) == OK:
			# Convert the URL string to UTF-8 bytes and update the context with this data
			context.update(url.to_utf8_buffer())
			# Finalize the hashing process and get the hash as a PackedByteArray
			var url_hash: PackedByteArray = context.finish()
			# Encode the hash as hexadecimal
			return url_hash.hex_encode()
		else:
			return "temp-file"
