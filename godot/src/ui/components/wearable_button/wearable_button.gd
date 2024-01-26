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

func _update_category_icon():
	var texture_path = (
		"res://assets/wearable_categories/"
		+ type_to_category(self.filter_category)
		+ "-icon.svg"
	)
	if FileAccess.file_exists(texture_path):
		var texture = load(texture_path)
		if texture != null:
			icon = texture

func _ready():
	_update_category_icon()
	text = type_to_category(self.filter_category).to_upper().replace("_", " ")

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
	else:
		emit_signal("clear_filter")
