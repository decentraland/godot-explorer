extends Control

@onready var item_preview: ItemPreview = %ItemPreview
@onready var label_name: Label = %Label_Name
@onready var label_rarity: Label = %Label_Rarity
@onready var button_buy: Button = %Button_Buy
@onready var margin_container: MarginContainer = $MarginContainer
@onready var panel_container_rarity: PanelContainer = %PanelContainer_Rarity

var is_pressed:bool = false
var is_buyable:bool = true


func _ready():
	UiSounds.install_audio_recusirve(self)
	_update_view()

func async_set_item(item:DclItemEntityDefinition):
	item_preview.async_set_item(item)
	label_name.text = item.get_display_name()
	var rarity = item.get_rarity()
	if rarity.length() != 0:
		label_rarity.text = rarity.to_upper()
		panel_container_rarity.modulate = Wearables.RarityColor[rarity.to_upper()]
	else:
		label_rarity.text = 'BASE'
		is_buyable = false

func set_base_emote(urn:String):
	item_preview.set_base_emote_info(urn)
	label_rarity.text = 'BASE'
	label_name.text = Emotes.DEFAULT_EMOTE_NAMES[urn]
	is_buyable = false


func _update_view() -> void:
	var margin_pressed = 0
	var margin_unpressed = 15
	if is_pressed:
		if is_buyable:
			button_buy.show()
		margin_container.add_theme_constant_override("margin_top", margin_pressed)
		margin_container.add_theme_constant_override("margin_left", margin_pressed)
		margin_container.add_theme_constant_override("margin_bottom", margin_pressed)
		margin_container.add_theme_constant_override("margin_right", margin_pressed)
	else:
		button_buy.hide()
		margin_container.add_theme_constant_override("margin_top", margin_unpressed)
		margin_container.add_theme_constant_override("margin_left", margin_unpressed)
		margin_container.add_theme_constant_override("margin_bottom", margin_unpressed)
		margin_container.add_theme_constant_override("margin_right", margin_unpressed)


func _on_toggled(toggled_on: bool) -> void:
	is_pressed = toggled_on
	_update_view()
	
	
