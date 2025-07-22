extends Control

@onready var item_preview: ItemPreview = %ItemPreview
@onready var label_name: Label = %Label_Name
@onready var label_rarity: Label = %Label_Rarity

func _ready():
	UiSounds.install_audio_recusirve(self)


func async_set_item(item:DclItemEntityDefinition):
	item_preview.async_set_item(item)
	label_name.text = item.get_display_name()
	var rarity = item.get_rarity()
	if rarity.length() != 0:
		label_rarity.text = rarity.to_upper()
	else:
		label_rarity.text = 'BASE'

func set_base_emote(urn:String):
	item_preview.set_base_emote_info(urn)
	label_rarity.text = 'BASE'
	label_name.text = Emotes.DEFAULT_EMOTE_NAMES[urn]
