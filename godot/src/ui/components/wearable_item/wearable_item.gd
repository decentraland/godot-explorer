class_name WearableItem
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
var panel_container_external_orig_rect: Rect2
var was_pressed = false

@onready var panel_container_external = $PanelContainer_External

@onready var texture_rect_equiped = %TextureRect_Equiped
@onready var texture_rect_category = %TextureRect_Category
@onready var texture_rect_background = %TextureRect_Background
@onready var texture_rect_preview = %TextureRect_Preview


func _ready():
	panel_container_external_orig_rect = panel_container_external.get_rect()
	panel_container_external.hide()


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
			var current_size = texture_rect_preview.size
			texture_rect_preview.texture = res.texture
			texture_rect_preview.size = current_size


func effect_toggle():
	if button_pressed:
		if was_pressed == false:
			var new_size = panel_container_external_orig_rect.size * 0.9
			var new_position = (panel_container_external_orig_rect.size - new_size) / 2
			panel_container_external.set_position(new_position)
			panel_container_external.set_size(new_size)

			var tween = get_tree().create_tween().set_parallel(true)
			tween.tween_property(
				panel_container_external,
				"position",
				panel_container_external_orig_rect.position,
				0.15
			)
			tween.tween_property(
				panel_container_external, "size", panel_container_external_orig_rect.size, 0.15
			)

			panel_container_external.show()
	else:
		panel_container_external.hide()

	was_pressed = button_pressed


func _on_toggled(_button_pressed):
	if _button_pressed:
		self.equip.emit()
	else:
		self.unequip.emit()
	set_equiped(_button_pressed)


func set_equiped(is_equiped: bool):
	if is_equiped:
		texture_rect_equiped.show()
	else:
		texture_rect_equiped.hide()
	effect_toggle()


func _update_category_icon(wearable: Dictionary):
	var texture_path = (
		"res://assets/ui/wearable_categories/"
		+ wearable.get("metadata", "").get("data", "").get("category", "")
		+ "-icon.svg"
	)
	if FileAccess.file_exists(texture_path):
		var texture = load(texture_path)
		if texture != null:
			texture_rect_category.texture = texture
