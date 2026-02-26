extends TextureRect

signal close_requested(saved: bool)
signal save_failed

const PROFILE_LINK_BUTTON = preload("res://src/ui/components/profile/profile_link_button.tscn")
const MAX_LINKS = 5

const KEYBOARD_PADDING = 24.0

var _original_values: Dictionary = {}
var _current_links: Array = []
var _keyboard_spacer: Control = null
var _last_keyboard_height: int = 0

@onready var button_back: Button = %Button_BackToExplorer
@onready var button_save: Button = %Button_Save
@onready var button_cancel: Button = %Button_Cancel
@onready var scroll_container: ScrollContainer = %ScrollContainer
@onready var profile_picture: ProfilePicture = %ProfilePicture
@onready var dcl_text_edit_country: DclTextEdit = %DclTextEdit_Country
@onready var dropdown_list_pronouns: DropdownList = %DropdownList_Pronouns
@onready var dropdown_list_gender: DropdownList = %DropdownList_Gender
@onready var dropdown_list_sexual_orientation: DropdownList = %DropdownList_SexualOrientation
@onready var dropdown_list_relationship: DropdownList = %DropdownList_Relationship
@onready var dcl_text_edit_employment_status: DclTextEdit = %DclTextEdit_EmploymentStatus
@onready var dcl_text_edit_profession: DclTextEdit = %DclTextEdit_Profession
@onready var dcl_text_edit_real_name: DclTextEdit = %DclTextEdit_RealName
@onready var dcl_text_edit_hobby: DclTextEdit = %DclTextEdit_Hobby
@onready var h_flow_container_links: HFlowContainer = %HFlowContainer_Links
@onready var button_add_link: Button = %Button_AddLink
@onready var profile_new_link_popup = %ProfileNewLinkPopup
@onready var dcl_text_edit_description: DclTextEdit = %DclTextEdit_Description
@onready var dcl_text_edit_username: DclTextEdit = %DclTextEdit_Username
@onready var label_tag: Label = %Label_Tag


func _ready() -> void:
	_populate_dropdown(dropdown_list_pronouns, ProfileConstants.PRONOUNS)
	_populate_dropdown(dropdown_list_gender, ProfileConstants.GENDERS)
	_populate_dropdown(dropdown_list_sexual_orientation, ProfileConstants.SEXUAL_ORIENTATIONS)
	_populate_dropdown(dropdown_list_relationship, ProfileConstants.RELATIONSHIP_STATUS)

	dcl_text_edit_username.dcl_text_edit_changed.connect(_on_field_changed)
	dcl_text_edit_description.dcl_text_edit_changed.connect(_on_field_changed)
	dcl_text_edit_country.dcl_text_edit_changed.connect(_on_field_changed)
	dcl_text_edit_employment_status.dcl_text_edit_changed.connect(_on_field_changed)
	dcl_text_edit_profession.dcl_text_edit_changed.connect(_on_field_changed)
	dcl_text_edit_real_name.dcl_text_edit_changed.connect(_on_field_changed)
	dcl_text_edit_hobby.dcl_text_edit_changed.connect(_on_field_changed)

	dropdown_list_pronouns.item_selected.connect(_on_dropdown_changed)
	dropdown_list_gender.item_selected.connect(_on_dropdown_changed)
	dropdown_list_sexual_orientation.item_selected.connect(_on_dropdown_changed)
	dropdown_list_relationship.item_selected.connect(_on_dropdown_changed)

	button_back.pressed.connect(_on_close)
	button_cancel.pressed.connect(_on_close)
	button_save.pressed.connect(_async_save_profile)

	Global.change_virtual_keyboard.connect(_on_virtual_keyboard_changed)
	get_viewport().gui_focus_changed.connect(_on_focus_changed)

	button_save.disabled = true


