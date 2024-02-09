@tool
class_name EmoteWheelItem
extends Control

signal play_emote(emote_id: String)

@export var rarity: Wearables.ItemRarityEnum = Wearables.ItemRarityEnum.COMMON:
	set(new_value):
		rarity = new_value
		%Glow.set_visible(rarity != Wearables.ItemRarityEnum.COMMON)
		var color = Color("#ECEBED")
		match rarity:
			Wearables.ItemRarityEnum.COMMON:
				color = Color("#ECEBED")
			Wearables.ItemRarityEnum.UNCOMMON:
				color = Color("#FF8362")
			Wearables.ItemRarityEnum.RARE:
				color = Color("#34CE76")
			Wearables.ItemRarityEnum.EPIC:
				color = Color("#599CFF")
			Wearables.ItemRarityEnum.LEGENDARY:
				color = Color("#B262FF")
			Wearables.ItemRarityEnum.MYTHIC:
				color = Color("#FF63E1")
			Wearables.ItemRarityEnum.UNIQUE:
				color = Color("#FFB626")
		%Inner.self_modulate = color

@export var picture: Texture2D = null:
	set(new_value):
		%TextureRect_Picture.texture = new_value
		picture = new_value

@export var emote_id: String = "wave"

var pressed = false

@onready var control_inner = %Control_Inner

@onready var texture_rect_selected = %Selected
@onready var texture_rect_pressed = %Pressed

func _ready():
	if not Engine.is_editor_hint():
		mouse_entered.connect(self._on_mouse_entered)
		mouse_exited.connect(self._on_mouse_exited)
		gui_input.connect(self._on_gui_input)

# Executed with @tool
func _on_item_rect_changed():
	%TextureRect_Picture.set_rotation(-get_rotation())


func _on_mouse_exited():
	texture_rect_selected.hide()


func _on_mouse_entered():
	texture_rect_selected.show()


func _on_gui_input(event):
	if event is InputEventScreenTouch:
		if event.pressed != pressed:
			pressed = event.pressed
			texture_rect_pressed.set_visible(pressed)
			if !pressed:
				play_emote.emit(emote_id)
