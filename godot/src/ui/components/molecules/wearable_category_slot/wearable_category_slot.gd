@tool
## The Wearables subcategory row: shows the equipped item's thumbnail (or an empty-slot
## placeholder) and still acts as the subcategory filter toggle. Renders the 8 Figma card
## states (Empty/Equipped x Default/Pressed/Selected/Disable) via _update_card_visual().
class_name WearableCategorySlot
extends WearableCategoryButton

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
var style_empty_default = preload(
	"res://src/ui/components/molecules/wearable_category_slot/style_subcategory_slot_empty_default.tres"
)
var style_empty_pressed = preload(
	"res://src/ui/components/molecules/wearable_category_slot/style_subcategory_slot_empty_pressed.tres"
)
var style_empty_selected = preload(
	"res://src/ui/components/molecules/wearable_category_slot/style_subcategory_slot_empty_selected.tres"
)
var style_equipped = preload(
	"res://src/ui/components/molecules/wearable_category_slot/style_subcategory_slot_equipped_default.tres"
)
# The pressed/selected outline is a ring offset outward from the card (2px gap, 3px border) —
# shared by the Empty and Equipped cards alike, so it lives on its own overlay node rather
# than being baked into each card fill style.
var style_outline_pressed = preload(
	"res://src/ui/components/molecules/wearable_category_slot/style_subcategory_slot_outline_pressed.tres"
)
var style_outline_selected = preload(
	"res://src/ui/components/molecules/wearable_category_slot/style_subcategory_slot_outline_selected.tres"
)

var is_equipped := false
var _was_pressing := false

@onready var panel_container_card = %PanelContainer_Card
@onready var panel_container_outline = %PanelContainer_Outline
@onready var marquee_label_category: ScrollContainer = %MarqueeLabel_Category
@onready var texture_rect_equipped_background = %TextureRect_EquippedBackground
@onready var texture_rect_equipped_thumbnail = %TextureRect_EquippedThumbnail
@onready var control_empty_icon = %Control_EmptyIcon


func _apply_category_text(display_text: String) -> void:
	# filter_category's setter calls this before @onready vars exist (property application
	# runs ahead of _ready) — fall back to the native Button text until the slot nodes exist;
	# _ready() re-runs this once they do.
	if not is_node_ready():
		text = display_text
		return
	text = ""
	marquee_label_category.set_text(display_text)
	_async_update_category_label_layout()


## The slot's own width is 0 while its main-category tab isn't the selected one (it's simply
## hidden — see backpack.gd's _update_visible_categories), so a check right after _apply_category_
## text can't tell yet whether the name will fit. Wait a frame for a size the hidden case can't
## give us; _on_visibility_changed below re-checks for real once the tab is actually selected.
func _async_update_category_label_layout() -> void:
	await get_tree().process_frame
	_update_category_label_layout()


## es/pt_BR category names (e.g. "Sobre la cabeza" for "Top Head") can run well past what the
## slot fits — marquee-scroll it instead of letting it clip. For the common case where it does
## fit, MarqueeLabel's own idle position is left-aligned (built for a wider name field in
## Profile); center it here instead so a short name still looks like the single centered word
## the old plain Label rendered.
func _update_category_label_layout() -> void:
	if not is_visible_in_tree():
		return
	marquee_label_category.stop_marquee_effect()
	if marquee_label_category.is_text_overflowing():
		marquee_label_category.start_marquee_effect()
	else:
		var label: Label = marquee_label_category.label
		label.position.x = maxf((marquee_label_category.size.x - label.size.x) / 2.0, 0.0)


## Re-measure when this slot's own tab is selected (backpack.gd flips .visible directly, one
## event per switch) — not on marquee_label_category's `resized`: writing label.position.x
## inside that handler re-triggers the ScrollContainer's own layout pass, which re-fires
## resized and re-enters this same logic with a stale mid-pass size, flapping between the
## centered offset and a spuriously "overflowing" marquee start.
func _on_visibility_changed() -> void:
	if is_visible_in_tree():
		_async_update_category_label_layout()


