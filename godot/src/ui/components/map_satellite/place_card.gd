extends Control

@export var img_url:String= ''
@export var title_place:String= "Place's Name"
@export var contact_name:String= ''
@onready var texture_rect: TextureRect = %TextureRect
@onready var panel_to_round: PanelContainer = %PanelToRound
@onready var label_title: Label = %LabelTitle
@onready var label_creator: Label = %LabelCreator
@onready var creator_h_box_container: HBoxContainer = %CreatorHBoxContainer

func _ready() -> void:
	panel_to_round.clip_contents = true
	label_title.text = title_place
	label_creator.text = contact_name
	_async_download_image(img_url)
	
	if contact_name.length() <= 0:
		creator_h_box_container.hide()

func _async_download_image(url: String):
	var url_hash = get_hash_from_url(url)
	var promise = Global.content_provider.fetch_texture_by_url(url_hash, url)
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		printerr("places_generator::_async_download_image promise error: ", result.get_error())
		return
	if is_instance_valid(self):  # maybe was deleted...
		set_image(result.texture)

func get_hash_from_url(url: String) -> String:
	if url.contains("/content/contents/"):
		var parts = url.split("/")
		return parts[parts.size() - 1]  # Return the last part

	# Convert URL to a hexadecimal
	var context := HashingContext.new()
	if context.start(HashingContext.HASH_SHA256) == OK:
		# Convert the URL string to UTF-8 bytes and update the context with this data
		context.update(url.to_utf8_buffer())
		# Finalize the hashing process and get the hash as a PackedByteArray
		var url_hash: PackedByteArray = context.finish()
		# Encode the hash as hexadecimal
		return url_hash.hex_encode()

	return "temp-file"
	
func set_image(_texture: Texture2D):
	if is_instance_valid(texture_rect):
		texture_rect.texture = _texture
