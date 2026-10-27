@tool
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
		_update_category_text()
		filter_category = new_value
@export var uppercase := false
# The main-category tabs (ALL/BODY/FACE/CLOTHING/EXTRAS) instance this same scene as the
# sub-category slots — keep their original text-pill look by turning the slot visuals off.
@export var is_category_slot := true

# Same rarity-background set as WearableItem.async_set_wearable — kept in sync with it.
var base_thumbnail = preload("res://assets/ui/BaseThumbnail.png")
var common_thumbnail = preload("res://assets/ui/CommonThumbnail.png")
var uncommon_thumbnail = preload("res://assets/ui/UncommonThumbnail.png")
var rare_thumbnail = preload("res://assets/ui/RareThumbnail.png")
var epic_thumbnail = preload("res://assets/ui/EpicThumbnail.png")
var exotic_thumbnail = preload("res://assets/ui/ExoticThumbnail.png")
var mythic_thumbnail = preload("res://assets/ui/MythicThumbnail.png")
var legendary_thumbnail = preload("res://assets/ui/LegendaryThumbnail.png")
var unique_thumbnail = preload("res://assets/ui/UniqueThumbnail.png")
var style_card_equipped = preload(
	"res://src/ui/components/molecules/wearable_button/style_subcategory_slot_equipped.tres"
)
var style_card_empty = preload(
	"res://src/ui/components/molecules/wearable_button/style_subcategory_slot_normal.tres"
)

@onready var panel_container_card = %PanelContainer_Card
@onready var panel_container_selected = %PanelContainer_Selected
@onready var label_category = %Label_Category
@onready var texture_rect_equipped_background = %TextureRect_EquippedBackground
@onready var texture_rect_equipped_thumbnail = %TextureRect_EquippedThumbnail
@onready var control_empty_icon = %Control_EmptyIcon


func _update_category_text():
	# type_to_text() returns a translation key, so uppercasing happens on the *translation*.
	# Kept because `uppercase` is a per-instance styling flag, but note it is a display transform
	# on translated text: a locale that should not be shouted needs a styling fix, not a to_upper.
	# type_to_text() always returns a key — it defaults to WEARABLE_BUTTON_UNKNOWN — so
	# there is no empty case to guard. The old `!= ""` check was already unreachable.
	var category_key := type_to_text(filter_category)
	var display_text := category_key.upper() if uppercase else category_key.text()
	# filter_category's setter calls this before @onready vars exist (property application
	# runs ahead of _ready) — fall back to the native Button text until the slot nodes exist;
	# _ready() re-runs this once they do.
	if not is_category_slot or not is_node_ready():
		self.text = display_text
		return
	self.text = ""
	label_category.text = display_text


func _ready():
	_update_category_text()
	if is_category_slot:
		# No button chrome of our own — only the card's empty/equipped look and the
		# selected frame should ever change what this renders.
		flat = true
		panel_container_selected.visible = button_pressed
	else:
		# Don't rely on the scene's own `flat` default here — it's shared with the slot
		# nodes above and must stay off for the main-category tabs' native pressed/underline
		# style to render.
		flat = false
		panel_container_card.hide()
		panel_container_selected.hide()
		label_category.hide()
		custom_minimum_size = Vector2.ZERO


func get_category_name():
	return type_to_category(filter_category)


## backpack.gd drives the default-selected sub-category via set_pressed_no_signal (no
## `toggled` signal, by design — Godot forbids overriding that native BaseButton method
## directly) — call this instead so the selected frame doesn't go stale.
func set_slot_pressed_no_signal(p_pressed: bool) -> void:
	set_pressed_no_signal(p_pressed)
	if is_category_slot and is_node_ready():
		panel_container_selected.visible = p_pressed


## Shows the equipped item's thumbnail (with its rarity background) on the slot, replacing
## the empty-slot placeholder.
func set_equipped_texture(thumbnail: Texture2D, rarity: String) -> void:
	texture_rect_equipped_background.texture = _rarity_background(rarity)
	texture_rect_equipped_thumbnail.texture = thumbnail
	texture_rect_equipped_background.show()
	texture_rect_equipped_thumbnail.show()
	control_empty_icon.hide()
	panel_container_card.add_theme_stylebox_override("panel", style_card_equipped)


## Reverts the slot to the empty-category placeholder.
func clear_equipped_texture() -> void:
	texture_rect_equipped_background.hide()
	texture_rect_equipped_thumbnail.hide()
	control_empty_icon.show()
	panel_container_card.add_theme_stylebox_override("panel", style_card_empty)


func _rarity_background(rarity: String) -> Texture2D:
	match rarity:
		"common":
			return common_thumbnail
		"uncommon":
			return uncommon_thumbnail
		"rare":
			return rare_thumbnail
		"epic":
			return epic_thumbnail
		"legendary":
			return legendary_thumbnail
		"exotic":
			return exotic_thumbnail
		"mythic":
			return mythic_thumbnail
		"unique":
			return unique_thumbnail
		_:
			return base_thumbnail


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


