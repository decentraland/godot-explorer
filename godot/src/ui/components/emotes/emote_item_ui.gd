@tool
class_name EmoteItemUi
extends BaseButton

signal play_emote(emote_urn: String)
signal select_emote(selected: bool, emote_urn: String)

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
var _is_dirty := false

@onready var control_inner := %Control_Inner
@onready var texture_rect_background := %TextureRect_Background
@onready var texture_rect_selected := %Pressed
@onready var texture_rect_selected_bold := %Pressed_bold
@onready var texture_rect_equiped := %Equiped
@onready var texture_rect_equiped_mark := %TextureRect_Equiped
@onready var texture_rect_skeleton: TextureRect = %TextureRect_Skeleton
@onready var texture_rect_picture: TextureRect = %TextureRect_Picture


func async_load_from_urn(_emote_urn: String, _index: int = -1):
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
	texture_rect_picture.texture = picture
	texture_rect_skeleton.hide()
	texture_rect_background.show()
	_is_dirty = false


func _ready():
	texture_rect_background.hide()
	texture_rect_skeleton.show()
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


func set_rarity_background() -> void:
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


# Executed with @tool
func _on_item_rect_changed():
	%TextureRect_Picture.set_rotation(-get_rotation())


func _on_mouse_exited():
	texture_rect_selected.set_visible(button_pressed)
	inside = false
	select_emote.emit(false, emote_urn)


func _on_mouse_entered():
	texture_rect_selected.show()
	inside = true
	select_emote.emit(true, emote_urn)


func _on_pressed():
	play_emote.emit(emote_urn)


func _on_toggled(new_toggled: bool):
	texture_rect_selected.set_visible(new_toggled)


func _on_button_down():
	if !toggle_mode:
		texture_rect_selected.set_visible(true)


func _on_button_up():
	if !toggle_mode:
		texture_rect_selected.set_visible(inside)


func set_equipped(equipped: bool) -> void:
	_is_equipped = equipped
	texture_rect_equiped.set_visible(_is_equipped)
	texture_rect_equiped_mark.set_visible(_is_equipped)


func set_slot_selected(toggled_on: bool) -> void:
	texture_rect_selected_bold.set_visible(toggled_on)
	texture_rect_selected.hide()


func set_empty() -> void:
	emote_urn = ""
	emote_name = ""
	picture = null
	rarity = Wearables.ItemRarity.COMMON
	_is_dirty = true
