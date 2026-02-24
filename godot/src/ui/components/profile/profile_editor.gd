extends TextureRect

signal close_requested(saved: bool)
signal save_failed

var _original_values: Dictionary = {}

@onready var button_back: Button = %Button_BackToExplorer
@onready var button_save: Button = %Button_Save
@onready var button_cancel: Button = %Button_Cancel
@onready var dcl_line_edit_username = %DclLineEdit_Username
@onready var profile_picture: ProfilePicture = %ProfilePicture
@onready var dcl_line_edit_description = %DclLineEdit_Description
@onready var dcl_line_edit_country = %DclLineEdit_Country
@onready var dropdown_list_pronouns: DropdownList = %DropdownList_Pronouns
@onready var dropdown_list_gender: DropdownList = %DropdownList_Gender
@onready var dropdown_list_sexual_orientation: DropdownList = %DropdownList_SexualOrientation
@onready var dropdown_list_relationship: DropdownList = %DropdownList_Relationship
@onready var dcl_line_edit_employment_status = %DclLineEdit_EmploymentStatus
@onready var dcl_line_edit_profession = %DclLineEdit_Profession
@onready var dcl_line_edit_real_name = %DclLineEdit_RealName
@onready var dcl_line_edit_hobby = %DclLineEdit_Hobby


func _ready() -> void:
	_populate_dropdown(dropdown_list_pronouns, ProfileConstants.PRONOUNS)
	_populate_dropdown(dropdown_list_gender, ProfileConstants.GENDERS)
	_populate_dropdown(dropdown_list_sexual_orientation, ProfileConstants.SEXUAL_ORIENTATIONS)
	_populate_dropdown(dropdown_list_relationship, ProfileConstants.RELATIONSHIP_STATUS)

	dcl_line_edit_username.dcl_line_edit_changed.connect(_on_field_changed)
	dcl_line_edit_description.dcl_line_edit_changed.connect(_on_field_changed)
	dcl_line_edit_country.dcl_line_edit_changed.connect(_on_field_changed)
	dcl_line_edit_employment_status.dcl_line_edit_changed.connect(_on_field_changed)
	dcl_line_edit_profession.dcl_line_edit_changed.connect(_on_field_changed)
	dcl_line_edit_real_name.dcl_line_edit_changed.connect(_on_field_changed)
	dcl_line_edit_hobby.dcl_line_edit_changed.connect(_on_field_changed)

	dropdown_list_pronouns.item_selected.connect(_on_dropdown_changed)
	dropdown_list_gender.item_selected.connect(_on_dropdown_changed)
	dropdown_list_sexual_orientation.item_selected.connect(_on_dropdown_changed)
	dropdown_list_relationship.item_selected.connect(_on_dropdown_changed)

	button_back.pressed.connect(_on_close)
	button_cancel.pressed.connect(_on_close)
	button_save.pressed.connect(_async_save_profile)

	button_save.disabled = true


func populate(profile: DclUserProfile) -> void:
	var social_data := SocialItemData.new()
	social_data.name = profile.get_name()
	social_data.address = profile.get_ethereum_address()
	social_data.has_claimed_name = profile.has_claimed_name()
	social_data.profile_picture_url = profile.get_avatar().get_snapshots_face_url()

	dcl_line_edit_username.set_text_value(social_data.name)
	if social_data.has_claimed_name:
		dcl_line_edit_username.label_tag.text = ""
		dcl_line_edit_username.label_tag.hide()
	else:
		dcl_line_edit_username.label_tag.show()
		dcl_line_edit_username.label_tag.text = (
			"#" + social_data.address.substr(social_data.address.length() - 4, 4)
		)

	profile_picture.async_update_profile_picture(social_data)

	var description_val := profile.get_description().strip_edges()
	var country_val := profile.get_country().strip_edges()
	var employment_status_val := profile.get_employment_status().strip_edges()
	var profession_val := profile.get_profession().strip_edges()
	var real_name_val := profile.get_real_name().strip_edges()
	var hobby_val := profile.get_hobbies().strip_edges()

	dcl_line_edit_description.set_text_value(description_val)
	dcl_line_edit_country.set_text_value(country_val)
	dcl_line_edit_employment_status.set_text_value(employment_status_val)
	dcl_line_edit_profession.set_text_value(profession_val)
	dcl_line_edit_real_name.set_text_value(real_name_val)
	dcl_line_edit_hobby.set_text_value(hobby_val)

	var pronouns_val := profile.get_pronouns().strip_edges()
	var gender_val := profile.get_gender().strip_edges()
	var sexual_orientation_val := profile.get_sexual_orientation().strip_edges()
	var relationship_val := profile.get_relationship_status().strip_edges()

	var pronouns_idx := _find_option_index(ProfileConstants.PRONOUNS, pronouns_val)
	var gender_idx := _find_option_index(ProfileConstants.GENDERS, gender_val)
	var sexual_orientation_idx := _find_option_index(
		ProfileConstants.SEXUAL_ORIENTATIONS, sexual_orientation_val
	)
	var relationship_idx := _find_option_index(
		ProfileConstants.RELATIONSHIP_STATUS, relationship_val
	)

	dropdown_list_pronouns.select(pronouns_idx)
	dropdown_list_gender.select(gender_idx)
	dropdown_list_sexual_orientation.select(sexual_orientation_idx)
	dropdown_list_relationship.select(relationship_idx)

	_original_values = {
		"username": social_data.name,
		"description": description_val,
		"country": country_val,
		"pronouns": pronouns_idx,
		"gender": gender_idx,
		"sexual_orientation": sexual_orientation_idx,
		"relationship": relationship_idx,
		"employment_status": employment_status_val,
		"profession": profession_val,
		"real_name": real_name_val,
		"hobby": hobby_val,
	}

	button_save.disabled = true


