class_name ItemPreview
extends Control

var base_thumbnail = preload("res://assets/ui/BaseThumbnail.png")
var common_thumbnail = preload("res://assets/ui/CommonThumbnail.png")
var uncommon_thumbnail = preload("res://assets/ui/UncommonThumbnail.png")
var rare_thumbnail = preload("res://assets/ui/RareThumbnail.png")
var epic_thumbnail = preload("res://assets/ui/EpicThumbnail.png")
var mythic_thumbnail = preload("res://assets/ui/MythicThumbnail.png")
var legendary_thumbnail = preload("res://assets/ui/LegendaryThumbnail.png")
var unique_thumbnail = preload("res://assets/ui/UniqueThumbnail.png")
var item_data: DclItemEntityDefinition

@onready var texture_rect_category = %TextureRect_Category
@onready var texture_rect_background = %TextureRect_Background
@onready var texture_rect_preview = %TextureRect_Preview
@onready var texture_progress_bar_loading = %TextureProgressBar_Loading
@onready var texture_rect_triangle: TextureRect = %TextureRect_Triangle
@onready var panel_container_border: PanelContainer = %PanelContainer_Border


func _ready():
	pass


func set_base_emote_info(urn: String):
	var picture = load("res://assets/avatar/default_emotes_thumbnails/%s.png" % urn)
	var current_size = texture_rect_preview.size
	texture_rect_preview.texture = picture
	texture_rect_preview.size = current_size

	texture_rect_background.texture = base_thumbnail

	var texture_path: String = "res://assets/ui/EmotesIcon.png"
	if ResourceLoader.exists(texture_path):
		var texture = load(texture_path)
		if texture != null:
			texture_rect_category.texture = texture
	texture_rect_triangle.self_modulate = Wearables.RarityColor.BASE
	panel_container_border.self_modulate = Wearables.RarityColor.BASE
	texture_progress_bar_loading.hide()


func async_set_item(item: DclItemEntityDefinition):
	item_data = item
	_update_category_icon(item)
	_update_rarity_background(item)
	_async_update_thumbnail(item)
	texture_progress_bar_loading.hide()


func _update_category_icon(item: DclItemEntityDefinition):
	var texture_path: String = "res://assets/ui/EmotesIcon.png"
	if !item.is_emote():
		texture_path = ("res://assets/ui/wearable_categories/" + item.get_category() + "-icon.svg")

	if ResourceLoader.exists(texture_path):
		var texture = load(texture_path)
		if texture != null:
			texture_rect_category.texture = texture


func _async_update_thumbnail(item: DclItemEntityDefinition):
	var dcl_content_mapping = item.get_content_mapping()
	var item_thumbnail: String = item.get_thumbnail()
	var thumbnail_hash = dcl_content_mapping.get_hash(item_thumbnail)

	if not thumbnail_hash.is_empty():
		var promise: Promise = Global.content_provider.fetch_texture(
			item_thumbnail, dcl_content_mapping
		)
		var res = await PromiseUtils.async_awaiter(promise)
		if res is PromiseError:
			printerr("Fetch texture error on ", item_thumbnail, ": ", res.get_error())
		else:
			var current_size = texture_rect_preview.size
			texture_rect_preview.texture = res.texture
			texture_rect_preview.size = current_size


func _update_rarity_background(item: DclItemEntityDefinition):
	match item.get_rarity():
		"common":
			texture_rect_background.texture = common_thumbnail
			texture_rect_triangle.self_modulate = Wearables.RarityColor.COMMON
			panel_container_border.self_modulate = Wearables.RarityColor.COMMON
		"uncommon":
			texture_rect_background.texture = uncommon_thumbnail
			texture_rect_triangle.self_modulate = Wearables.RarityColor.UNCOMMON
			panel_container_border.self_modulate = Wearables.RarityColor.UNCOMMON
		"rare":
			texture_rect_background.texture = rare_thumbnail
			texture_rect_triangle.self_modulate = Wearables.RarityColor.RARE
			panel_container_border.self_modulate = Wearables.RarityColor.RARE
		"epic":
			texture_rect_background.texture = epic_thumbnail
			texture_rect_triangle.self_modulate = Wearables.RarityColor.EPIC
			panel_container_border.self_modulate = Wearables.RarityColor.EPIC
		"legendary":
			texture_rect_background.texture = legendary_thumbnail
			texture_rect_triangle.self_modulate = Wearables.RarityColor.LEGENDARY
			panel_container_border.self_modulate = Wearables.RarityColor.LEGENDARY
		"mythic":
			texture_rect_background.texture = mythic_thumbnail
			texture_rect_triangle.self_modulate = Wearables.RarityColor.MYTHIC
			panel_container_border.self_modulate = Wearables.RarityColor.MYTHIC
		"unique":
			texture_rect_background.texture = unique_thumbnail
			texture_rect_triangle.self_modulate = Wearables.RarityColor.UNIQUE
			panel_container_border.self_modulate = Wearables.RarityColor.UNIQUE
		"exotic":
			# TO DO: Get background to Exotic Rarity
			texture_rect_background.texture = unique_thumbnail
			texture_rect_triangle.self_modulate = Wearables.RarityColor.EXOTIC
			panel_container_border.self_modulate = Wearables.RarityColor.EXOTIC
		_:
			texture_rect_background.texture = base_thumbnail
			texture_rect_triangle.self_modulate = Wearables.RarityColor.BASE
			panel_container_border.self_modulate = Wearables.RarityColor.BASE
