extends Control

signal emote_pressed
signal stop_emote

var is_pressed: bool = false
var is_buyable: bool = true
var is_emote: bool = false
var urn: String = ""
var marketplace_link: String = ""

@onready var item_preview: ItemPreview = %ItemPreview
@onready var label_rarity: Label = %Label_Rarity
@onready var button_view: Button = %Button_View
@onready var margin_container: MarginContainer = $MarginContainer
@onready var panel_container_rarity: PanelContainer = %PanelContainer_Rarity
@onready var marquee_label_name: ScrollContainer = %MarqueeLabel_Name


func _ready():
	UiSounds.install_audio_recusirve(self)
	_update_view()


func async_set_item(item: DclItemEntityDefinition):
	item_preview.async_set_item(item)
	marquee_label_name.set_text(item.get_display_name())
	var rarity = item.get_rarity()
	if rarity.length() != 0:
		label_rarity.text = rarity.to_upper()
		panel_container_rarity.modulate = Wearables.RarityColor[rarity.to_upper()]
		var item_id = item.get_id()
		var urn_parts = item_id.split(":")
		if urn_parts.size() >= 2:
			var contract_address = urn_parts[urn_parts.size() - 2]
			var item_number = urn_parts[urn_parts.size() - 1]
			marketplace_link = (
				"https://decentraland.org/marketplace/contracts/"
				+ contract_address
				+ "/items/"
				+ item_number
			)
	else:
		label_rarity.text = "BASE"
		is_buyable = false
		self.disabled = true


func _update_view() -> void:
	if is_pressed:
		marquee_label_name.check_and_start_marquee()
		if is_buyable:
			button_view.show()
	else:
		button_view.hide()
		marquee_label_name.stop_marquee_effect()


func _on_toggled(toggled_on: bool) -> void:
	is_pressed = toggled_on
	_update_view()
	if is_emote:
		if toggled_on:
			emit_signal("emote_pressed", urn)
		else:
			emit_signal("stop_emote")


func set_as_emote(emote_urn: String) -> void:
	is_emote = true
	urn = emote_urn


func _on_button_view_pressed() -> void:
	if marketplace_link.length() > 0:
		Global.open_url(marketplace_link)
