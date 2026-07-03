class_name TextWithLinks
extends RichTextLabel

## Attach to any RichTextLabel that uses [url] tags.
## Opens tapped URLs in the browser automatically — no extra wiring needed.


func _ready() -> void:
	meta_clicked.connect(_on_meta_clicked)


func _on_meta_clicked(meta: Variant) -> void:
	Global.open_url(str(meta), false)
