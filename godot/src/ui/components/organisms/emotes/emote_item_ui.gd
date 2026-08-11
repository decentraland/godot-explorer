@tool
class_name EmoteItemUi
extends BaseButton

signal play_emote(emote_urn: String)
signal select_emote(selected: bool, emote_urn: String)
signal emote_name_ready(emote_name: String)

# Empty-slot background tint for the wheel (issue #2458): design spec #24033B @ 50%.
const EMPTY_SLOT_INNER_COLOR := Color("24033b80")

@export var rarity: String = Wearables.ItemRarity.COMMON:
	set(new_value):
		rarity = new_value
		_is_dirty = true

@export var picture: Texture2D = null:
	set(new_value):
		picture = new_value
		_is_dirty = true

# The default emotes are not urns
@export var emote_urn: String = "wave"
# The display name
@export var emote_name: String = "wave"

# Wheel outline (#2458): one %Outline node recolored per state instead of one node per
# state. Exposed so the 4 state colors can be tweaked from the inspector.
@export_group("Wheel Outline Colors")
@export var outline_color_normal: Color = Color("d3d3d3")
@export var outline_color_pressed: Color = Color("800000")
@export var outline_color_equipped: Color = Color("ff8000")
@export var outline_color_empty: Color = Color("ae8ad2")
@export_group("")

var base_thumbnail = preload("res://assets/ui/BaseThumbnail.png")
var common_thumbnail = preload("res://assets/ui/CommonThumbnail.png")
var uncommon_thumbnail = preload("res://assets/ui/UncommonThumbnail.png")
var rare_thumbnail = preload("res://assets/ui/RareThumbnail.png")
var epic_thumbnail = preload("res://assets/ui/EpicThumbnail.png")
var exotic_thumbnail = preload("res://assets/ui/ExoticThumbnail.png")
var mythic_thumbnail = preload("res://assets/ui/MythicThumbnail.png")
var legendary_thumbnail = preload("res://assets/ui/LegendaryThumbnail.png")
var unique_thumbnail = preload("res://assets/ui/UniqueThumbnail.png")
var empty_thumbnail = preload("res://assets/ui/EmptyThumbnail.png")
var inside = false
var _is_equipped: bool = false
var _is_empty: bool = false
var _is_selected: bool = false
var _is_dirty := false

@onready var control_inner := %Control_Inner
@onready var texture_rect_background: TextureRect = get_node_or_null("%TextureRect_Background")
# Square backpack item drives per-state overlay nodes; the wheel item recolors a single
# %Outline. Both are optional so the shared script fits either scene.
@onready var texture_rect_selected: TextureRect = get_node_or_null("%Pressed")
@onready var texture_rect_selected_bold: TextureRect = get_node_or_null("%Pressed_bold")
@onready var texture_rect_equiped: TextureRect = get_node_or_null("%Equiped")
@onready var texture_rect_equiped_mark: Control = get_node_or_null("%TextureRect_Equiped")
@onready var texture_rect_skeleton: TextureRect = get_node_or_null("%TextureRect_Skeleton")
@onready var texture_rect_picture: TextureRect = %TextureRect_Picture
@onready var texture_rect_add: TextureRect = get_node_or_null("%TextureRect_Add")
@onready var outline_rect: TextureRect = get_node_or_null("%Outline")
@onready var button_equiped: Button = get_node_or_null("%Button_Equiped")
@onready var panel_new_badge: PanelContainer = get_node_or_null("%PanelContainer_NewBadge")
@onready var inner_rect: TextureRect = get_node_or_null("%Inner")
@onready var glow_rect: TextureRect = get_node_or_null("%Glow")


func async_load_from_urn(_emote_urn: String, _index: int = -1):
	_is_empty = false
	emote_urn = _emote_urn

	# Convert short emote IDs to full URNs for remote fetching
	var fetch_urn = _emote_urn
	if not _emote_urn.begins_with("urn"):
		if Emotes.is_emote_default(_emote_urn):
			fetch_urn = Emotes.get_base_emote_urn(_emote_urn)
		else:
			printerr("Unknown emote ID: ", _emote_urn)
			return

	await WearableRequest.async_fetch_emote(fetch_urn)
	var emote_data := Global.content_provider.get_wearable(fetch_urn)
	if emote_data == null:
		printerr("Failed to get emote data: ", fetch_urn)
		return

	await async_load_from_entity(emote_data)


