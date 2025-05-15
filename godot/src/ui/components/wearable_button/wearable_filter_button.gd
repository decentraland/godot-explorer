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

var press_time: int = 0


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
	var text := "Unknown"
	match category_enum:
		WearableCategoryEnum.ALL:
			text = "All Wearables"
		WearableCategoryEnum.BODY:
			text = "Body Shape"
		WearableCategoryEnum.HAIR:
			text = "Hair"
		WearableCategoryEnum.EYEBROWS:
			text = "Eyebrows"
		WearableCategoryEnum.EYES:
			text = "Eyes"
		WearableCategoryEnum.MOUTH:
			text = "Mouth"
		WearableCategoryEnum.FACIAL_HAIR:
			text = "Facial Hair"
		WearableCategoryEnum.UPPER_BODY:
			text = "Upper Body"
		WearableCategoryEnum.HANDWEAR:
			text = "Gloves"
		WearableCategoryEnum.LOWER_BODY:
			text = "Lower Body"
		WearableCategoryEnum.FEET:
			text = "Footwear"
		WearableCategoryEnum.HAT:
			text = "Hats"
		WearableCategoryEnum.EYEWEAR:
			text = "Glasses"
		WearableCategoryEnum.EARRING:
			text = "Earrings"
		WearableCategoryEnum.MASK:
			text = "Masks"
		WearableCategoryEnum.TIARA:
			text = "Tiaras"
		WearableCategoryEnum.TOP_HEAD:
			text = "Accessories"
		WearableCategoryEnum.HELMET:
			text = "Helmets"
		WearableCategoryEnum.SKIN:
			text = "Skin"
	return text


func _on_toggled(_button_pressed):
	if _button_pressed:
		filter_type.emit(type_to_category(filter_category))
	else:
		clear_filter.emit()


func _on_pressed() -> void:
	prints("Hello world")


func _on_button_down() -> void:
	press_time = Time.get_ticks_msec()


func _on_button_up() -> void:
	var release_time = Time.get_ticks_msec()
	var duration = release_time - press_time
	print("Button was held for %d milliseconds" % duration)
	if duration <= 300:
		button_pressed = !button_pressed
