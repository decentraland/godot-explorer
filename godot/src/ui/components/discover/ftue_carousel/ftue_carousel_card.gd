class_name FtueCarouselCard
extends Control

var _data: Dictionary = {}

@onready var panel_image: Panel = %Panel_Container_Image
@onready var panel_no_image: PanelContainer = %Panel_Container_NoImage
@onready var texture_image: TextureRect = %TextureRect_Image


func set_data(place_data: Dictionary) -> void:
	_data = place_data
	var image_url: String = place_data.get("image", "")
	if not image_url.is_empty():
		_async_download_image(image_url)


func get_title() -> String:
	return _data.get("title", "")


func get_creator() -> String:
	var contact_name: String = _data.get("contact_name", "")
	if contact_name.is_empty():
		contact_name = _data.get("owner", "")
	return contact_name


func get_place_data() -> Dictionary:
	return _data


func set_image(texture: Texture2D) -> void:
	texture_image.texture = texture
	panel_image.show()
	panel_no_image.hide()


static func _get_hash_from_url(url: String) -> String:
	if url.contains("/content/contents/"):
		var parts = url.split("/")
		return parts[parts.size() - 1]
	var context := HashingContext.new()
	if context.start(HashingContext.HASH_SHA256) == OK:
		context.update(url.to_utf8_buffer())
		return context.finish().hex_encode()
	return "temp-file"


func _async_download_image(url: String) -> void:
	var url_hash := _get_hash_from_url(url)
	var content_mapping = Global.content_provider.fetch_texture_by_url(url_hash, url)
	var result = await PromiseUtils.async_awaiter(content_mapping)
	if result is PromiseError:
		printerr("ftue_carousel_card::_async_download_image error: ", result.get_error())
		return
	if not is_instance_valid(self):
		return
	set_image(result.texture)