## Returns the catalogue key for a category. Typed, so every branch is visibly a key and
## the checker needs no marker to find them.
func type_to_text(category_enum: WearableCategoryEnum) -> TranslationKey:
	var text := TranslationKey.new("WEARABLE_BUTTON_UNKNOWN")
	match category_enum:
		WearableCategoryEnum.ALL:
			text = TranslationKey.new("WEARABLE_CATEGORY_ALL")
		WearableCategoryEnum.BODY:
			text = TranslationKey.new("WEARABLE_CATEGORY_BODY")
		WearableCategoryEnum.HEAD:
			text = TranslationKey.new("WEARABLE_CATEGORY_HEAD")
		#WearableCategoryEnum.CHEST:
		#	text = "Chest"
		#WearableCategoryEnum.LEGS:
		#	text = "Legs"
		WearableCategoryEnum.HAIR:
			text = TranslationKey.new("WEARABLE_CATEGORY_HAIR")
		WearableCategoryEnum.EYEBROWS:
			text = TranslationKey.new("WEARABLE_CATEGORY_EYEBROWS")
		WearableCategoryEnum.EYES:
			text = TranslationKey.new("WEARABLE_CATEGORY_EYES")
		WearableCategoryEnum.MOUTH:
			text = TranslationKey.new("WEARABLE_CATEGORY_MOUTH")
		WearableCategoryEnum.FACIAL_HAIR:
			text = TranslationKey.new("WEARABLE_CATEGORY_FACIAL_HAIR")
		WearableCategoryEnum.UPPER_BODY:
			text = TranslationKey.new("WEARABLE_CATEGORY_CHEST")  #"Upper Body"
		WearableCategoryEnum.HANDWEAR:
			text = TranslationKey.new("WEARABLE_CATEGORY_HANDS")
		WearableCategoryEnum.LOWER_BODY:
			text = TranslationKey.new("WEARABLE_CATEGORY_LEGS")  #"Lower Body"
		WearableCategoryEnum.FEET:
			text = TranslationKey.new("WEARABLE_CATEGORY_FEET")  #"Footwear"
		WearableCategoryEnum.HAT:
			text = TranslationKey.new("WEARABLE_CATEGORY_HATS")
		WearableCategoryEnum.EYEWEAR:
			text = TranslationKey.new("WEARABLE_CATEGORY_GLASSES")
		WearableCategoryEnum.EARRING:
			text = TranslationKey.new("WEARABLE_CATEGORY_EARRINGS")
		WearableCategoryEnum.MASK:
			text = TranslationKey.new("WEARABLE_CATEGORY_MASKS")
		WearableCategoryEnum.TIARA:
			text = TranslationKey.new("WEARABLE_CATEGORY_TIARAS")
		WearableCategoryEnum.TOP_HEAD:
			text = TranslationKey.new("WEARABLE_CATEGORY_TOP_HEAD")
		WearableCategoryEnum.HELMET:
			text = TranslationKey.new("WEARABLE_CATEGORY_HELMETS")
		WearableCategoryEnum.SKIN:
			text = TranslationKey.new("WEARABLE_CATEGORY_SKIN")
		WearableCategoryEnum.BODY_SHAPE:
			text = TranslationKey.new("WEARABLE_CATEGORY_SHAPE")

		WearableCategoryEnum.FACE:
			text = TranslationKey.new("WEARABLE_CATEGORY_FACE")
		WearableCategoryEnum.CLOTHING:
			text = TranslationKey.new("WEARABLE_CATEGORY_CLOTHING")
		WearableCategoryEnum.EXTRAS:
			text = TranslationKey.new("WEARABLE_CATEGORY_EXTRAS")
		WearableCategoryEnum.ALL_EXTRAS:
			text = TranslationKey.new("WEARABLE_CATEGORY_ALL")
	return text


func _on_toggled(_button_pressed):
	if _button_pressed:
		filter_type.emit(type_to_category(filter_category))
	else:
		clear_filter.emit()
	# A `button_pressed = true` instance override fires `toggled` synchronously while the
	# scene is still being instantiated — before `is_category_slot`'s own override is applied
	# and before @onready vars exist. _ready() does the initial sync instead.
	if is_category_slot and is_node_ready():
		panel_container_selected.visible = _button_pressed


## The label is assigned from GDScript, so it does not re-translate itself the way a scene `text`
## property does. These buttons persist across a locale change rather than being rebuilt on open.
func _notification(what: int) -> void:
	if what == NOTIFICATION_TRANSLATION_CHANGED and is_node_ready():
		_update_category_text()
