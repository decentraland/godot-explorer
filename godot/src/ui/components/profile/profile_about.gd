@tool
extends VBoxContainer

enum AboutMode { NONE, DESCRIPTION_ONLY, ABOUT_DATA_ONLY, BOTH }

## If true, always shows expanded view, hides SEE MORE, and uses a 2-column grid.
@export var is_portrait: bool = false:
	set(value):
		is_portrait = value
		if is_inside_tree():
			grid_container_about.columns = 2 if is_portrait else 3

var _about_mode: int = AboutMode.NONE
var _about_data_count: int = 0
var _description_truncated: bool = false

@onready var margin_container_description: MarginContainer = %MarginContainer_Description
@onready var label_info_description: Label = %Label_InfoDescription
@onready var margin_container_data_about: MarginContainer = %MarginContainer_DataAbout
@onready var grid_container_about: GridContainer = %GridContainer_About
@onready var about_data_country: AboutData = %AboutData_Country
@onready var about_data_language: AboutData = %AboutData_Language
@onready var about_data_pronouns: AboutData = %AboutData_Pronouns
@onready var about_data_gender: AboutData = %AboutData_Gender
@onready var about_data_relationship_status: AboutData = %AboutData_RelationshipStatus
@onready var about_data_sexual_orientation: AboutData = %AboutData_SexualOrientation
@onready var about_data_employment_status: AboutData = %AboutData_EmploymentStatus
@onready var about_data_profession: AboutData = %AboutData_Profession
@onready var about_data_real_name: AboutData = %AboutData_RealName
@onready var about_data_hobby: AboutData = %AboutData_Hobby
@onready var margin_container_see_more: MarginContainer = %MarginContainer_SeeMore
@onready var underlined_button_see_more: UnderlinedButton = %UnderlinedButton_SeeMore


func _ready() -> void:
	if Engine.is_editor_hint():
		return
	underlined_button_see_more.toggled.connect(_on_underlined_button_toggled)


func refresh(profile: DclUserProfile) -> void:
	if Engine.is_editor_hint():
		return
	if profile == null:
		return
	if not is_inside_tree():
		return

	var description = profile.get_description()
	var has_description = not description.is_empty()

	about_data_country.set_value(profile.get_country())
	about_data_language.set_value(profile.get_language())
	about_data_pronouns.set_value(profile.get_pronouns())
	about_data_gender.set_value(profile.get_gender())
	about_data_relationship_status.set_value(profile.get_relationship_status())
	about_data_sexual_orientation.set_value(profile.get_sexual_orientation())
	about_data_employment_status.set_value(profile.get_employment_status())
	about_data_profession.set_value(profile.get_profession())
	about_data_real_name.set_value(profile.get_real_name())
	about_data_hobby.set_value(profile.get_hobbies())

	_about_data_count = 0
	for child in grid_container_about.get_children():
		if child is AboutData and child.has_value():
			_about_data_count += 1

	var has_about_data = _about_data_count > 0

	if has_description:
		var regex = RegEx.new()
		regex.compile("  +")
		description = regex.sub(description, " ", true)
		label_info_description.text = description
		label_info_description.max_lines_visible = -1
		_description_truncated = label_info_description.get_line_count() > 2

	if has_description and has_about_data:
		_about_mode = AboutMode.BOTH
	elif has_description:
		_about_mode = AboutMode.DESCRIPTION_ONLY
	elif has_about_data:
		_about_mode = AboutMode.ABOUT_DATA_ONLY
	else:
		_about_mode = AboutMode.NONE

	if is_portrait:
		grid_container_about.columns = 2
		_set_portrait_view()
	else:
		underlined_button_see_more.set_pressed_no_signal(false)
		_set_compact_view()


func _set_portrait_view() -> void:
	margin_container_see_more.hide()
	match _about_mode:
		AboutMode.NONE:
			hide()
		AboutMode.DESCRIPTION_ONLY:
			show()
			margin_container_description.show()
			_expand_description()
			margin_container_data_about.hide()
		AboutMode.ABOUT_DATA_ONLY:
			show()
			margin_container_description.hide()
			margin_container_data_about.show()
			_show_all_about_data()
		AboutMode.BOTH:
			show()
			margin_container_description.show()
			_expand_description()
			margin_container_data_about.show()
			_show_all_about_data()


func _set_compact_view() -> void:
	match _about_mode:
		AboutMode.NONE:
			hide()
		AboutMode.DESCRIPTION_ONLY:
			show()
			margin_container_description.show()
			_compact_description()
			margin_container_data_about.hide()
			margin_container_see_more.visible = _description_truncated
			underlined_button_see_more.underlined_text = "SEE MORE"
		AboutMode.ABOUT_DATA_ONLY:
			show()
			margin_container_description.hide()
			margin_container_data_about.show()
			_show_about_data_limited(3)
			margin_container_see_more.visible = _about_data_count > 3
			underlined_button_see_more.underlined_text = "SEE MORE"
		AboutMode.BOTH:
			show()
			margin_container_description.show()
			_compact_description()
			margin_container_data_about.hide()
			margin_container_see_more.show()
			underlined_button_see_more.underlined_text = "SEE MORE"


func _set_expand_view() -> void:
	match _about_mode:
		AboutMode.DESCRIPTION_ONLY:
			_expand_description()
			underlined_button_see_more.underlined_text = "SEE LESS"
		AboutMode.ABOUT_DATA_ONLY:
			margin_container_data_about.show()
			_show_all_about_data()
			underlined_button_see_more.underlined_text = "SEE LESS"
		AboutMode.BOTH:
			_expand_description()
			margin_container_data_about.show()
			_show_all_about_data()
			underlined_button_see_more.underlined_text = "SEE LESS"


func _compact_description() -> void:
	label_info_description.max_lines_visible = 2
	label_info_description.update_minimum_size()
	margin_container_description.update_minimum_size()


func _expand_description() -> void:
	label_info_description.max_lines_visible = -1
	label_info_description.update_minimum_size()
	margin_container_description.update_minimum_size()


func _show_about_data_limited(max_count: int) -> void:
	var shown := 0
	for child in grid_container_about.get_children():
		if child is AboutData and child.has_value():
			shown += 1
			child.visible = shown <= max_count
		elif child is AboutData:
			child.visible = false


func _show_all_about_data() -> void:
	for child in grid_container_about.get_children():
		if child is AboutData:
			child.visible = child.has_value()


func _on_underlined_button_toggled(toggled_on: bool) -> void:
	if toggled_on:
		_set_expand_view()
	else:
		_set_compact_view()
