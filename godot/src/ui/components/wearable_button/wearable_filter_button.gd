class_name WearableFilterButton
extends Button

signal filter_type(type: String)
signal clear_filter

enum WearableCategoryEnum {
	ALL,
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
		_update_category_text()
		filter_category = new_value


func _update_category_icon():
	var texture_path = (
		"res://assets/ui/wearable_categories/"
		+ type_to_category(self.filter_category)
		+ "-icon.svg"
	)
	if ResourceLoader.exists(texture_path):
		var texture = load(texture_path)
		if texture != null:
			icon = texture
	else:
		printerr("_update_category_icon texture_path not found ", texture_path)


func _update_category_text():
	var category_text = type_to_text(filter_category)
	if category_text != "":
		self.text = category_text
	else:
		self.text = "Unknown"


func _ready():
	_update_category_icon()
	_update_category_text()


func get_category_name():
	return type_to_category(filter_category)


func type_to_category(category_enum: WearableCategoryEnum) -> String:
	var result: String = ""
	match category_enum:
		WearableCategoryEnum.ALL:
			result = Wearables.Categories.ALL
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


func type_to_text(category_enum: WearableCategoryEnum) -> String:
	match category_enum:
		WearableCategoryEnum.ALL:
			return "All Wearables"
		WearableCategoryEnum.BODY:
			return "Body Shape"
		WearableCategoryEnum.HAIR:
			return "Hair"
		WearableCategoryEnum.EYEBROWS:
			return "Eyebrows"
		WearableCategoryEnum.EYES:
			return "Eyes"
		WearableCategoryEnum.MOUTH:
			return "Mouth"
		WearableCategoryEnum.FACIAL_HAIR:
			return "Facial Hair"
		WearableCategoryEnum.UPPER_BODY:
			return "Upper Body"
		WearableCategoryEnum.HANDWEAR:
			return "Gloves"
		WearableCategoryEnum.LOWER_BODY:
			return "Lower Body"
		WearableCategoryEnum.FEET:
			return "Footwear"
		WearableCategoryEnum.HAT:
			return "Hats"
		WearableCategoryEnum.EYEWEAR:
			return "Glasses"
		WearableCategoryEnum.EARRING:
			return "Earrings"
		WearableCategoryEnum.MASK:
			return "Masks"
		WearableCategoryEnum.TIARA:
			return "Tiaras"
		WearableCategoryEnum.TOP_HEAD:
			return "Accessories"
		WearableCategoryEnum.HELMET:
			return "Helmets"
		WearableCategoryEnum.SKIN:
			return "Skin"
		_:
			return "Unknown"
	

func _on_toggled(_button_pressed):
	if _button_pressed:
		filter_type.emit(type_to_category(filter_category))
	else:
		clear_filter.emit()
