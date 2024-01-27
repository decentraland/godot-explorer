extends Button

signal equip(wearable_id: String)
signal unequip(wearable_id: String)
signal info(wearable_id: String)

const WEARABLE_PANEL = preload("res://src/ui/components/wearable_panel/wearable_panel.tscn")
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
var wearable_data: Dictionary

@onready var panel_container_black = $PanelContainer_Black
@onready var panel_container_white = $PanelContainer_Black/PanelContainer_White

@onready var button_equip = $PanelContainer_Black/PanelContainer_White/VBoxContainer/Button_Equip

@onready
var texture_rect_equiped = $PanelContainer_Black/PanelContainer_White/VBoxContainer/Panel/TextureRect_Preview/TextureRect_Equiped
@onready
var texture_rect_category = $PanelContainer_Black/PanelContainer_White/VBoxContainer/Panel/TextureRect_Preview/TextureRect_Category
@onready
var texture_rect_background = $PanelContainer_Black/PanelContainer_White/VBoxContainer/Panel/TextureRect_Background
@onready
var texture_rect_preview = $PanelContainer_Black/PanelContainer_White/VBoxContainer/Panel/TextureRect_Preview


func _ready():
	pass


func async_set_wearable(wearable: Dictionary):
	wearable_id = wearable.get("id", "")
	var wearable_thumbnail: String = wearable.get("metadata", {}).get("thumbnail", "")
	thumbnail_hash = wearable.get("content").get_hash(wearable_thumbnail)
	_update_category_icon(wearable)
	wearable_data = wearable

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


func _on_toggled(_button_pressed):
	if button_pressed:
		size = panel_container_black.size
		self.equip.emit()
		button_equip.show()
		panel_container_white.self_modulate = Color("#ffffff00")
		panel_container_black.self_modulate = Color("#ffffff")
		z_index = 5
	else:
		self.unequip.emit()
		button_equip.hide()
		size = panel_container_white.size
		panel_container_white.self_modulate = Color("#ffffff")
		panel_container_black.self_modulate = Color("#ffffff00")
		z_index = 0


func set_equiped(is_equiped: bool):
	if is_equiped:
		texture_rect_equiped.show()
	else:
		texture_rect_equiped.hide()


func _update_category_icon(wearable: Dictionary):
	var texture_path = (
		"res://assets/wearable_categories/"
		+ wearable.get("metadata", "").get("data", "").get("category", "")
		+ "-icon.svg"
	)
	if FileAccess.file_exists(texture_path):
		var texture = load(texture_path)
		if texture != null:
			texture_rect_category.texture = texture
