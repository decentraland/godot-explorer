extends VBoxContainer

@export_enum("wearables", "emotes") var marketplace_section: String = "wearables"

@onready var rich_text_box: RichTextLabel = $RichTextBox


func _ready():
	if Global.is_ios():
		rich_text_box.hide()


func _on_rich_text_box_meta_clicked(_meta):
	Global.open_url(DclUrls.marketplace() + "/browse?section=" + marketplace_section)