func _on_close() -> void:
	close_requested.emit(false)


func _populate_dropdown(dropdown: DropdownList, options: Array) -> void:
	dropdown.clear()
	dropdown.add_item("Select", 0)
	for i in range(1, options.size()):
		dropdown.add_item(options[i], i)
	dropdown.select(0)


func _find_option_index(options: Array, value: String) -> int:
	if value.is_empty():
		return 0
	for i in range(options.size()):
		if options[i] == value:
			return i
	return 0


func _on_field_changed() -> void:
	_check_dirty()


func _on_dropdown_changed(_index: int) -> void:
	_check_dirty()


func _check_dirty() -> void:
	if _original_values.is_empty():
		return

	var is_dirty := false

	if dcl_line_edit_username.get_text_value() != _original_values.get("username", ""):
		is_dirty = true
	elif dcl_line_edit_description.get_text_value() != _original_values.get("description", ""):
		is_dirty = true
	elif dcl_line_edit_country.get_text_value() != _original_values.get("country", ""):
		is_dirty = true
	elif dropdown_list_pronouns.selected != _original_values.get("pronouns", 0):
		is_dirty = true
	elif dropdown_list_gender.selected != _original_values.get("gender", 0):
		is_dirty = true
	elif dropdown_list_sexual_orientation.selected != _original_values.get("sexual_orientation", 0):
		is_dirty = true
	elif dropdown_list_relationship.selected != _original_values.get("relationship", 0):
		is_dirty = true
	elif (
		dcl_line_edit_employment_status.get_text_value()
		!= _original_values.get("employment_status", "")
	):
		is_dirty = true
	elif dcl_line_edit_profession.get_text_value() != _original_values.get("profession", ""):
		is_dirty = true
	elif dcl_line_edit_real_name.get_text_value() != _original_values.get("real_name", ""):
		is_dirty = true
	elif dcl_line_edit_hobby.get_text_value() != _original_values.get("hobby", ""):
		is_dirty = true

	button_save.disabled = !is_dirty


func _get_dropdown_value(options: Array, index: int) -> String:
	if index <= 0 or index >= options.size():
		return ""
	return options[index]


func _async_save_profile() -> void:
	var mutable_profile: DclUserProfile = Global.player_identity.get_mutable_profile()
	if mutable_profile == null:
		return

	var current_username = dcl_line_edit_username.get_text_value()
	if current_username != _original_values.get("username", ""):
		mutable_profile.set_name(current_username)

	var current_description = dcl_line_edit_description.get_text_value()
	if current_description != _original_values.get("description", ""):
		mutable_profile.set_description(current_description)

	var current_country = dcl_line_edit_country.get_text_value()
	if current_country != _original_values.get("country", ""):
		mutable_profile.set_country(current_country)

	var current_pronouns_idx := dropdown_list_pronouns.selected
	if current_pronouns_idx != _original_values.get("pronouns", 0):
		mutable_profile.set_pronouns(
			_get_dropdown_value(ProfileConstants.PRONOUNS, current_pronouns_idx)
		)

	var current_gender_idx := dropdown_list_gender.selected
	if current_gender_idx != _original_values.get("gender", 0):
		mutable_profile.set_gender(
			_get_dropdown_value(ProfileConstants.GENDERS, current_gender_idx)
		)

	var current_sexual_orientation_idx := dropdown_list_sexual_orientation.selected
	if current_sexual_orientation_idx != _original_values.get("sexual_orientation", 0):
		mutable_profile.set_sexual_orientation(
			_get_dropdown_value(
				ProfileConstants.SEXUAL_ORIENTATIONS, current_sexual_orientation_idx
			)
		)

	var current_relationship_idx := dropdown_list_relationship.selected
	if current_relationship_idx != _original_values.get("relationship", 0):
		mutable_profile.set_relationship_status(
			_get_dropdown_value(ProfileConstants.RELATIONSHIP_STATUS, current_relationship_idx)
		)

	var current_employment = dcl_line_edit_employment_status.get_text_value()
	if current_employment != _original_values.get("employment_status", ""):
		mutable_profile.set_employment_status(current_employment)

	var current_profession = dcl_line_edit_profession.get_text_value()
	if current_profession != _original_values.get("profession", ""):
		mutable_profile.set_profession(current_profession)

	var current_real_name = dcl_line_edit_real_name.get_text_value()
	if current_real_name != _original_values.get("real_name", ""):
		mutable_profile.set_real_name(current_real_name)

	var current_hobby = dcl_line_edit_hobby.get_text_value()
	if current_hobby != _original_values.get("hobby", ""):
		mutable_profile.set_hobbies(current_hobby)

	# Optimistic: close immediately and let portrait refresh with mutable profile
	close_requested.emit(true)

	# Deploy in background
	var promise = ProfileService.async_deploy_profile(mutable_profile)
	await PromiseUtils.async_awaiter(promise)

	if promise.is_rejected():
		printerr("Failed to save profile: ", PromiseUtils.get_error_message(promise))
		save_failed.emit()
