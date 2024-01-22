extends Button

signal equip(wearable_id: String)
signal unequip(wearable_id: String)

var base_thumbnail = preload("res://assets/ui/BaseThumbnail.png")
var common_thumbnail = preload("res://assets/ui/CommonThumbnail.png")
var uncommon_thumbnail = preload("res://assets/ui/UncommonThumbnail.png")
var rare_thumbnail = preload("res://assets/ui/RareThumbnail.png")
var epic_thumbnail = preload("res://assets/ui/EpicThumbnail.png")
var mythic_thumbnail = preload("res://assets/ui/MythicThumbnail.png")
var legendary_thumbnail = preload("res://assets/ui/LegendaryThumbnail.png")
var unique_thumbnail = preload("res://assets/ui/UniqueThumbnail.png")
var thumbnail_hash: String
var wearable_id: String

@onready var texture_rect_background = $PanelContainer/VBoxContainer/Panel/TextureRect_Background
@onready var texture_rect_preview = $PanelContainer/VBoxContainer/Panel/TextureRect_Preview


@onready var panel_container = $PanelContainer
@onready var button_info = $PanelContainer/VBoxContainer/Button_Info
@onready var panel = $PanelContainer/VBoxContainer/Panel


func _ready():
	if button_pressed:
		button_info.show()
		panel_container.self_modulate = Color("#161518")
	else:
		button_info.hide()
		panel_container.self_modulate = Color("#ffffff")


func async_set_wearable(wearable: Dictionary):
	wearable_id = wearable.get("id", "")
	var wearable_thumbnail: String = wearable.get("metadata", {}).get("thumbnail", "")
	thumbnail_hash = wearable.get("content").get_hash(wearable_thumbnail)

	match wearable.get("rarity", ""):
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
		_:
			texture_rect_background.texture = base_thumbnail

	if not thumbnail_hash.is_empty():
		var dcl_content_mapping = wearable.get("content")
		var promise: Promise = Global.content_provider.fetch_texture(
			wearable_thumbnail, dcl_content_mapping
		)
		var res = await PromiseUtils.async_awaiter(promise)
		if res is PromiseError:
			printerr("Fetch texture error on ", wearable_thumbnail, ": ", res.get_error())
		else:
			texture_rect_preview.texture = res.texture


func _on_mouse_entered():
	scale = Vector2(1.1, 1.1)
	


func _on_mouse_exited():
	scale = Vector2(1, 1)
	


func _on_toggled(_button_pressed):
	
	if _button_pressed:
		self.equip.emit(wearable_id)
		button_info.show()
		panel_container.self_modulate = Color("#161518")

	else:
		self.unequip.emit(wearable_id)
		button_info.hide()
		panel_container.self_modulate = Color("#ffffff")


