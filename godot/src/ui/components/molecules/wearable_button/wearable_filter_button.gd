class_name WearableFilterButton
extends CustomTouchButton

signal filter_type(type: String)
signal clear_filter

enum WearableCategoryEnum {
	ALL,
	BODY,
	HEAD,
	CHEST,
	LEGS,
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
	SKIN,
	BODY_SHAPE,
	FACE,
	CLOTHING,
	EXTRAS,
	ALL_EXTRAS
}

@export var filter_category: WearableCategoryEnum:
	set(new_value):
		#_update_category_icon()
		_update_category_text()
		filter_category = new_value
@export var uppercase := false


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
	# type_to_text() returns a translation key, so uppercasing happens on the *translation*.
	# Kept because `uppercase` is a per-instance styling flag, but note it is a display transform
	# on translated text: a locale that should not be shouted needs a styling fix, not a to_upper.
	var category_key = type_to_text(filter_category)
	if category_key != "":
		var category_text := tr(category_key)
		if uppercase:
			self.text = category_text.to_upper()
		else:
			self.text = category_text
	else:
		self.text = tr("WEARABLE_BUTTON_UNKNOWN")


func _ready():
	#_update_category_icon()
	_update_category_text()


func get_category_name():
	return type_to_category(filter_category)


func type_to_category(category_enum: WearableCategoryEnum) -> String:
	var result: String = ""
	match category_enum:
		WearableCategoryEnum.ALL:
			result = Wearables.Categories.ALL
		WearableCategoryEnum.BODY:
			result = Wearables.Categories.BODY
		WearableCategoryEnum.HEAD:
			result = Wearables.Categories.HEAD
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
		WearableCategoryEnum.BODY_SHAPE:
			result = Wearables.Categories.BODY_SHAPE

		WearableCategoryEnum.FACE:
			result = Wearables.Categories.FACE
		WearableCategoryEnum.CLOTHING:
			result = Wearables.Categories.CLOTHING
		WearableCategoryEnum.EXTRAS:
			result = Wearables.Categories.EXTRAS
		WearableCategoryEnum.ALL_EXTRAS:
			result = Wearables.Categories.ALL_EXTRAS

	return result


# i18n-keys: WEARABLE_CATEGORY_*
func type_to_text(category_enum: WearableCategoryEnum) -> String:
	var text := "WEARABLE_BUTTON_UNKNOWN"
	match category_enum:
		WearableCategoryEnum.ALL:
			text = "WEARABLE_CATEGORY_ALL"
		WearableCategoryEnum.BODY:
			text = "WEARABLE_CATEGORY_BODY"
		WearableCategoryEnum.HEAD:
			text = "WEARABLE_CATEGORY_HEAD"
		#WearableCategoryEnum.CHEST:
		#	text = "Chest"
		#WearableCategoryEnum.LEGS:
		#	text = "Legs"
		WearableCategoryEnum.HAIR:
			text = "WEARABLE_CATEGORY_HAIR"
		WearableCategoryEnum.EYEBROWS:
			text = "WEARABLE_CATEGORY_EYEBROWS"
		WearableCategoryEnum.EYES:
			text = "WEARABLE_CATEGORY_EYES"
		WearableCategoryEnum.MOUTH:
			text = "WEARABLE_CATEGORY_MOUTH"
		WearableCategoryEnum.FACIAL_HAIR:
			text = "WEARABLE_CATEGORY_FACIAL_HAIR"
		WearableCategoryEnum.UPPER_BODY:
			text = "WEARABLE_CATEGORY_CHEST"  #"Upper Body"
		WearableCategoryEnum.HANDWEAR:
			text = "WEARABLE_CATEGORY_HANDS"
		WearableCategoryEnum.LOWER_BODY:
			text = "WEARABLE_CATEGORY_LEGS"  #"Lower Body"
		WearableCategoryEnum.FEET:
			text = "WEARABLE_CATEGORY_FEET"  #"Footwear"
		WearableCategoryEnum.HAT:
			text = "WEARABLE_CATEGORY_HATS"
		WearableCategoryEnum.EYEWEAR:
			text = "WEARABLE_CATEGORY_GLASSES"
		WearableCategoryEnum.EARRING:
			text = "WEARABLE_CATEGORY_EARRINGS"
		WearableCategoryEnum.MASK:
			text = "WEARABLE_CATEGORY_MASKS"
		WearableCategoryEnum.TIARA:
			text = "WEARABLE_CATEGORY_TIARAS"
		WearableCategoryEnum.TOP_HEAD:
			text = "WEARABLE_CATEGORY_TOP_HEAD"
		WearableCategoryEnum.HELMET:
			text = "WEARABLE_CATEGORY_HELMETS"
		WearableCategoryEnum.SKIN:
			text = "WEARABLE_CATEGORY_SKIN"
		WearableCategoryEnum.BODY_SHAPE:
			text = "WEARABLE_CATEGORY_SHAPE"

		WearableCategoryEnum.FACE:
			text = "WEARABLE_CATEGORY_FACE"
		WearableCategoryEnum.CLOTHING:
			text = "WEARABLE_CATEGORY_CLOTHING"
		WearableCategoryEnum.EXTRAS:
			text = "WEARABLE_CATEGORY_EXTRAS"
		WearableCategoryEnum.ALL_EXTRAS:
			text = "WEARABLE_CATEGORY_ALL"
	return text


func _on_toggled(_button_pressed):
	if _button_pressed:
		filter_type.emit(type_to_category(filter_category))
	else:
		clear_filter.emit()


## The label is assigned from GDScript, so it does not re-translate itself the way a scene `text`
## property does. These buttons persist across a locale change rather than being rebuilt on open.
func _notification(what: int) -> void:
	if what == NOTIFICATION_TRANSLATION_CHANGED and is_node_ready():
		_update_category_text()
