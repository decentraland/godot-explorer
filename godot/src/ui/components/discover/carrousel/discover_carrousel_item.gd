class_name PlaceItem
extends Control

signal item_pressed(data)

@onready var label_title := %Label_Title
@onready var label_description := %Label_Description

@onready var label_online := %Label_Online
@onready var container_online := %Container_Online

@onready var label_views := %Label_Views
@onready var container_views := %HBoxContainer_Views

@onready var label_likes := %Label_Likes
@onready var container_likes := %HBoxContainer_Likes

@onready var texture_image = %TextureRect_Image

@export var texture: Texture2D = load("res://assets/ui/placeholder.png")

@export var title: String = "Scene Title"
		
@export var description: String = "Scene Description"
		
@export var views: int = 0
		
@export var onlines: int = 0
		
@export var likes_percent: float = 0.0
		
@export var metadata: Dictionary = {}

var _data = null

func _ready():
	if metadata.is_empty():
		set_image(texture)
		set_views(views)
		set_online(onlines)
		set_title(title)
		set_description(description)
		set_likes_percent(likes_percent)
	else:
		set_data(metadata)


func set_image(_texture: Texture2D):
	if is_instance_valid(texture_image):
		texture_image.texture = _texture


func set_title(_title: String):
	label_title.text = _title


func set_description(_description: String):
	label_description.text = _description


func set_views(_views: int):
	container_views.set_visible(_views > 0)
	label_views.text = _format_number(_views)
	
	
func set_likes_percent(_likes: float):
	container_likes.set_visible(_likes > 0.0)
	label_likes.text = str(round(_likes * 100)) + "%"


func set_online(_online: int):
	container_online.set_visible(_online > 0)
	label_online.text = _format_number(_online)


func _format_number(num: int) -> String:
	if num < 1e3:
		return str(num)
	if num < 1e6:
		return str(ceil(num / 1000.0)) + "k"
	return str(floor(num / 1000000.0)) + "M"


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

func _get_or_empty_string(dict: Dictionary, key: String) -> String:
	var value = dict.get(key, null)
	if value is String:
		return value
	return ""

func set_data(item_data):
	_data = item_data
	set_title(item_data.get("title", ""))
	set_description(_get_or_empty_string(item_data, "description"))
	
	set_views(item_data.get("user_visits", 0))
	var like_score = item_data.get("like_score", 0.0)
	set_likes_percent(like_score if like_score is float else 0.0)
	set_online(item_data.get("user_count", 0))
	
	var image_url = item_data.get("image", "")
	if not image_url.is_empty():
		_async_download_image(image_url)

func _async_download_image(url: String):
	var url_hash = get_hash_from_url(url)
	var promise = Global.content_provider.fetch_texture_by_url(url_hash, url)
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		printerr(
			"places_generator::_async_download_image promise error: ", result.get_error()
		)
		return
	if is_instance_valid(self): # maybe was deleted...
		set_image(result.texture)


func _on_pressed():
	item_pressed.emit(_data)
