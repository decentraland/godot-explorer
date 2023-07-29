extends Button

@onready var panel_container = $PanelContainer
@onready var label_test = $Panel/Label_Test
@onready var sprite_2d_preview = $Panel/Sprite2D_Preview

var thumbnail_hash: String


func _ready():
	if button_pressed:
		panel_container.show()


func _on_control_scale_mouse_entered():
	scale = Vector2(1.1, 1.1)
	panel_container.show()


func _on_control_scale_mouse_exitsed():
	if not pressed:
		scale = Vector2(1, 1)
		panel_container.hide()


func set_wearable(wearable: Dictionary):
	var wearable_name: String = wearable.get("metadata", {}).get("name", "")
	var wearable_display: Array = wearable.get("metadata", {}).get("i18n", [])

	if wearable_display.size() > 0:
		label_test.text = wearable_display[0].get("text")
	else:
		label_test.text = wearable_name

	var wearable_thumbnail: String = wearable.get("metadata", {}).get("thumbnail", "")
	thumbnail_hash = wearable.get("content", {}).get(wearable_thumbnail, "")

	if not thumbnail_hash.is_empty():
		if Global.content_manager.get_resource_from_hash(thumbnail_hash) == null:
			var content_mapping: Dictionary = {
				"content": wearable.get("content", {}),
				"base_url": "https://peer.decentraland.org/content/contents/"
			}
			Global.content_manager.fetch_texture(wearable_thumbnail, content_mapping)
			Global.content_manager.content_loading_finished.connect(
				self._on_content_loading_finished
			)
		else:
			load_thumbnail()


func _on_content_loading_finished(content_hash: String):
	if content_hash != thumbnail_hash:
		return

	Global.content_manager.content_loading_finished.disconnect(self._on_content_loading_finished)
	load_thumbnail()


func load_thumbnail():
	var image = Global.content_manager.get_resource_from_hash(thumbnail_hash)
	var texture = ImageTexture.create_from_image(image)
	sprite_2d_preview.texture = texture


func _on_mouse_entered():
	scale = Vector2(1.1, 1.1)
	if not button_pressed:
		panel_container.show()


func _on_mouse_exited():
	scale = Vector2(1, 1)
	if not button_pressed:
		panel_container.hide()


func _on_toggled(_button_pressed):
	if _button_pressed:
		panel_container.show()
	else:
		panel_container.hide()
