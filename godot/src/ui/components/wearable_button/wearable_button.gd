@tool

extends Button

enum WearableCategoryEnum {
	BODY,
	HAIR,
	EYEBROWS,
	EYES,
	MOUTH,
	FACIAL_HAIR,
	UPPER_BODY,
	HANDWEAR,
	LOWER_BODY,
	FEET,
	HAT,
	EYEWEAR,
	EARRING,
	MASK,
	TIARA,
	TOP_HEAD,
	HELMET,
	SKIN
}

@onready var panel_container = $HBoxContainer/Control2/PanelContainer
@export var filter_category: WearableCategoryEnum:
	set(new_value):
		_update_category_icon()
		filter_category = new_value

signal filter_type(type: String)
signal clear_filter

@onready var texture_rect_icon = $HBoxContainer/Control/TextureRect_Icon
@onready var texture_rect_preview = $HBoxContainer/Control2/Panel/TextureRect_Preview

var thumbnail_hash: String


func _update_category_icon():
	if is_instance_valid(texture_rect_icon):
		var texture_path = (
			"res://assets/wearable_categories/"
			+ type_to_category(self.filter_category)
			+ "_icon.png"
		)
		if FileAccess.file_exists(texture_path):
			var texture = load(texture_path)
			if texture != null:
				texture_rect_icon.texture = texture


func _ready():
	panel_container.hide()
	_update_category_icon()


func _on_mouse_entered():
	panel_container.show()


func _on_mouse_exited():
	panel_container.hide()


func type_to_category(category_enum: WearableCategoryEnum) -> String:
	match category_enum:
		WearableCategoryEnum.BODY:
			return Wearables.Categories.BODY_SHAPE
		WearableCategoryEnum.HAIR:
			return Wearables.Categories.HAIR
		WearableCategoryEnum.EYEBROWS:
			return Wearables.Categories.EYEBROWS
		WearableCategoryEnum.EYES:
			return Wearables.Categories.EYES
		WearableCategoryEnum.MOUTH:
			return Wearables.Categories.MOUTH
		WearableCategoryEnum.FACIAL_HAIR:
			return Wearables.Categories.FACIAL_HAIR
		WearableCategoryEnum.UPPER_BODY:
			return Wearables.Categories.UPPER_BODY
		WearableCategoryEnum.HANDWEAR:
			return Wearables.Categories.HANDS_WEAR
		WearableCategoryEnum.LOWER_BODY:
			return Wearables.Categories.LOWER_BODY
		WearableCategoryEnum.FEET:
			return Wearables.Categories.FEET
		WearableCategoryEnum.HAT:
			return Wearables.Categories.HAT
		WearableCategoryEnum.EYEWEAR:
			return Wearables.Categories.EYEWEAR
		WearableCategoryEnum.EARRING:
			return Wearables.Categories.EARRING
		WearableCategoryEnum.MASK:
			return Wearables.Categories.MASK
		WearableCategoryEnum.TIARA:
			return Wearables.Categories.TIARA
		WearableCategoryEnum.TOP_HEAD:
			return Wearables.Categories.TOP_HEAD
		WearableCategoryEnum.HELMET:
			return Wearables.Categories.HELMET
		WearableCategoryEnum.SKIN:
			return Wearables.Categories.SKIN

	return ""


func _on_toggled(_button_pressed):
	if _button_pressed:
		emit_signal("filter_type", type_to_category(filter_category))
		flat = false
	else:
		emit_signal("clear_filter")
		flat = true


func set_wearable(wearable: Dictionary):
	var wearable_category = Wearables.get_category(wearable)
	if wearable_category != type_to_category(filter_category):
		return

	var wearable_thumbnail: String = wearable.get("metadata", {}).get("thumbnail", "")
	var new_thumbnail_hash = wearable.get("content", {}).get(wearable_thumbnail, "")

	if new_thumbnail_hash == thumbnail_hash:
		return

	thumbnail_hash = new_thumbnail_hash
	# TODO: loading?

	if not thumbnail_hash.is_empty():
		var content_mapping: Dictionary = {
			"content": wearable.get("content", {}),
			"base_url": "https://peer.decentraland.org/content/contents/"
		}
		var promise = Global.content_manager.fetch_texture(wearable_thumbnail, content_mapping)
		var res = await promise.co_awaiter()
		if res is PromiseError:
			printerr("Fetch texture error on ", wearable_thumbnail)
		else:
			texture_rect_preview.texture = res
