class_name PlaceFilterButton
extends Button

signal filter_toggled(is_on: bool, type: int)

@export var filter_type: Places.Categories = Places.Categories.ALL


func _update_category_button():
	var texture_path = (
		"res://assets/ui/place_categories/"
		+ Places.Categories.keys()[filter_type].to_lower()
		+ "-icon.svg"
	)
	if ResourceLoader.exists(texture_path):
		var texture = load(texture_path)
		if texture != null:
			icon = texture
	else:
		printerr("_update_place_category_icon texture_path not found ", texture_path)

	self.text = Places.Categories.keys()[filter_type].capitalize()


func _ready():
	UiSounds.install_audio_recusirve(self)

	toggle_mode = true
	connect("toggled", Callable(self, "_on_toggled"))
	_update_category_button()


func _on_toggled(toggled_on: bool) -> void:
	emit_signal("filter_toggled", toggled_on, filter_type)
