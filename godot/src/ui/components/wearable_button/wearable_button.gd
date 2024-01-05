@tool

extends Button

signal filter_type(type: String)
signal clear_filter

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

@export var filter_category: WearableCategoryEnum:
	set(new_value):
		_update_category_icon()
		filter_category = new_value

var thumbnail_hash: String

@onready var panel_container = $HBoxContainer/Control2/PanelContainer

@onready var texture_rect_icon = $HBoxContainer/Control/TextureRect_Icon
@onready var texture_rect_preview = $HBoxContainer/Control2/Panel/TextureRect_Preview


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
	var result: String = ""
	match category_enum:
		WearableCategoryEnum.BODY:
			result = Wearables.Categories.BODY_SHAPE
		WearableCategoryEnum.HAIR:
			result = Wearables.Categories.HAIR
		WearableCategoryEnum.EYEBROWS:
			result = Wearables.Categories.EYEBROWS
		WearableCategoryEnum.EYES:
			result = Wearables.Categories.EYES
		WearableCategoryEnum.MOUTH:
			result = Wearables.Categories.MOUTH
		WearableCategoryEnum.FACIAL_HAIR:
			result = Wearables.Categories.FACIAL_HAIR
		WearableCategoryEnum.UPPER_BODY:
			result = Wearables.Categories.UPPER_BODY
		WearableCategoryEnum.HANDWEAR:
			result = Wearables.Categories.HANDS_WEAR
		WearableCategoryEnum.LOWER_BODY:
			result = Wearables.Categories.LOWER_BODY
		WearableCategoryEnum.FEET:
			result = Wearables.Categories.FEET
		WearableCategoryEnum.HAT:
			result = Wearables.Categories.HAT
		WearableCategoryEnum.EYEWEAR:
			result = Wearables.Categories.EYEWEAR
		WearableCategoryEnum.EARRING:
			result = Wearables.Categories.EARRING
		WearableCategoryEnum.MASK:
			result = Wearables.Categories.MASK
		WearableCategoryEnum.TIARA:
			result = Wearables.Categories.TIARA
		WearableCategoryEnum.TOP_HEAD:
			result = Wearables.Categories.TOP_HEAD
		WearableCategoryEnum.HELMET:
			result = Wearables.Categories.HELMET
		WearableCategoryEnum.SKIN:
			result = Wearables.Categories.SKIN

	return result


func _on_toggled(_button_pressed):
	if _button_pressed:
		emit_signal("filter_type", type_to_category(filter_category))
		flat = false
	else:
		emit_signal("clear_filter")
		flat = true


func async_set_wearable(wearable: Dictionary):
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
		var dcl_content_mapping = DclContentMappingAndUrl.new()
		dcl_content_mapping.initialize(
			"https://peer.decentraland.org/content/contents/", wearable.get("content", {})
		)
		var promise = Global.content_manager.fetch_texture(wearable_thumbnail, dcl_content_mapping)
		var res = await PromiseUtils.async_awaiter(promise)
		if res is PromiseError:
			printerr("Fetch texture error on ", wearable_thumbnail)
		else:
			texture_rect_preview.texture = res
