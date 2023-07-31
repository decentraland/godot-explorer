extends Control
@onready var texture_rect_panel_background = $TextureRect_PanelBackground
@onready
var texture_rect_thumbnail_background = $HBoxContainer/MarginContainer2/TextureRect_ThumbnailBackground

@onready var label_name = $HBoxContainer/MarginContainer3/VBoxContainer/HBoxContainer_Name/Label_Name
@onready var button_equip = $HBoxContainer/MarginContainer/Button_Equip
@onready
var texture_rect_preview = $HBoxContainer/MarginContainer2/TextureRect_ThumbnailBackground/TextureRect_Preview

var thumbnail_hash: String

var base_panel = preload("res://assets/ui/InfoCardBase.png")
var common_panel = preload("res://assets/ui/CommonThumbnail.png")
var uncommon_panel = preload("res://assets/ui/InfoCardUncommon.png")
var rare_panel = preload("res://assets/ui/InfoCardRare.png")
var epic_panel = preload("res://assets/ui/InfoCardEpic.png")
var mythic_panel = preload("res://assets/ui/InfoCardMythic.png")
var legendary_panel = preload("res://assets/ui/InfoCardLegendary.png")
var unique_panel = preload("res://assets/ui/InfoCardUnique.png")

var base_thumbnail = preload("res://assets/ui/BaseThumbnail.png")
var common_thumbnail = preload("res://assets/ui/CommonThumbnail.png")
var uncommon_thumbnail = preload("res://assets/ui/UncommonThumbnail.png")
var rare_thumbnail = preload("res://assets/ui/RareThumbnail.png")
var epic_thumbnail = preload("res://assets/ui/EpicThumbnail.png")
var mythic_thumbnail = preload("res://assets/ui/MythicThumbnail.png")
var legendary_thumbnail = preload("res://assets/ui/LegendaryThumbnail.png")
var unique_thumbnail = preload("res://assets/ui/UniqueThumbnail.png")

signal equip(wearable_id: String)
signal unequip(wearable_id: String)

var wearable_id


func _ready():
	unset_wearable()


func set_wearable(wearable: Dictionary, _wearable_id: String):
	show()

	wearable_id = _wearable_id

	var wearable_name: String = wearable.get("metadata", {}).get("name", "")
	var wearable_display: Array = wearable.get("metadata", {}).get("i18n", [])

	match wearable.get("rarity", ""):
		_:
			texture_rect_panel_background.texture = base_panel
			texture_rect_thumbnail_background.texture = base_thumbnail
		"common":
			texture_rect_panel_background.texture = common_panel
			texture_rect_thumbnail_background.texture = common_thumbnail
		"uncommon":
			texture_rect_panel_background.texture = uncommon_panel
			texture_rect_thumbnail_background.texture = uncommon_thumbnail
		"rare":
			texture_rect_panel_background.texture = rare_panel
			texture_rect_thumbnail_background.texture = rare_thumbnail
		"epic":
			texture_rect_panel_background.texture = epic_panel
			texture_rect_thumbnail_background.texture = epic_thumbnail
		"legendary":
			texture_rect_panel_background.texture = legendary_panel
			texture_rect_thumbnail_background.texture = legendary_thumbnail
		"mythic":
			texture_rect_panel_background.texture = mythic_panel
			texture_rect_thumbnail_background.texture = mythic_thumbnail
		"unique":
			texture_rect_panel_background.texture = unique_panel
			texture_rect_thumbnail_background.texture = unique_thumbnail

	if wearable_display.size() > 0:
		label_name.text = wearable_display[0].get("text")
	else:
		label_name.text = wearable_name

	var wearable_thumbnail: String = wearable.get("metadata", {}).get("thumbnail", "")
	thumbnail_hash = wearable.get("content", {}).get(wearable_thumbnail, "")

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


func set_equipable_and_equip(equipable: bool, equipped: bool):
	button_equip.disabled = not equipable
	if not equipable:
		button_equip.text = "UNAVAILABLE"
		button_equip.button_pressed = false
	elif equipped:
		button_equip.text = "UNEQUIP"
		button_equip.button_pressed = true
	else:
		button_equip.text = "EQUIP"
		button_equip.button_pressed = false


func load_thumbnail():
	var image = Global.content_manager.get_resource_from_hash(thumbnail_hash)
	var texture = ImageTexture.create_from_image(image)
	texture_rect_preview.texture = texture


func unset_wearable():
	hide()


func _on_button_equip_toggled(button_pressed):
	if button_pressed:
		self.equip.emit(wearable_id)
		button_equip.text = "UNEQUIP"
	else:
		self.unequip.emit(wearable_id)
		button_equip.text = "EQUIP"