func populate(profile: DclUserProfile) -> void:
	var social_data := SocialItemData.new()
	social_data.name = profile.get_name()
	social_data.address = profile.get_ethereum_address()
	social_data.has_claimed_name = profile.has_claimed_name()
	social_data.profile_picture_url = profile.get_avatar().get_snapshots_face_url()

	dcl_text_edit_username.set_text_value(social_data.name)
	if social_data.has_claimed_name:
		label_tag.text = ""
		label_tag.hide()
	else:
		label_tag.show()
		label_tag.text = ("#" + social_data.address.substr(social_data.address.length() - 4, 4))

	profile_picture.async_update_profile_picture(social_data)

	var description_val := profile.get_description().strip_edges()
	var country_val := profile.get_country().strip_edges()
	var employment_status_val := profile.get_employment_status().strip_edges()
	var profession_val := profile.get_profession().strip_edges()
	var real_name_val := profile.get_real_name().strip_edges()
	var hobby_val := profile.get_hobbies().strip_edges()

	dcl_text_edit_description.set_text_value(description_val)
	dcl_text_edit_country.set_text_value(country_val)
	dcl_text_edit_employment_status.set_text_value(employment_status_val)
	dcl_text_edit_profession.set_text_value(profession_val)
	dcl_text_edit_real_name.set_text_value(real_name_val)
	dcl_text_edit_hobby.set_text_value(hobby_val)

	profile_new_link_popup.hide()

	_current_links = []
	var links = profile.get_links()
	for link in links:
		_current_links.append(
			{"title": str(link.get("title", "")), "url": str(link.get("url", ""))}
		)
	_refresh_links_ui()

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
		"links": _current_links.duplicate(true),
	}

	button_save.disabled = true


func _on_close() -> void:
	close_requested.emit(false)


func _populate_dropdown(dropdown: DropdownList, options: Array) -> void:
	dropdown.clear()
	dropdown.add_item("Select", 0)
	for i in range(options.size()):
		dropdown.add_item(options[i], i + 1)
	dropdown.placeholder_index = 0
	dropdown.select(0)


func _find_option_index(options: Array, value: String) -> int:
	if value.is_empty():
		return 0
	var lower_value := value.to_lower()
	for i in range(options.size()):
		if options[i].to_lower() == lower_value:
			return i + 1
	return 0


func _on_field_changed() -> void:
	_check_dirty()


func _on_dropdown_changed(_index: int) -> void:
	_check_dirty()


func _has_any_error() -> bool:
	return (
		dcl_text_edit_username.error
		or dcl_text_edit_description.error
		or dcl_text_edit_country.error
		or dcl_text_edit_employment_status.error
		or dcl_text_edit_profession.error
		or dcl_text_edit_real_name.error
		or dcl_text_edit_hobby.error
	)


func _check_dirty() -> void:
	if _original_values.is_empty():
		return

	if _has_any_error():
		button_save.disabled = true
		return

	var is_dirty := false

	if dcl_text_edit_username.get_text_value() != _original_values.get("username", ""):
		is_dirty = true
	elif dcl_text_edit_description.get_text_value() != _original_values.get("description", ""):
		is_dirty = true
	elif dcl_text_edit_country.get_text_value() != _original_values.get("country", ""):
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
		dcl_text_edit_employment_status.get_text_value()
		!= _original_values.get("employment_status", "")
	):
		is_dirty = true
	elif dcl_text_edit_profession.get_text_value() != _original_values.get("profession", ""):
		is_dirty = true
	elif dcl_text_edit_real_name.get_text_value() != _original_values.get("real_name", ""):
		is_dirty = true
	elif dcl_text_edit_hobby.get_text_value() != _original_values.get("hobby", ""):
		is_dirty = true
	elif _are_links_dirty():
		is_dirty = true

	button_save.disabled = !is_dirty


func _get_dropdown_value(options: Array, index: int) -> String:
	var array_idx := index - 1
	if array_idx < 0 or array_idx >= options.size():
		return ""
	return options[array_idx]


