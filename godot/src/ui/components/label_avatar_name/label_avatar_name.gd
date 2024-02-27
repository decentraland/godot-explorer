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


func _update_node():
	if not is_node_ready():
		return

	%Label_Name.text = avatar_name
	%Label_Subaddress.text = "#" + avatar_subaddress

	%Label_Name.add_theme_color_override(
		"font_color", Color.GOLD if avatar_has_claimed_name else Color.WHITE
	)
	%Label_Subaddress.visible = not avatar_has_claimed_name

	if font:
		%Label_Name.add_theme_font_override("font", font)
		%Label_Subaddress.add_theme_font_override("font", font)

	%Container_Label.set_alignment(hbox_alignament)

	if fit_text_to_label:
		var profile_name = avatar_name
		if not avatar_has_claimed_name:
			profile_name += "#" + avatar_subaddress

		var font_size = get_font_size_adapted(%Label_Name, profile_name)
		%Label_Name.add_theme_font_size_override("font_size", font_size)
		%Label_Subaddress.add_theme_font_size_override("font_size", font_size)
	else:
		self.size = %Container_Label.size
		%Label_Name.add_theme_font_size_override("font_size", max_font_size)
		%Label_Subaddress.add_theme_font_size_override("font_size", max_font_size)


func get_font_size_adapted(label: Label, profile_name: String) -> int:
	var font = label.get_theme_font("font")
	var current_font_size = max_font_size
	while true:
		var string_size = font.get_string_size(
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
