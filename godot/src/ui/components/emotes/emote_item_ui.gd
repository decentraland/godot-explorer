@tool
class_name EmoteItemUi
extends BaseButton

signal play_emote(emote_urn: String)
signal select_emote(selected: bool, emote_urn: String)

@export var rarity: String = Wearables.ItemRarity.COMMON:
	set(new_value):
		rarity = new_value
		%Glow.set_visible(rarity != Wearables.ItemRarity.COMMON)
		var color = Color("#ECEBED")
		match rarity:
			Wearables.ItemRarity.COMMON:
				color = Color("#ECEBED")
			Wearables.ItemRarity.UNCOMMON:
				color = Color("#FF8362")
			Wearables.ItemRarity.RARE:
				color = Color("#34CE76")
			Wearables.ItemRarity.EPIC:
				color = Color("#599CFF")
			Wearables.ItemRarity.LEGENDARY:
				color = Color("#B262FF")
			Wearables.ItemRarity.MYTHIC:
				color = Color("#FF63E1")
			Wearables.ItemRarity.UNIQUE:
				color = Color("#FFB626")
		%Inner.self_modulate = color

@export var picture: Texture2D = null:
	set(new_value):
		%TextureRect_Picture.texture = new_value
		picture = new_value

# The default emotes are not urns
@export var emote_urn: String = "wave"
# The display name
@export var emote_name: String = "wave"

var inside = false

@onready var control_inner = %Control_Inner

@onready var texture_rect_selected = %Selected
@onready var texture_rect_pressed = %Pressed


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


func _ready():
	if not Engine.is_editor_hint():
		set_meta("attenuated_sound", true)
		UiSounds.install_audio_recusirve(self)

		mouse_entered.connect(self._on_mouse_entered)
		mouse_exited.connect(self._on_mouse_exited)

		pressed.connect(self._on_pressed)
		button_down.connect(self._on_button_down)
		button_up.connect(self._on_button_up)
		toggled.connect(self._on_toggled)


# Executed with @tool
func _on_item_rect_changed():
	%TextureRect_Picture.set_rotation(-get_rotation())


func _on_mouse_exited():
	texture_rect_selected.hide()
	inside = false
	select_emote.emit(false, emote_urn)


func _on_mouse_entered():
	texture_rect_selected.show()
	inside = true
	select_emote.emit(true, emote_urn)


func _on_pressed():
	play_emote.emit(emote_urn)


func _on_toggled(new_toggled: bool):
	texture_rect_pressed.set_visible(new_toggled)


func _on_button_down():
	if !toggle_mode:
		texture_rect_pressed.set_visible(true)


func _on_button_up():
	if !toggle_mode:
		texture_rect_pressed.set_visible(false)
