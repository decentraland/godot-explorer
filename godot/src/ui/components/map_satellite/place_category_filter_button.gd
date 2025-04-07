class_name PlaceFilterButton
extends Button

signal filter_type(type: String)
signal clear_filter

enum PlaceCategoryEnum {
	ALL,
	FAVORITES,
	ART,
	CRYPTO,
	SOCIAL,
	GAME,
	SHOP,
	EDUCATION,
	MUSIC,
	FASHION,
	CASINO,
	SPORTS,
	BUSINESS
}

@export var filter_category: PlaceCategoryEnum:
	set(new_value):
		_update_category_icon()
		_update_category_text()
		filter_category = new_value


func _update_category_icon():
	var texture_path = (
		"res://assets/ui/place_categories/"
		+ type_to_category(self.filter_category)
		+ "-icon.svg"
	)
	if ResourceLoader.exists(texture_path):
		var texture = load(texture_path)
		if texture != null:
			icon = texture
	else:
		printerr("_update_place_category_icon texture_path not found ", texture_path)


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


func type_to_category(category_enum: PlaceCategoryEnum) -> String:
	var result: String = ""
	match category_enum:
		PlaceCategoryEnum.ALL:
			result = Places.Categories.ALL
		PlaceCategoryEnum.FAVORITES:
			result = Places.Categories.FAVORITES
		PlaceCategoryEnum.ART:
			result = Places.Categories.ART
		PlaceCategoryEnum.CRYPTO:
			result = Places.Categories.CRYPTO
		PlaceCategoryEnum.SOCIAL:
			result = Places.Categories.SOCIAL
		PlaceCategoryEnum.GAME:
			result = Places.Categories.GAME
		PlaceCategoryEnum.SHOP:
			result = Places.Categories.SHOP
		PlaceCategoryEnum.EDUCATION:
			result = Places.Categories.EDUCATION
		PlaceCategoryEnum.MUSIC:
			result = Places.Categories.MUSIC
		PlaceCategoryEnum.FASHION:
			result = Places.Categories.FASHION
		PlaceCategoryEnum.CASINO:
			result = Places.Categories.CASINO
		PlaceCategoryEnum.SPORTS:
			result = Places.Categories.SPORTS
		PlaceCategoryEnum.BUSINESS:
			result = Places.Categories.BUSINESS

	return result


func type_to_text(category_enum: PlaceCategoryEnum) -> String:
	var text := "Unknown"
	match category_enum:
		PlaceCategoryEnum.ALL:
			text = "ALL"
		PlaceCategoryEnum.FAVORITES:
			text = "FAVORITES"
		PlaceCategoryEnum.ART:
			text = "ART"
		PlaceCategoryEnum.CRYPTO:
			text = "CRYPTO"
		PlaceCategoryEnum.SOCIAL:
			text = "SOCIAL"
		PlaceCategoryEnum.GAME:
			text = "GAME"
		PlaceCategoryEnum.SHOP:
			text = "SHOP"
		PlaceCategoryEnum.EDUCATION:
			text = "EDUCATION"
		PlaceCategoryEnum.MUSIC:
			text = "MUSIC"
		PlaceCategoryEnum.FASHION:
			text = "FASHION"
		PlaceCategoryEnum.CASINO:
			text = "CASINO"
		PlaceCategoryEnum.SPORTS:
			text = "SPORTS"
		PlaceCategoryEnum.BUSINESS:
			text = "BUSINESS"
	return text


func _on_toggled(_button_pressed):
	if _button_pressed:
		filter_type.emit(type_to_category(filter_category))
	else:
		clear_filter.emit()