func async_load_from_entity(emote_data: DclItemEntityDefinition) -> void:
	emote_name = emote_data.get_display_name()
	rarity = emote_data.get_rarity()
	await async_set_texture(emote_data)
	if button_pressed:
		emote_name_ready.emit(emote_name)


func async_set_texture(emote_data: DclItemEntityDefinition) -> void:
	var promise: Promise = Global.content_provider.fetch_texture(
		emote_data.get_thumbnail(), emote_data.get_content_mapping()
	)
	var res = await PromiseUtils.async_awaiter(promise)
	if res is PromiseError:
		printerr("Fetch texture error on ", emote_data.get_thumbnail(), ": ", res.get_error())
	else:
		self.picture = res.texture


func _process(_delta: float) -> void:
	if not _is_dirty:
		return
	if not is_node_ready():
		return
	set_rarity_background()
	_update_picture()
	_update_outline()
	if is_instance_valid(texture_rect_skeleton):
		texture_rect_skeleton.hide()
		texture_rect_background.show()
	_is_dirty = false


func _ready():
	if is_instance_valid(texture_rect_background):
		texture_rect_background.hide()
		texture_rect_skeleton.show()
	# Square overlay starts hidden; the wheel's single outline is colored by _update_outline.
	if is_instance_valid(texture_rect_selected):
		texture_rect_selected.hide()
	_update_equip_ui()
	if not Engine.is_editor_hint():
		set_meta("attenuated_sound", true)
		UiSounds.install_audio_recusirve(self)

		mouse_entered.connect(self._on_mouse_entered)
		mouse_exited.connect(self._on_mouse_exited)

		pressed.connect(self._on_pressed)
		button_down.connect(self._on_button_down)
		button_up.connect(self._on_button_up)
		toggled.connect(self._on_toggled)
	set_rarity_background()
	_update_outline()


func set_rarity_background() -> void:
	if is_instance_valid(texture_rect_background):
		match rarity:
			Wearables.ItemRarity.COMMON:
				texture_rect_background.texture = common_thumbnail
			Wearables.ItemRarity.UNCOMMON:
				texture_rect_background.texture = uncommon_thumbnail
			Wearables.ItemRarity.RARE:
				texture_rect_background.texture = rare_thumbnail
			Wearables.ItemRarity.EPIC:
				texture_rect_background.texture = epic_thumbnail
			Wearables.ItemRarity.LEGENDARY:
				texture_rect_background.texture = legendary_thumbnail
			Wearables.ItemRarity.EXOTIC:
				texture_rect_background.texture = exotic_thumbnail
			Wearables.ItemRarity.MYTHIC:
				texture_rect_background.texture = mythic_thumbnail
			Wearables.ItemRarity.UNIQUE:
				texture_rect_background.texture = unique_thumbnail
			_:
				texture_rect_background.texture = base_thumbnail
		if emote_urn == "":
			texture_rect_background.texture = empty_thumbnail

	if is_instance_valid(inner_rect):
		# The wheel item (has %Inner) dims to the empty-slot color instead of a rarity tint.
		inner_rect.self_modulate = EMPTY_SLOT_INNER_COLOR if _is_empty else _get_rarity_color()
		glow_rect.visible = not _is_empty and rarity != Wearables.ItemRarity.COMMON


# Empty wheel slot (#2458): hide the thumbnail and show the "+" add icon. Only the wheel
# item has %TextureRect_Add; the square backpack item keeps its empty_thumbnail background.
func _update_picture() -> void:
	texture_rect_picture.texture = picture
	if is_instance_valid(texture_rect_add):
		texture_rect_add.visible = _is_empty
		texture_rect_picture.visible = not _is_empty


