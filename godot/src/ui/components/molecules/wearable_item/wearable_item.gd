class_name WearableItem
extends CustomTouchButton

signal equip
signal unequip

var base_thumbnail = preload("res://assets/ui/BaseThumbnail.png")
var common_thumbnail = preload("res://assets/ui/CommonThumbnail.png")
var uncommon_thumbnail = preload("res://assets/ui/UncommonThumbnail.png")
var rare_thumbnail = preload("res://assets/ui/RareThumbnail.png")
var epic_thumbnail = preload("res://assets/ui/EpicThumbnail.png")
var exotic_thumbnail = preload("res://assets/ui/ExoticThumbnail.png")
var mythic_thumbnail = preload("res://assets/ui/MythicThumbnail.png")
var legendary_thumbnail = preload("res://assets/ui/LegendaryThumbnail.png")
var unique_thumbnail = preload("res://assets/ui/UniqueThumbnail.png")
var thumbnail_hash: String
var wearable_id: String
var wearable_data: DclItemEntityDefinition
var panel_container_external_orig_rect: Rect2
var was_pressed = false

## Marketplace mode fields
var is_marketplace_item: bool = false
var marketplace_price: int = 0
var marketplace_url: String = ""

@onready var panel_container_external = $PanelContainer_External

@onready var texture_rect_equiped = %TextureRect_Equiped
@onready var texture_rect_category = %TextureRect_Category
@onready var texture_rect_background = %TextureRect_Background
@onready var texture_rect_preview = %TextureRect_Preview
@onready var texture_progress_bar_loading = %TextureRect_Skeleton
@onready var panel_container_price: PanelContainer = %PanelContainer_Price
@onready var label_price: Label = %Label_Price
@onready var button_action: Button = %Button_Action
@onready var panel_new_badge: PanelContainer = %PanelContainer_NewBadge


func _ready():
	UiSounds.install_audio_recusirve(self)
	panel_container_external_orig_rect = panel_container_external.get_rect()
	panel_container_external.hide()
	texture_rect_background.hide()
	texture_progress_bar_loading.show()
	button_action.pressed.connect(_on_action_pressed)


func async_set_wearable(wearable: DclItemEntityDefinition):
	wearable_id = wearable.get_id()
	var dcl_content_mapping = wearable.get_content_mapping()
	var wearable_thumbnail: String = wearable.get_thumbnail()
	thumbnail_hash = dcl_content_mapping.get_hash(wearable_thumbnail)
	_update_category_icon(wearable)
	wearable_data = wearable

	match wearable.get_rarity():
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
		"exotic":
			texture_rect_background.texture = exotic_thumbnail
		"mythic":
			texture_rect_background.texture = mythic_thumbnail
		"unique":
			texture_rect_background.texture = unique_thumbnail
		_:
			texture_rect_background.texture = base_thumbnail

	if not thumbnail_hash.is_empty():
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

	texture_rect_background.show()
	texture_progress_bar_loading.hide()


func effect_toggle():
	if button_pressed:
		if was_pressed == false:
			#var new_size = panel_container_external_orig_rect.size * 0.8
			#var new_position = (panel_container_external_orig_rect.size - new_size) / 2
			#panel_container_external.set_position(new_position)
			#panel_container_external.set_size(new_size)
			panel_container_external.scale = Vector2.ONE * 0.97

			var tween = get_tree().create_tween().set_parallel(true)
			#tween.tween_property(
			#panel_container_external,
			#"position",
			#panel_container_external_orig_rect.position,
			#0.15
			#)
			tween.tween_property(panel_container_external, "scale", Vector2.ONE, 0.15)

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
	if is_marketplace_item:
		_update_marketplace_state(is_equiped)
	else:
		if is_equiped:
			texture_rect_equiped.show()
		else:
			texture_rect_equiped.hide()
	effect_toggle()


## Shows the "NEW" tag (top-right corner) for a recently-acquired wearable (#2300).
func set_new_badge(is_new: bool) -> void:
	panel_new_badge.visible = is_new


func setup_marketplace(price: int, url: String):
	is_marketplace_item = true
	marketplace_price = price
	marketplace_url = url
	texture_rect_equiped.hide()
	panel_container_price.show()
	label_price.text = "%d" % price


func _update_marketplace_state(is_selected: bool):
	if is_selected:
		panel_container_price.hide()
		button_action.show()
		if Iap.get_balance() >= marketplace_price:
			button_action.text = "DETAIL"
		else:
			button_action.text = "GET CREDITS"
	else:
		panel_container_price.show()
		button_action.hide()


func _on_action_pressed():
	if not is_marketplace_item:
		return
	if Iap.get_balance() >= marketplace_price and not marketplace_url.is_empty():
		MarketplaceTracker.open_and_track(marketplace_url)
	else:
		Global.open_credits.emit()


func _update_category_icon(wearable: DclItemEntityDefinition):
	var texture_path = (
		"res://assets/ui/wearable_categories/" + wearable.get_category() + "-icon.svg"
	)
	if ResourceLoader.exists(texture_path):
		var texture = load(texture_path)
		if texture != null:
			texture_rect_category.texture = texture
