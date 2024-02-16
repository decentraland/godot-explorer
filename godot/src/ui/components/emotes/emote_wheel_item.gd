@tool
class_name EmoteWheelItem
extends Control

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

@export var number: int = 0:
	set(new_value):
		%Label_Number.text = str(new_value)
		number = new_value

@export var picture: Texture2D = null:
	set(new_value):
		%TextureRect_Picture.texture = new_value
		picture = new_value

# The default emotes are not urns
@export var emote_urn: String = "wave"
# The display name
@export var emote_name: String = "wave"

var pressed = false
var inside = false

@onready var control_inner = %Control_Inner

@onready var texture_rect_selected = %Selected
@onready var texture_rect_pressed = %Pressed
@onready var label_number = %Label_Number


func async_load_from_urn(_emote_urn: String, index: int):
	emote_urn = _emote_urn
	number = index

	if Emotes.is_emote_default(emote_urn):
		emote_name = Emotes.DEFAULT_EMOTE_NAMES[emote_urn]
		rarity = Wearables.ItemRarity.COMMON
		picture = load(
			"res://assets/avatar/default_emotes_thumbnails/%s.png" % emote_urn
		)
	else:
		var emote_data := Global.content_provider.get_wearable(emote_urn)
		if emote_data == null:
			# Fallback to default emote
			await async_load_from_urn(Emotes.DEFAULT_EMOTE_NAMES.keys()[0], index)
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
		mouse_entered.connect(self._on_mouse_entered)
		mouse_exited.connect(self._on_mouse_exited)
		gui_input.connect(self._on_gui_input)

		label_number.set_visible(not Global.is_mobile())


# Executed with @tool
func _on_item_rect_changed():
	%TextureRect_Picture.set_rotation(-get_rotation())
	%Label_Number.set_rotation(-get_rotation())


func _on_mouse_exited():
	texture_rect_selected.hide()
	inside = false
	select_emote.emit(false, emote_urn)


func _on_mouse_entered():
	texture_rect_selected.show()
	inside = true
	select_emote.emit(true, emote_urn)


func _on_gui_input(event):
	if event is InputEventScreenTouch:
		if event.pressed != pressed:
			pressed = event.pressed
			texture_rect_pressed.set_visible(pressed)
			if !pressed:
				if inside:
					play_emote.emit(emote_urn)