func _ready() -> void:
	super._ready()
	flat = true
	visibility_changed.connect(_on_visibility_changed)
	_update_card_visual()


## CustomTouchButton only learns a touch was a drag (vs. a tap) at release — polling it
## per-frame during the hold left the card stuck "pressed" for the whole gesture whenever the
## subcategory row was scrolled, since is_dragging never goes true until the finger lifts.
## Watch the drag ourselves instead and drop the pressed look the moment it crosses the
## threshold, not after the fact.
func _on_gui_input(event: InputEvent) -> void:
	super._on_gui_input(event)
	if event is InputEventScreenTouch:
		if event.pressed:
			_was_pressing = true
			_update_card_visual()
		elif _was_pressing:
			_was_pressing = false
			_update_card_visual()
	elif event is InputEventScreenDrag and _was_pressing:
		if press_position.distance_to(event.position) > CustomTouchButton.DRAG_THRESHOLD:
			_was_pressing = false
			_update_card_visual()


func _on_toggled(_button_pressed: bool) -> void:
	super._on_toggled(_button_pressed)
	_update_card_visual()


## backpack.gd drives the default-selected sub-category via set_pressed_no_signal (no
## `toggled` signal, by design — Godot forbids overriding that native BaseButton method
## directly) — call this instead so the card style doesn't go stale.
func set_slot_pressed_no_signal(p_pressed: bool) -> void:
	set_pressed_no_signal(p_pressed)
	_update_card_visual()


## Shows the equipped item's thumbnail (with its rarity background) on the slot, replacing
## the empty-slot placeholder.
func set_equipped_texture(thumbnail: Texture2D, rarity: String) -> void:
	is_equipped = true
	texture_rect_equipped_background.texture = _rarity_background(rarity)
	texture_rect_equipped_thumbnail.texture = thumbnail
	texture_rect_equipped_background.show()
	texture_rect_equipped_thumbnail.show()
	control_empty_icon.hide()
	_update_card_visual()


## Reverts the slot to the empty-category placeholder.
func clear_equipped_texture() -> void:
	is_equipped = false
	texture_rect_equipped_background.hide()
	texture_rect_equipped_thumbnail.hide()
	control_empty_icon.show()
	_update_card_visual()


## Resolves which of the 8 Figma card states (Empty/Equipped x Default/Pressed/Selected/
## Disable) currently applies. Disabled wins over the live touch-down state, which wins over
## the persisted filter-selected state.
func _resolve_card_state() -> String:
	var prefix := "equipped" if is_equipped else "empty"
	if disabled:
		return prefix + "_disabled"
	if _was_pressing:
		return prefix + "_pressed"
	if button_pressed:
		return prefix + "_selected"
	return prefix + "_default"


func _update_card_visual() -> void:
	if not is_node_ready():
		return
	var state := _resolve_card_state()
	var fill: StyleBox = style_equipped if is_equipped else style_empty_default
	match state:
		"empty_pressed":
			fill = style_empty_pressed
		"empty_selected":
			fill = style_empty_selected
	panel_container_card.add_theme_stylebox_override("panel", fill)
	# The outline ring only appears for the live touch-down and the persisted-selected
	# states — Default and Disable render with no outline at all, Empty and Equipped alike.
	if state.ends_with("_pressed"):
		panel_container_outline.add_theme_stylebox_override("panel", style_outline_pressed)
		panel_container_outline.show()
	elif state.ends_with("_selected"):
		panel_container_outline.add_theme_stylebox_override("panel", style_outline_selected)
		panel_container_outline.show()
	else:
		panel_container_outline.hide()
	# Empty-Disable is "Default with 60% opacity" on the whole card, Control_EmptyIcon included.
	panel_container_card.modulate = Color(1, 1, 1, 0.6 if state == "empty_disabled" else 1.0)
	var thumbnail_alpha := 0.8 if state == "equipped_disabled" else 1.0
	texture_rect_equipped_background.modulate = Color(1, 1, 1, thumbnail_alpha)
	texture_rect_equipped_thumbnail.modulate = Color(1, 1, 1, thumbnail_alpha)


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
