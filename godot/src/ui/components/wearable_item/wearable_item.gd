extends Button

@onready var panel_container = $PanelContainer
@onready var texture_rect_background = $Panel/TextureRect_Background
@onready var texture_rect_preview = $Panel/TextureRect_Preview

var base_thumbnail = preload("res://assets/ui/BaseThumbnail.png")
var common_thumbnail = preload("res://assets/ui/CommonThumbnail.png")
var uncommon_thumbnail = preload("res://assets/ui/UncommonThumbnail.png")
var rare_thumbnail = preload("res://assets/ui/RareThumbnail.png")
var epic_thumbnail = preload("res://assets/ui/EpicThumbnail.png")
var mythic_thumbnail = preload("res://assets/ui/MythicThumbnail.png")
var legendary_thumbnail = preload("res://assets/ui/LegendaryThumbnail.png")
var unique_thumbnail = preload("res://assets/ui/UniqueThumbnail.png")

var thumbnail_hash: String


func _ready():
	if button_pressed:
		panel_container.show()


func set_wearable(wearable: Dictionary):
	var wearable_display: Array = wearable.get("metadata", {}).get("i18n", [])

	var wearable_thumbnail: String = wearable.get("metadata", {}).get("thumbnail", "")
	thumbnail_hash = wearable.get("content", {}).get(wearable_thumbnail, "")

	match wearable.get("rarity", ""):
		_:
			texture_rect_background.texture = base_thumbnail
		"common":
			texture_rect_background.texture = common_thumbnail
		"uncommon":
			texture_rect_background.texture = uncommon_thumbnail
		"rare":
			texture_rect_background.texture = rare_thumbnail
		"epic":
			texture_rect_background.texture = epic_thumbnail
		"legendary":
			texture_rect_background.texture = legendary_thumbnail
		"mythic":
			texture_rect_background.texture = mythic_thumbnail
		"unique":
			texture_rect_background.texture = unique_thumbnail

	if not thumbnail_hash.is_empty():
		if Global.content_manager.get_resource_from_hash(thumbnail_hash) == null:
			var content_mapping: Dictionary = {
				"content": wearable.get("content", {}),
				"base_url": "https://peer.decentraland.org/content/contents/"
			}
			Global.content_manager.fetch_texture(wearable_thumbnail, content_mapping)
			Global.content_manager.content_loading_finished.connect(
				self._on_content_loading_finished
			)
		else:
			load_thumbnail()


func _on_content_loading_finished(content_hash: String):
	if content_hash != thumbnail_hash:
		return

	Global.content_manager.content_loading_finished.disconnect(self._on_content_loading_finished)
	load_thumbnail()


func load_thumbnail():
	var image = Global.content_manager.get_resource_from_hash(thumbnail_hash)
	var texture = ImageTexture.create_from_image(image)
	texture_rect_preview.texture = texture


func _on_mouse_entered():
	scale = Vector2(1.1, 1.1)
	if not button_pressed:
		panel_container.show()


func _on_mouse_exited():
	scale = Vector2(1, 1)
	if not button_pressed:
		panel_container.hide()


func _on_toggled(_button_pressed):
	if _button_pressed:
		panel_container.show()
	else:
		panel_container.hide()
