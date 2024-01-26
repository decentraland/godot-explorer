extends Control

signal equip(wearable_id: String)
signal unequip(wearable_id: String)

var thumbnail_hash: String

var base_thumbnail = preload("res://assets/ui/BaseThumbnail.png")
var common_thumbnail = preload("res://assets/ui/CommonThumbnail.png")
var uncommon_thumbnail = preload("res://assets/ui/UncommonThumbnail.png")
var rare_thumbnail = preload("res://assets/ui/RareThumbnail.png")
var epic_thumbnail = preload("res://assets/ui/EpicThumbnail.png")
var mythic_thumbnail = preload("res://assets/ui/MythicThumbnail.png")
var legendary_thumbnail = preload("res://assets/ui/LegendaryThumbnail.png")
var unique_thumbnail = preload("res://assets/ui/UniqueThumbnail.png")


var wearable_id

@onready var button_equip = $HBoxContainer/MarginContainer/Button_Equip
@onready var texture_rect_thumbnail_background = $HBoxContainer/TextureRect_ThumbnailBackground
@onready var texture_rect_preview = $HBoxContainer/TextureRect_ThumbnailBackground/TextureRect_Preview
@onready var wearable_category = $HBoxContainer/VBoxContainer/HBoxContainer_Name/Wearable_Category
@onready var label_name = $HBoxContainer/VBoxContainer/HBoxContainer_Name/Label_Name





func _ready():
	unset_wearable()


func async_set_wearable(wearable: Dictionary, _wearable_id: String):
	show()

	wearable_id = _wearable_id

	var wearable_name: String = wearable.get("metadata", {}).get("name", "")
	var wearable_display: Array = wearable.get("metadata", {}).get("i18n", [])


	match wearable.get("rarity", ""):
		"common":
			texture_rect_thumbnail_background.texture = common_thumbnail
		"uncommon":
			texture_rect_thumbnail_background.texture = uncommon_thumbnail
		"rare":
			texture_rect_thumbnail_background.texture = rare_thumbnail
		"epic":
			texture_rect_thumbnail_background.texture = epic_thumbnail
		"legendary":
			texture_rect_thumbnail_background.texture = legendary_thumbnail
		"mythic":
			texture_rect_thumbnail_background.texture = mythic_thumbnail
		"unique":
			texture_rect_thumbnail_background.texture = unique_thumbnail
		_:
			texture_rect_thumbnail_background.texture = base_thumbnail

	if wearable_display.size() > 0:
		label_name.text = wearable_display[0].get("text")
	else:
		label_name.text = wearable_name

	var dcl_content_mapping = wearable.get("content")
	var wearable_thumbnail: String = wearable.get("metadata", {}).get("thumbnail", "")
	thumbnail_hash = dcl_content_mapping.get_hash(wearable_thumbnail)

	if not thumbnail_hash.is_empty():
		var promise = Global.content_provider.fetch_texture(wearable_thumbnail, dcl_content_mapping)
		var res = await PromiseUtils.async_awaiter(promise)
		if res is PromiseError:
			printerr("Fetch texture error on ", wearable_thumbnail, ": ", res.get_error())
		else:
			texture_rect_preview.texture = res.texture


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


func unset_wearable():
	hide()


func _on_button_equip_toggled(button_pressed):
	if button_pressed:
		self.equip.emit(wearable_id)
		button_equip.text = "UNEQUIP"
	else:
		self.unequip.emit(wearable_id)
		button_equip.text = "EQUIP"
