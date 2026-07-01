extends VBoxContainer

@export_enum("wearables", "emotes") var marketplace_section: String = "wearables"

@onready var vbox_content: VBoxContainer = %VBoxContainer_Content
@onready var label_iap: Label = %Label_Iap


func _ready():
	var iap_available = Iap.is_available()
	label_iap.visible = iap_available
	vbox_content.visible = not iap_available


func _on_rich_text_box_meta_clicked(_meta):
	MarketplaceTracker.open_and_track(
		DclUrls.marketplace() + "/browse?section=" + marketplace_section
	)
