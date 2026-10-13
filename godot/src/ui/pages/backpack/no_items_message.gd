extends VBoxContainer

@export_enum("wearables", "emotes") var marketplace_section: String = "wearables"

@onready var vbox_content: VBoxContainer = %VBoxContainer_Content
@onready var label_iap: Label = %Label_Iap


func _ready():
	# The rich empty state carries a link to the web marketplace, so it only ships where
	# IAP is absent AND external purchase links are allowed (#2814). Anywhere else the
	# plain label stands in.
	var show_marketplace_link := (
		not Iap.is_available() and StorePolicy.can_show_external_purchase_links()
	)
	vbox_content.visible = show_marketplace_link
	label_iap.visible = not show_marketplace_link


func _on_rich_text_box_meta_clicked(_meta):
	MarketplaceTracker.open_and_track(DclUrls.marketplace_browse(marketplace_section))