func _async_save_profile() -> void:
	var mutable_profile: DclUserProfile = Global.player_identity.get_mutable_profile()
	if mutable_profile == null:
		return

	var current_username = dcl_text_edit_username.get_text_value()
	if current_username != _original_values.get("username", ""):
		mutable_profile.set_name(current_username)

	var current_description = dcl_text_edit_description.get_text_value()
	if current_description != _original_values.get("description", ""):
		mutable_profile.set_description(current_description)

	var current_country = dcl_text_edit_country.get_text_value()
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

	var current_employment = dcl_text_edit_employment_status.get_text_value()
	if current_employment != _original_values.get("employment_status", ""):
		mutable_profile.set_employment_status(current_employment)

	var current_profession = dcl_text_edit_profession.get_text_value()
	if current_profession != _original_values.get("profession", ""):
		mutable_profile.set_profession(current_profession)

	var current_real_name = dcl_text_edit_real_name.get_text_value()
	if current_real_name != _original_values.get("real_name", ""):
		mutable_profile.set_real_name(current_real_name)

	var current_hobby = dcl_text_edit_hobby.get_text_value()
	if current_hobby != _original_values.get("hobby", ""):
		mutable_profile.set_hobbies(current_hobby)

	if _are_links_dirty():
		var typed_links: Array[Dictionary] = []
		for link in _current_links:
			typed_links.append(link)
		mutable_profile.set_links(typed_links)

	# Optimistic: close immediately and let portrait refresh with mutable profile
	close_requested.emit(true)

	# Deploy in background
	var promise = ProfileService.async_deploy_profile(mutable_profile)
	await PromiseUtils.async_awaiter(promise)

	if promise.is_rejected():
		printerr("Failed to save profile: ", PromiseUtils.get_error_message(promise))
		save_failed.emit()


func _are_links_dirty() -> bool:
	var original_links: Array = _original_values.get("links", [])
	if _current_links.size() != original_links.size():
		return true
	for i in range(_current_links.size()):
		if _current_links[i]["title"] != original_links[i]["title"]:
			return true
		if _current_links[i]["url"] != original_links[i]["url"]:
			return true
	return false


func _refresh_links_ui() -> void:
	for child in h_flow_container_links.get_children():
		if child == button_add_link:
			continue
		h_flow_container_links.remove_child(child)
		child.queue_free()

	for i in range(_current_links.size()):
		var link = _current_links[i]
		var button = PROFILE_LINK_BUTTON.instantiate()
		h_flow_container_links.add_child(button)
		button.text = link["title"]
		button.url = link["url"]
		button.emit_signal("change_editing", true)
		var idx = i
		button.delete_link.connect(func(): _on_link_deleted(idx))

	# Keep add link button always last
	h_flow_container_links.move_child(button_add_link, -1)
	button_add_link.visible = _current_links.size() < MAX_LINKS


func _on_link_deleted(index: int) -> void:
	if index >= 0 and index < _current_links.size():
		_current_links.remove_at(index)
		_refresh_links_ui()
		_check_dirty()


func _on_add_link_pressed() -> void:
	profile_new_link_popup.open()


func _on_profile_new_link_popup_add_link(link_title: String, link_url: String) -> void:
	if _current_links.size() >= MAX_LINKS:
		return
	_current_links.append({"title": link_title, "url": link_url})
	_refresh_links_ui()
	_check_dirty()


func _get_parent_field(control: Control) -> Control:
	var node := control
	while node != null:
		if node is DclTextEdit or node is DclLineEdit:
			return node
		node = node.get_parent() as Control
	return control


func _on_virtual_keyboard_changed(keyboard_height: int) -> void:
	_last_keyboard_height = keyboard_height
	if keyboard_height == 0:
		if _keyboard_spacer != null:
			_keyboard_spacer.custom_minimum_size.y = 0
		return
	_async_ensure_field_visible()


func _on_focus_changed(_control: Control) -> void:
	if _last_keyboard_height > 0:
		_async_ensure_field_visible()


func _async_ensure_field_visible() -> void:
	var focused := get_viewport().gui_get_focus_owner()
	if focused == null:
		return

	var field := _get_parent_field(focused)

	var viewport_size := get_viewport().get_visible_rect().size
	var window_size := Vector2(DisplayServer.window_get_size())
	var y_factor: float = viewport_size.y / window_size.y
	var kb_top: float = viewport_size.y - _last_keyboard_height * y_factor

	var field_rect := field.get_global_rect()
	var field_bottom: float = field_rect.position.y + field_rect.size.y

	var overlap: float = field_bottom + KEYBOARD_PADDING - kb_top
	if overlap <= 0:
		return

	var scroll_content := scroll_container.get_child(0)
	if _keyboard_spacer == null:
		_keyboard_spacer = Control.new()
		scroll_content.add_child(_keyboard_spacer)
	_keyboard_spacer.custom_minimum_size.y = _last_keyboard_height * y_factor

	await get_tree().process_frame
	scroll_container.scroll_vertical += int(overlap)
