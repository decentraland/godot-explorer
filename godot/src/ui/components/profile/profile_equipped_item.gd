extends Control

signal emote_pressed
signal stop_emote

@onready var item_preview: ItemPreview = %ItemPreview
@onready var label_rarity: Label = %Label_Rarity
@onready var button_buy: Button = %Button_Buy
@onready var margin_container: MarginContainer = $MarginContainer
@onready var panel_container_rarity: PanelContainer = %PanelContainer_Rarity
@onready var marquee_label_name: ScrollContainer = %MarqueeLabel_Name

var is_pressed:bool = false
var is_buyable:bool = true
var is_emote:bool = false
var urn: String = ""

func _ready():
	UiSounds.install_audio_recusirve(self)
	_update_view()

func async_set_item(item:DclItemEntityDefinition):
	item_preview.async_set_item(item)
	marquee_label_name.set_text(item.get_display_name())
	var rarity = item.get_rarity()
	if rarity.length() != 0:
		label_rarity.text = rarity.to_upper()
		panel_container_rarity.modulate = Wearables.RarityColor[rarity.to_upper()]
	else:
		label_rarity.text = 'BASE'
		is_buyable = false
		self.disabled = true

func set_base_emote(emote_urn:String):
	self.disabled = true
	item_preview.set_base_emote_info(emote_urn)
	label_rarity.text = 'BASE'
	marquee_label_name.set_text(Emotes.DEFAULT_EMOTE_NAMES[emote_urn])
	is_buyable = false
	urn = emote_urn
	is_emote = true


func _update_view() -> void:

	if is_pressed:
		marquee_label_name.check_and_start_marquee()
		if is_buyable:
			button_buy.show()
	else:
		button_buy.hide()
		marquee_label_name.stop_marquee_effect()



func _on_toggled(toggled_on: bool) -> void:
	is_pressed = toggled_on
	_update_view()
	if is_emote:
		if toggled_on:
			emit_signal("emote_pressed", urn)
		else:
			emit_signal("stop_emote")
	
func set_as_emote(emote_urn:String) -> void:
	is_emote = true
	urn = emote_urn
	
