@tool
extends Control

@export var avatar_name: String = "Kiko":
	set(new_value):
		avatar_name = new_value
		_update_node()

@export var avatar_subaddress: String = "1234":
	set(new_value):
		avatar_subaddress = new_value
		_update_node()

@export var hide_subaddress: bool = false:
	set(new_value):
		hide_subaddress = new_value
		_update_node()

@export var avatar_has_claimed_name: bool = false:
	set(new_value):
		avatar_has_claimed_name = new_value
		_update_node()

@export var fit_text_to_label: bool = false:
	set(new_value):
		fit_text_to_label = new_value
		_update_node()

@export var max_font_size: int = 16:
	set(new_value):
		max_font_size = new_value
		_update_node()

@export var font: Font = null:
	set(new_value):
		font = new_value
		_update_node()

@export var hbox_alignament: BoxContainer.AlignmentMode = BoxContainer.ALIGNMENT_BEGIN:
	set(new_value):
		hbox_alignament = new_value
		_update_node()

@onready var container_label = %Container_Label
@onready var label_name = %Label_Name
@onready var label_subaddress = %Label_Subaddress


func _update_node():
	if not is_node_ready():
		return

	label_name.text = avatar_name
	label_subaddress.text = "#" + avatar_subaddress

	label_name.add_theme_color_override(
		"font_color", Color.GOLD if avatar_has_claimed_name else Color.WHITE
	)
	label_subaddress.visible = not avatar_has_claimed_name and not hide_subaddress

	if font:
		label_name.add_theme_font_override("font", font)
		label_subaddress.add_theme_font_override("font", font)

	container_label.set_alignment(hbox_alignament)

	var profile_name = avatar_name
	if not avatar_has_claimed_name and not hide_subaddress:
		profile_name += "#" + avatar_subaddress

	if fit_text_to_label:
		var font_size = get_font_size_adapted(label_name, profile_name)
		label_name.add_theme_font_size_override("font_size", font_size)
		label_subaddress.add_theme_font_size_override("font_size", font_size)

		container_label.size = self.size
	else:
		label_name.add_theme_font_size_override("font_size", max_font_size)
		label_subaddress.add_theme_font_size_override("font_size", max_font_size)

		var text_size = get_string_size(label_name, profile_name)
		self.size = text_size
		#container_label.size = text_size
		container_label.set_size.call_deferred(text_size)

		container_label.set_position(Vector2(0, 0))


func get_string_size(label: Label, profile_name: String):
	var current_font = label.get_theme_font("font")
	return current_font.get_string_size(
		profile_name, HORIZONTAL_ALIGNMENT_CENTER, -1, max_font_size
	)


func get_font_size_adapted(label: Label, profile_name: String) -> int:
	var current_font = label.get_theme_font("font")
	var current_font_size = max_font_size
	while true:
		var string_size = current_font.get_string_size(
			profile_name, HORIZONTAL_ALIGNMENT_CENTER, -1, current_font_size
		)
		if string_size.x <= self.size.x or current_font_size <= 8:
			break
		current_font_size -= 1

	return current_font_size


func load_from_profile(profile: DclUserProfile):
	avatar_has_claimed_name = profile.has_claimed_name()
	avatar_name = profile.get_name()
	avatar_subaddress = profile.get_ethereum_address().right(4)


func _ready():
	resized.connect(self._on_resized)
	_update_node()


func _on_resized():
	if fit_text_to_label:
		_update_node()
