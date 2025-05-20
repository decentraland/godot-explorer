extends PanelContainer

@export var filter_type: Place.Categories = Place.Categories.ALL

signal clean_searchbar
signal submited_text

@onready var icon: TextureRect = %Icon
@onready var label: DclUiInput = %Label
@onready var texture_button: Button = %TextureButton

var searchTexture = preload("res://src/ui/components/debug_panel/icons/Search.svg")

var texture_path = ""


func _ready() -> void:
	update_filtered_category()


# Called every frame. 'delta' is the elapsed time since the previous frame.
func _process(_delta: float) -> void:
	if label.text.length() == 0:
		texture_button.hide()
	else:
		texture_button.show()


func _on_texture_button_pressed() -> void:
	reset()


func reset() -> void:
	clean_searchbar.emit()
	label.clear()
	label.editable = true
	icon.self_modulate = "#000000"
	icon.texture = searchTexture


func update_filtered_category():
	label.editable = false
	texture_path = (
		"res://assets/ui/place_categories/"
		+ Place.Categories.keys()[filter_type].to_lower()
		+ "-icon.svg"
	)
	if ResourceLoader.exists(texture_path):
		var texture = load(texture_path)
		if texture != null:
			icon.texture = texture
			icon.self_modulate = "#ffffff"
	else:
		printerr("_update_place_category_icon texture_path not found ", texture_path)

	label.text = Place.Categories.keys()[filter_type].capitalize()


func _on_label_text_submitted(new_text: String) -> void:
	submited_text.emit(new_text)