# Recolor the single wheel outline (#2458) by state instead of toggling per-state nodes.
# Precedence: pressed/selected > equipped > empty > normal. No-op for the square backpack
# item, which has no %Outline and keeps its own per-node visibility.
func _update_outline() -> void:
	if not is_instance_valid(outline_rect):
		return
	var color: Color = outline_color_normal
	if _is_selected:
		color = outline_color_pressed
	elif _is_equipped:
		color = outline_color_equipped
	elif _is_empty:
		color = outline_color_empty
	outline_rect.self_modulate = color


# Selection/press highlight: the square item shows a dedicated overlay node, the wheel
# recolors its single outline. Kept in one place so both stay in sync.
func _set_selected(selected: bool) -> void:
	_is_selected = selected
	if is_instance_valid(texture_rect_selected):
		texture_rect_selected.set_visible(selected)
	_update_outline()


func _get_rarity_color() -> Color:
	match rarity:
		Wearables.ItemRarity.COMMON:
			return Wearables.RarityColor.COMMON
		Wearables.ItemRarity.UNCOMMON:
			return Wearables.RarityColor.UNCOMMON
		Wearables.ItemRarity.RARE:
			return Wearables.RarityColor.RARE
		Wearables.ItemRarity.EPIC:
			return Wearables.RarityColor.EPIC
		Wearables.ItemRarity.LEGENDARY:
			return Wearables.RarityColor.LEGENDARY
		Wearables.ItemRarity.MYTHIC:
			return Wearables.RarityColor.MYTHIC
		Wearables.ItemRarity.UNIQUE:
			return Wearables.RarityColor.UNIQUE
		Wearables.ItemRarity.EXOTIC:
			return Wearables.RarityColor.EXOTIC
		_:
			return Wearables.RarityColor.BASE


# Executed with @tool
func _on_item_rect_changed():
	%TextureRect_Picture.set_rotation(-get_rotation())
	# Keep the empty-slot "+" upright too, matching the thumbnail's counter-rotation.
	var add_node: Node = get_node_or_null("%TextureRect_Add")
	if add_node != null:
		add_node.set_rotation(-get_rotation())


func _on_mouse_exited():
	inside = false
	_set_selected(button_pressed)
	select_emote.emit(false, emote_urn)


func _on_mouse_entered():
	inside = true
	_set_selected(true)
	select_emote.emit(true, emote_urn)


func _on_pressed():
	play_emote.emit(emote_urn)


func _on_toggled(new_toggled: bool):
	_set_selected(new_toggled)
	_update_equip_ui()


func _on_button_down():
	if !toggle_mode:
		_set_selected(true)


func _on_button_up():
	if !toggle_mode:
		_set_selected(inside)


func set_equipped(equipped: bool) -> void:
	_is_equipped = equipped
	_update_equip_ui()


## Shows the "NEW" tag (top-right corner) for a recently-acquired emote (#2300).
func set_new_badge(is_new: bool) -> void:
	# The wheel variant (emote_wheel_item.tscn) has no badge node; only square items do.
	if is_instance_valid(panel_new_badge):
		panel_new_badge.visible = is_new


func set_slot_selected(toggled_on: bool) -> void:
	if is_instance_valid(texture_rect_selected_bold):
		texture_rect_selected_bold.set_visible(toggled_on)
	if is_instance_valid(texture_rect_selected):
		texture_rect_selected.hide()


func _update_equip_ui() -> void:
	if is_instance_valid(texture_rect_equiped):
		texture_rect_equiped.set_visible(_is_equipped)
	_update_outline()
	if not is_instance_valid(button_equiped):
		return
	if not button_pressed:
		texture_rect_equiped_mark.set_visible(_is_equipped)
		button_equiped.hide()
	else:
		texture_rect_equiped.hide()
		texture_rect_equiped_mark.hide()
		button_equiped.show()
		button_equiped.set_pressed_no_signal(_is_equipped)
		button_equiped.text = "UNEQUIP" if _is_equipped else "EQUIP"


func set_empty() -> void:
	emote_urn = ""
	emote_name = ""
	picture = null
	rarity = Wearables.ItemRarity.COMMON
	_is_empty = true
	_is_dirty = true
