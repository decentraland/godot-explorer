extends Control

signal close_profile

const PROFILE_EQUIPPED_ITEM = preload("res://src/ui/components/profile/profile_equipped_item.tscn")
const PROFILE_LINK_BUTTON = preload("res://src/ui/components/profile/profile_link_button.tscn")
const NICK_MAX_LENGTH: int = 15
const MUTE = preload("res://assets/ui/audio_off.svg")
const UNMUTE = preload("res://assets/ui/audio_on.svg")
const BLOCK = preload("res://assets/ui/block.svg")

@export var rounded: bool = false
@export var closable: bool = false

var url_to_visit: String = ""
var links_to_save: Array[Dictionary] = []
var avatar_loading_counter: int = 0
var is_own_passport: bool = false
var is_blocked_user: bool = false
var is_muted_user: bool = false
var current_profile: DclUserProfile = null
var current_friendship_status: int = Global.FriendshipStatus.UNKNOWN
var address: String = ""
var original_country_index: int = 0
var original_language_index: int = 0
var original_pronouns_index: int = 0
var original_gender_index: int = 0
var original_relationship_index: int = 0
var original_sexual_orientation_index: int = 0
var original_employment_index: int = 0
var original_profession: String = ""
var original_real_name: String = ""
var original_hobbies: String = ""
var original_about_me: String = ""
var player_profile = Global.player_identity.get_profile_or_null()
var _deploy_loading_id: int = -1
var _deploy_timeout_timer: Timer

@onready var h_box_container_about_1: HBoxContainer = %HBoxContainer_About1
@onready var label_no_links: Label = %Label_NoLinks
@onready var label_editing_links: Label = %Label_EditingLinks
@onready var scroll_container: ScrollContainer = %ScrollContainer
@onready var avatar_preview_portrait: AvatarPreview = %AvatarPreviewPortrait
@onready var avatar_preview_landscape: AvatarPreview = %AvatarPreviewLandscape
@onready var button_edit_about: Button = %Button_EditAbout
@onready var button_edit_links: Button = %Button_EditLinks
@onready var h_flow_container_equipped_wearables: HFlowContainer = %HFlowContainer_EquippedWearables
@onready var label_info_description: Label = %Label_InfoDescription
@onready var label_info_description_2: Label = %Label_InfoDescription2
@onready var grid_container_about: GridContainer = %GridContainer_About
@onready var h_separator_1: HSeparator = %HSeparator1
@onready var v_box_container_about_actions: VBoxContainer = %VBoxContainer_AboutActions
@onready var v_box_container_links_actions: VBoxContainer = %VBoxContainer_LinksActions
@onready var h_flow_container_links: HFlowContainer = %HFlowContainer_Links
@onready var button_add_link: Button = %Button_AddLink
@onready var profile_field_text_about_me: MarginContainer = %ProfileFieldText_AboutMe
@onready var profile_field_option_country: MarginContainer = %ProfileFieldOption_Country
@onready var profile_field_option_language: MarginContainer = %ProfileFieldOption_Language
@onready var profile_field_option_pronouns: MarginContainer = %ProfileFieldOption_Pronouns
@onready var profile_field_option_gender: MarginContainer = %ProfileFieldOption_Gender
@onready
var profile_field_option_relationship_status: MarginContainer = %ProfileFieldOption_RelationshipStatus
@onready
var profile_field_option_sexual_orientation: MarginContainer = %ProfileFieldOption_SexualOrientation
@onready
var profile_field_option_employment_status: MarginContainer = %ProfileFieldOption_EmploymentStatus
@onready var profile_field_text_profession: MarginContainer = %ProfileFieldText_Profession
@onready var profile_field_text_real_name: MarginContainer = %ProfileFieldText_RealName
@onready var profile_field_text_hobbies: MarginContainer = %ProfileFieldText_Hobbies
@onready var label_nickname: Label = %Label_Nickname
@onready var label_address: Label = %Label_Address
@onready var texture_rect_claimed_checkmark: TextureRect = %TextureRect_ClaimedCheckmark
@onready var label_tag: Label = %Label_Tag
@onready var button_edit_nick: Button = %Button_EditNick
@onready var button_add_friend: Button = %Button_AddFriend
@onready var button_block_user: Button = %Button_BlockUser
@onready var label_no_intro: Label = %Label_NoIntro
@onready var button_claim_name: Button = %Button_ClaimName
@onready var url_popup: ColorRect = %UrlPopup
@onready var profile_new_link_popup: ColorRect = %ProfileNewLinkPopup
@onready var change_nick_popup: ColorRect = %ChangeNickPopup
@onready var v_box_container_content: VBoxContainer = %VBoxContainer_Content
@onready var panel_container_getting_data: PanelContainer = %PanelContainer_GettingData
@onready var button_mute_user: Button = %Button_MuteUser
@onready var control_avatar: Control = %Control_Avatar
@onready var button_close_profile: Button = %Button_CloseProfile
@onready var button_menu: Button = %Button_Menu
@onready var button_cancel_request: Button = %Button_CancelRequest
@onready var button_friend: Button = %Button_Friend
@onready var button_unfriend: Button = %Button_Unfriend
@onready var menu: ColorRect = %Menu
@onready var mutual_friends: Control = %MutualFriends
@onready var profile_header: VBoxContainer = %ProfileHeader


func _ready() -> void:
	scroll_container.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	Global.player_identity.profile_changed.connect(self._on_global_profile_changed)
	control_avatar.custom_minimum_size.y = get_viewport().get_visible_rect().size.y * .65
	button_menu.button_pressed = false
	menu.hide()

	if rounded:
		var current_style = get_theme_stylebox("panel")
		if current_style is StyleBoxFlat:
			var style_box = current_style.duplicate()
			style_box.corner_radius_top_left = 15
			style_box.corner_radius_top_right = 15
			style_box.corner_radius_bottom_right = 15
			style_box.corner_radius_bottom_left = 15
			add_theme_stylebox_override("panel", style_box)

	if closable:
		button_close_profile.show()
	else:
		button_close_profile.hide()

	_deploy_timeout_timer = Timer.new()
	_deploy_timeout_timer.wait_time = 15.0
	_deploy_timeout_timer.one_shot = true
	_deploy_timeout_timer.timeout.connect(self._async_on_deploy_timeout)
	add_child(_deploy_timeout_timer)

	_populate_about_fields()
	_update_elements_visibility()
	add_to_group("blacklist_ui_sync")

	# Connect friendship buttons
	if not button_add_friend.pressed.is_connected(_on_button_add_friend_pressed):
		button_add_friend.pressed.connect(_on_button_add_friend_pressed)
	if not button_cancel_request.pressed.is_connected(_on_button_cancel_request_pressed):
		button_cancel_request.pressed.connect(_on_button_cancel_request_pressed)
	if not button_unfriend.pressed.is_connected(_on_button_unfriend_pressed):
		button_unfriend.pressed.connect(_on_button_unfriend_pressed)
	if not button_block_user.pressed.is_connected(_on_button_block_user_pressed):
		button_block_user.pressed.connect(_on_button_block_user_pressed)


func _find_option_index(value: String, options_array: Array) -> int:
	if value.is_empty():
		return 0

	var clean_value = value.strip_edges()

	for i in range(options_array.size()):
		var option = options_array[i]

		if option == clean_value:
			return i

		if option.to_lower() == clean_value.to_lower():
			return i

	return 0


func _save_original_values() -> void:
	original_country_index = profile_field_option_country.option_button.selected
	original_language_index = profile_field_option_language.option_button.selected
	original_pronouns_index = profile_field_option_pronouns.option_button.selected
	original_gender_index = profile_field_option_gender.option_button.selected
	original_relationship_index = profile_field_option_relationship_status.option_button.selected
	original_sexual_orientation_index = (
		profile_field_option_sexual_orientation.option_button.selected
	)
	original_employment_index = profile_field_option_employment_status.option_button.selected
	original_profession = profile_field_text_profession.text_edit_value.text
	original_real_name = profile_field_text_real_name.text_edit_value.text
	original_hobbies = profile_field_text_hobbies.text_edit_value.text
	original_about_me = profile_field_text_about_me.text_edit_value.text


func _restore_original_values() -> void:
	profile_field_option_country.select_option(original_country_index)
	profile_field_option_language.select_option(original_language_index)
	profile_field_option_pronouns.select_option(original_pronouns_index)
	profile_field_option_gender.select_option(original_gender_index)
	profile_field_option_relationship_status.select_option(original_relationship_index)
	profile_field_option_sexual_orientation.select_option(original_sexual_orientation_index)
	profile_field_option_employment_status.select_option(original_employment_index)
	profile_field_text_profession.set_text(original_profession)
	profile_field_text_real_name.set_text(original_real_name)
	profile_field_text_hobbies.set_text(original_hobbies)
	profile_field_text_about_me.set_text(original_about_me)


func _get_option_text(option_field: MarginContainer, index: int) -> String:
	if index <= 0:
		return ""

	var option_button = option_field.get_node("VBoxContainer/OptionButton")
	if option_button and index < option_button.get_item_count():
		return option_button.get_item_text(index)
	return ""


func _async_save_profile_changes() -> void:
	var current_country_index = profile_field_option_country.option_button.selected
	if current_country_index != original_country_index:
		var country_text = _get_option_text(profile_field_option_country, current_country_index)
		ProfileHelper.get_mutable_profile().set_country(country_text)
		original_country_index = current_country_index

	var current_language_index = profile_field_option_language.option_button.selected
	if current_language_index != original_language_index:
		var language_text = _get_option_text(profile_field_option_language, current_language_index)
		ProfileHelper.get_mutable_profile().set_language(language_text)
		ProfileHelper.get_mutable_profile()
		original_language_index = current_language_index

	var current_pronouns_index = profile_field_option_pronouns.option_button.selected
	if current_pronouns_index != original_pronouns_index:
		var pronouns_text = _get_option_text(profile_field_option_pronouns, current_pronouns_index)
		ProfileHelper.get_mutable_profile().set_pronouns(pronouns_text)
		original_pronouns_index = current_pronouns_index

	var current_gender_index = profile_field_option_gender.option_button.selected
	if current_gender_index != original_gender_index:
		var gender_text = _get_option_text(profile_field_option_gender, current_gender_index)
		ProfileHelper.get_mutable_profile().set_gender(gender_text)
		original_gender_index = current_gender_index

	var current_relationship_index = profile_field_option_relationship_status.option_button.selected
	if current_relationship_index != original_relationship_index:
		var relationship_text = _get_option_text(
			profile_field_option_relationship_status, current_relationship_index
		)
		ProfileHelper.get_mutable_profile().set_relationship_status(relationship_text)
		original_relationship_index = current_relationship_index

	var current_sexual_orientation_index = (
		profile_field_option_sexual_orientation.option_button.selected
	)
	if current_sexual_orientation_index != original_sexual_orientation_index:
		var sexual_orientation_text = _get_option_text(
			profile_field_option_sexual_orientation, current_sexual_orientation_index
		)
		ProfileHelper.get_mutable_profile().set_sexual_orientation(sexual_orientation_text)
		original_sexual_orientation_index = current_sexual_orientation_index

	var current_employment_index = profile_field_option_employment_status.option_button.selected
	if current_employment_index != original_employment_index:
		var employment_text = _get_option_text(
			profile_field_option_employment_status, current_employment_index
		)
		ProfileHelper.get_mutable_profile().set_employment_status(employment_text)
		original_employment_index = current_employment_index

	var current_profession = profile_field_text_profession.text_edit_value.text
	if current_profession != original_profession:
		ProfileHelper.get_mutable_profile().set_profession(current_profession)
		original_profession = current_profession

	var current_real_name = profile_field_text_real_name.text_edit_value.text
	if current_real_name != original_real_name:
		ProfileHelper.get_mutable_profile().set_real_name(current_real_name)
		original_real_name = current_real_name

	var current_hobbies = profile_field_text_hobbies.text_edit_value.text
	if current_hobbies != original_hobbies:
		ProfileHelper.get_mutable_profile().set_hobbies(current_hobbies)
		original_hobbies = current_hobbies

	var current_about_me = profile_field_text_about_me.text_edit_value.text
	if current_about_me != original_about_me:
		ProfileHelper.get_mutable_profile().set_description(current_about_me)
		original_about_me = current_about_me

	await ProfileHelper.async_save_profile(false)


func _update_elements_visibility() -> void:
	# Hide all friendship buttons by default - they will be shown by _update_friendship_buttons()
	button_add_friend.hide()
	button_cancel_request.hide()
	button_friend.hide()
	button_unfriend.hide()
	url_popup.close()
	change_nick_popup.close()
	profile_new_link_popup.close()
	menu.hide()
	if is_own_passport:
		button_block_user.hide()
		button_mute_user.hide()
		button_cancel_request.hide()
		button_friend.hide()
		button_menu.hide()
		button_add_friend.hide()
		button_unfriend.hide()
		button_edit_about.show()
		button_edit_links.show()
		button_edit_nick.show()
		if current_profile != null:
			if current_profile.has_claimed_name():
				button_claim_name.hide()
			else:
				button_claim_name.show()
	else:
		button_block_user.show()
		button_mute_user.show()
		button_edit_about.hide()
		button_edit_links.hide()
		button_edit_nick.hide()
		button_claim_name.hide()
		button_menu.show()

	if current_profile != null:
		if current_profile.has_claimed_name():
			texture_rect_claimed_checkmark.show()
			label_tag.text = ""
			label_tag.hide()
			button_claim_name.hide()
		else:
			texture_rect_claimed_checkmark.hide()
			label_tag.show()
			label_tag.text = "#" + address.substr(address.length() - 4, 4)
			if is_own_passport:
				button_claim_name.show()

	_turn_links_editing(false)
	_turn_about_editing(false)


func _set_avatar_loading() -> int:
	panel_container_getting_data.show()
	profile_header.hide()
	v_box_container_content.hide()
	button_edit_about.hide()
	button_edit_links.hide()
	avatar_preview_portrait.hide()
	avatar_preview_landscape.hide()
	avatar_loading_counter += 1
	return avatar_loading_counter


func _unset_avatar_loading(current: int):
	if current != avatar_loading_counter:
		return
	avatar_preview_portrait.show()
	avatar_preview_landscape.show()
	panel_container_getting_data.hide()
	profile_header.show()
	v_box_container_content.show()
	_on_stop_emote()
	if not avatar_preview_landscape.avatar.emote_controller.is_playing():
		avatar_preview_landscape.avatar.emote_controller.play_emote("wave")
	if not avatar_preview_portrait.avatar.emote_controller.is_playing():
		avatar_preview_portrait.avatar.emote_controller.play_emote("wave")
	_update_elements_visibility()
	# Only update buttons for block/mute, not friendship buttons yet
	# Friendship buttons will be updated after _async_check_friendship_status() completes
	if is_own_passport:
		_update_buttons()
	else:
		# Update only block/mute buttons, not friendship buttons
		var current_avatar = avatar_preview_landscape.avatar
		is_blocked_user = Global.social_blacklist.is_blocked(current_avatar.avatar_id)
		if is_blocked_user:
			button_block_user.icon = null
			button_block_user.text = "UNBLOCK"
			button_block_user.custom_minimum_size.x = 86
			button_mute_user.hide()
		else:
			button_block_user.icon = BLOCK
			button_block_user.custom_minimum_size.x = 38
			button_block_user.text = "BLOCK"
			button_mute_user.show()

		is_muted_user = Global.social_blacklist.is_muted(current_avatar.avatar_id)
		button_mute_user.set_pressed_no_signal(is_muted_user)
		if is_muted_user:
			button_mute_user.icon = MUTE
		else:
			button_mute_user.icon = UNMUTE
	_update_friendship_buttons()


func async_show_profile(profile: DclUserProfile) -> void:
	_hide_all_social_buttons()
	current_profile = profile
	# Reset friendship status to ensure buttons don't show with old state
	current_friendship_status = Global.FriendshipStatus.UNKNOWN
	await avatar_preview_portrait.avatar.async_update_avatar_from_profile(current_profile)
	await avatar_preview_landscape.avatar.async_update_avatar_from_profile(current_profile)

	if player_profile != null:
		is_own_passport = profile.get_ethereum_address() == player_profile.get_ethereum_address()
	else:
		is_own_passport = false

	var loading_id := _set_avatar_loading()

	_refresh_about()
	_refresh_links()
	_refresh_name_and_address()
	_async_refresh_equipped_items()

	change_nick_popup.close()
	profile_new_link_popup.close()
	url_popup.close()

	_unset_avatar_loading(loading_id)

	if not is_own_passport:
		_connect_friendship_signals()
		# Wait for friendship status check before showing buttons
		await _async_check_friendship_status()
		mutual_friends.async_set_mutual_friends(profile.get_ethereum_address())

	if is_own_passport:
		var mutable := ProfileHelper.get_mutable_profile()
		if mutable != null and profile.get_profile_version() < mutable.get_profile_version():
			if _deploy_loading_id == -1:
				_deploy_loading_id = _set_avatar_loading()
				_deploy_timeout_timer.start()

	UiSounds.play_sound("mainmenu_widget_open")
	show()


func _on_emote_pressed(urn: String) -> void:
	avatar_preview_landscape.reset_avatar_rotation()
	avatar_preview_portrait.reset_avatar_rotation()
	avatar_preview_landscape.avatar.emote_controller.stop_emote()
	if not avatar_preview_landscape.avatar.emote_controller.is_playing():
		avatar_preview_landscape.avatar.emote_controller.play_emote(urn)
	avatar_preview_portrait.avatar.emote_controller.stop_emote()
	if not avatar_preview_portrait.avatar.emote_controller.is_playing():
		avatar_preview_portrait.avatar.emote_controller.play_emote(urn)


func _on_stop_emote() -> void:
	avatar_preview_landscape.avatar.emote_controller.stop_emote()
	avatar_preview_portrait.avatar.emote_controller.stop_emote()


func _on_reset_avatars_rotation() -> void:
	avatar_preview_landscape.reset_avatar_rotation()
	avatar_preview_portrait.reset_avatar_rotation()


func _on_button_edit_about_pressed() -> void:
	_save_original_values()
	_turn_about_editing(true)


func _on_button_edit_links_pressed() -> void:
	_turn_links_editing(true)


func _turn_about_editing(editing: bool) -> void:
	if editing:
		label_info_description.show()
		label_info_description_2.show()
		v_box_container_about_actions.show()
		button_edit_about.hide()
		label_no_intro.hide()
	else:
		if profile_field_text_about_me.label_value.text == "":
			label_no_intro.show()
		else:
			label_no_intro.hide()
		label_info_description.hide()
		label_info_description_2.hide()
		v_box_container_about_actions.hide()
		if is_own_passport:
			button_edit_about.show()

	for child in h_box_container_about_1.get_children():
		child.emit_signal("change_editing", editing)
	for child in grid_container_about.get_children():
		child.emit_signal("change_editing", editing)


func _turn_links_editing(editing: bool) -> void:
	for child in h_flow_container_links.get_children():
		if child.is_in_group("profile_link_buttons"):
			child.emit_signal("change_editing", editing)
	if editing:
		_check_add_link_button_status()
		label_editing_links.show()
		v_box_container_links_actions.show()
		button_edit_links.hide()
		label_no_links.hide()
	else:
		if current_profile != null:
			if current_profile.get_links().size() == 0:
				label_no_links.show()
			else:
				label_no_links.hide()
			button_add_link.hide()
			label_editing_links.hide()
			v_box_container_links_actions.hide()
		if is_own_passport:
			button_edit_links.show()

	_reorder_add_link_button()


func _on_button_about_cancel_pressed() -> void:
	_restore_original_values()
	_turn_about_editing(false)


func _on_button_links_cancel_pressed() -> void:
	_turn_links_editing(false)
	_refresh_links()


func _async_on_button_about_save_pressed() -> void:
	if current_profile != null:
		_async_save_profile_changes()
		_turn_about_editing(false)
	else:
		printerr("No current profile to save")


func _on_button_copy_nick_pressed() -> void:
	_copy_name_and_tag()


func _on_button_copy_address_pressed() -> void:
	_copy_address()


func close() -> void:
	hide()
	_hide_all_social_buttons()
	_on_stop_emote()
	_on_reset_avatars_rotation()
	_turn_links_editing(false)
	_turn_about_editing(false)
	_disconnect_friendship_signals()
	if closable:
		close_profile.emit()


func _on_button_claim_name_pressed() -> void:
	Global.open_url("https://decentraland.org/marketplace/names/claim")


func _on_button_edit_nick_pressed() -> void:
	change_nick_popup.open()


func _refresh_links() -> void:
	if current_profile == null:
		return
	var children_to_remove = []
	for child in h_flow_container_links.get_children():
		if child.is_in_group("profile_link_buttons"):
			children_to_remove.append(child)
	for child in children_to_remove:
		h_flow_container_links.remove_child(child)
		child.queue_free()
	for link in current_profile.get_links():
		_instantiate_link_button(link.title, link.url, false)


func _refresh_about() -> void:
	if current_profile == null:
		return

	var country = current_profile.get_country()
	var country_index = _find_option_index(country, ProfileConstants.COUNTRIES)
	profile_field_option_country.select_option(country_index)

	var language = current_profile.get_language()
	var language_index = _find_option_index(language, ProfileConstants.LANGUAGES)
	profile_field_option_language.select_option(language_index)

	var pronouns = current_profile.get_pronouns()
	var pronouns_index = _find_option_index(pronouns, ProfileConstants.PRONOUNS)
	profile_field_option_pronouns.select_option(pronouns_index)

	var gender = current_profile.get_gender()
	var gender_index = _find_option_index(gender, ProfileConstants.GENDERS)
	profile_field_option_gender.select_option(gender_index)

	var relationship_status = current_profile.get_relationship_status()
	var relationship_index = _find_option_index(
		relationship_status, ProfileConstants.RELATIONSHIP_STATUS
	)
	profile_field_option_relationship_status.select_option(relationship_index)

	var sexual_orientation = current_profile.get_sexual_orientation()
	var sexual_orientation_index = _find_option_index(
		sexual_orientation, ProfileConstants.SEXUAL_ORIENTATIONS
	)
	profile_field_option_sexual_orientation.select_option(sexual_orientation_index)

	var employment_status = current_profile.get_employment_status()
	var employment_index = _find_option_index(employment_status, ProfileConstants.EMPLOYMENT_STATUS)
	profile_field_option_employment_status.select_option(employment_index)

	var profession = current_profile.get_profession()
	profile_field_text_profession.set_text(profession, true)

	var real_name = current_profile.get_real_name()
	profile_field_text_real_name.set_text(real_name, true)

	var hobbies = current_profile.get_hobbies()
	profile_field_text_hobbies.set_text(hobbies, true)

	var about_me = current_profile.get_description()
	profile_field_text_about_me.set_text(about_me, true)


func _refresh_name_and_address() -> void:
	address = current_profile.get_ethereum_address()
	label_address.text = Global.shorten_address(address)

	label_nickname.text = current_profile.get_name()
	var nickname_color = DclAvatar.get_nickname_color(current_profile.get_name())
	label_nickname.add_theme_color_override("font_color", nickname_color)


func _async_refresh_equipped_items() -> void:
	var equipped_button_group = ButtonGroup.new()
	equipped_button_group.allow_unpress = true

	for child in h_flow_container_equipped_wearables.get_children():
		child.queue_free()

	var profile_dictionary = current_profile.to_godot_dictionary()
	var avatar_data = profile_dictionary.get("content", {}).get("avatar", {})
	var wearables_urns = avatar_data.get("wearables", [])

	if not wearables_urns.is_empty():
		var equipped_wearables_promises = Global.content_provider.fetch_wearables(
			wearables_urns, Global.realm.get_profile_content_url()
		)
		await PromiseUtils.async_all(equipped_wearables_promises)

		for wearable_urn in wearables_urns:
			var wearable_definition: DclItemEntityDefinition = Global.content_provider.get_wearable(
				wearable_urn
			)
			if wearable_definition != null:
				var wearable_item = PROFILE_EQUIPPED_ITEM.instantiate()
				h_flow_container_equipped_wearables.add_child(wearable_item)
				wearable_item.button_group = equipped_button_group
				wearable_item.async_set_item(wearable_definition)
			else:
				printerr("Error getting wearable: ", wearable_urn)
	else:
		printerr("Error getting wearables")

	var emotes = avatar_data.get("emotes", [])

	if not emotes.is_empty():
		for emote in emotes:
			var emote_definition: DclItemEntityDefinition = Global.content_provider.get_wearable(
				emote.urn
			)
			if emote_definition != null:
				var emote_item = PROFILE_EQUIPPED_ITEM.instantiate()
				h_flow_container_equipped_wearables.add_child(emote_item)
				emote_item.button_group = equipped_button_group
				emote_item.async_set_item(emote_definition)
				emote_item.set_as_emote(emote.urn)
				emote_item.emote_pressed.connect(_on_emote_pressed)
				emote_item.stop_emote.connect(_on_stop_emote)
			else:
				if Emotes.is_emote_default(emote.urn):
					var emote_item = PROFILE_EQUIPPED_ITEM.instantiate()
					h_flow_container_equipped_wearables.add_child(emote_item)
					emote_item.button_group = equipped_button_group
					emote_item.set_base_emote(emote.urn)
					emote_item.emote_pressed.connect(_on_emote_pressed)
					emote_item.stop_emote.connect(_on_stop_emote)

	else:
		printerr("Error getting emotes")


func _on_button_add_link_pressed() -> void:
	if links_to_save.size() < 5:
		profile_new_link_popup.open()


func _open_go_to_link(link_url: String) -> void:
	url_popup.open(link_url)


func _reorder_add_link_button() -> void:
	if (
		h_flow_container_links.get_child_count() > 0
		and (
			h_flow_container_links.get_child(h_flow_container_links.get_child_count() - 1)
			!= button_add_link
		)
	):
		h_flow_container_links.move_child(
			button_add_link, h_flow_container_links.get_child_count() - 1
		)


func _on_change_nick_popup_update_name_on_profile(nickname: String) -> void:
	label_nickname.text = nickname


func _copy_name_and_tag() -> void:
	DisplayServer.clipboard_set(label_nickname.text + label_tag.text)


func _copy_address() -> void:
	DisplayServer.clipboard_set(address)


func _on_label_nickname_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if event.pressed:
			_copy_name_and_tag()


func _on_label_tag_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if event.pressed:
			_copy_name_and_tag()


func _on_label_address_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if event.pressed:
			_copy_address()


func _on_delete_link() -> void:
	call_deferred("_check_add_link_button_status")


func _check_add_link_button_status() -> void:
	var links_quantity = 0
	for child in h_flow_container_links.get_children():
		if child.is_in_group("profile_link_buttons"):
			if child.is_queued_for_deletion():
				continue
			links_quantity = links_quantity + 1
	if links_quantity >= 5:
		button_add_link.hide()
	else:
		button_add_link.show()


func _instantiate_link_button(title: String, url: String, editing: bool) -> void:
	var new_link_button = PROFILE_LINK_BUTTON.instantiate()
	h_flow_container_links.add_child(new_link_button)
	new_link_button.try_open_link.connect(_open_go_to_link)
	new_link_button.text = title
	new_link_button.url = url
	new_link_button.emit_signal("change_editing", editing)
	new_link_button.connect("delete_link", _on_delete_link)


func _on_profile_new_link_popup_add_link(title: String, url: String) -> void:
	_instantiate_link_button(title, url, true)
	_reorder_add_link_button()
	_check_add_link_button_status()


func _async_on_button_links_save_pressed():
	links_to_save.clear()
	for child in h_flow_container_links.get_children():
		if child.is_in_group("profile_link_buttons"):
			links_to_save.append({"title": child.text, "url": child.url})
	ProfileHelper.get_mutable_profile().set_links(links_to_save)
	await ProfileHelper.async_save_profile(false)
	_turn_links_editing(false)


func _populate_about_fields() -> void:
	for country in ProfileConstants.COUNTRIES:
		profile_field_option_country.add_option(country)
	for language in ProfileConstants.LANGUAGES:
		profile_field_option_language.add_option(language)
	for pronoun in ProfileConstants.PRONOUNS:
		profile_field_option_pronouns.add_option(pronoun)
	for gender in ProfileConstants.GENDERS:
		profile_field_option_gender.add_option(gender)
	for relationship in ProfileConstants.RELATIONSHIP_STATUS:
		profile_field_option_relationship_status.add_option(relationship)
	for sexual_orientation in ProfileConstants.SEXUAL_ORIENTATIONS:
		profile_field_option_sexual_orientation.add_option(sexual_orientation)
	for employment_status in ProfileConstants.EMPLOYMENT_STATUS:
		profile_field_option_employment_status.add_option(employment_status)


func _on_global_profile_changed(new_profile: DclUserProfile) -> void:
	if new_profile == null:
		return
	var new_addr = new_profile.get_ethereum_address()
	if not is_own_passport and new_addr != address:
		return
	current_profile = new_profile
	_refresh_links()
	_refresh_about()
	_refresh_name_and_address()
	if _deploy_loading_id != -1:
		_unset_avatar_loading(_deploy_loading_id)
		_deploy_loading_id = -1
	if _deploy_timeout_timer != null and _deploy_timeout_timer.is_stopped() == false:
		_deploy_timeout_timer.stop()


func _async_on_deploy_timeout() -> void:
	if _deploy_loading_id == -1:
		return
	var addr = Global.player_identity.get_address_str()
	var lambda_url = Global.realm.get_lambda_server_base_url()
	await Global.player_identity.async_fetch_profile(addr, lambda_url)
	if _deploy_loading_id != -1:
		_unset_avatar_loading(_deploy_loading_id)
		_deploy_loading_id = -1


func _on_button_mute_user_toggled(toggled_on: bool) -> void:
	if toggled_on:
		Global.social_blacklist.add_muted(avatar_preview_landscape.avatar.avatar_id)
	else:
		Global.social_blacklist.remove_muted(avatar_preview_landscape.avatar.avatar_id)
	_update_buttons()

	_notify_other_components_of_change()


func _check_block_and_mute_status() -> void:
	var current_avatar = avatar_preview_landscape.avatar
	is_blocked_user = Global.social_blacklist.is_blocked(current_avatar.avatar_id)
	is_muted_user = Global.social_blacklist.is_muted(current_avatar.avatar_id)

	if is_blocked_user:
		button_block_user.hide()
		button_mute_user.hide()
	elif is_muted_user:
		button_block_user.show()
		button_mute_user.button_pressed = true


func _update_buttons() -> void:
	if is_own_passport:
		return
	var current_avatar = avatar_preview_landscape.avatar
	is_blocked_user = Global.social_blacklist.is_blocked(current_avatar.avatar_id)
	if is_blocked_user:
		button_block_user.icon = null
		button_block_user.text = "UNBLOCK"
		button_block_user.custom_minimum_size.x = 86
		button_mute_user.hide()
	else:
		button_block_user.icon = BLOCK
		button_block_user.custom_minimum_size.x = 38
		button_block_user.text = "BLOCK"
		button_mute_user.show()

	is_muted_user = Global.social_blacklist.is_muted(current_avatar.avatar_id)
	button_mute_user.set_pressed_no_signal(is_muted_user)
	if is_muted_user:
		button_mute_user.icon = MUTE
	else:
		button_mute_user.icon = UNMUTE

	# Update friendship buttons based on status (only if status has been checked)
	# Don't update if status is still UNKNOWN and we haven't verified it yet
	if current_friendship_status != Global.FriendshipStatus.UNKNOWN or is_own_passport:
		_update_friendship_buttons()


func _on_button_block_user_pressed() -> void:
	var current_avatar = avatar_preview_landscape.avatar
	is_blocked_user = Global.social_blacklist.is_blocked(current_avatar.avatar_id)
	if is_blocked_user:
		Global.social_blacklist.remove_blocked(current_avatar.avatar_id)
	else:
		Global.social_blacklist.add_blocked(current_avatar.avatar_id)
		# When blocking, also delete friendship if it exists
		_async_delete_friendship_if_exists(current_avatar.avatar_id)
	_update_buttons()
	_notify_other_components_of_change()
	button_menu.button_pressed = false


func _notify_other_components_of_change() -> void:
	if avatar_preview_landscape.avatar != null:
		Global.get_tree().call_group(
			"blacklist_ui_sync", "_sync_blacklist_ui", avatar_preview_landscape.avatar.avatar_id
		)


func _sync_blacklist_ui(changed_avatar_id: String) -> void:
	if (
		not is_own_passport
		and current_profile != null
		and avatar_preview_landscape.avatar != null
		and avatar_preview_landscape.avatar.avatar_id == changed_avatar_id
	):
		call_deferred("_update_buttons")


func _on_button_close_profile_button_up() -> void:
	close()


func _on_visibility_changed() -> void:
	if visible:
		grab_focus()


func _async_delete_friendship_if_exists(friend_address: String) -> void:
	# Check if there's an active friendship or pending request
	var promise = Global.social_service.get_friendship_status(friend_address)
	await PromiseUtils.async_awaiter(promise)

	if promise.is_rejected():
		# On error, skip deletion
		return

	var status_data = promise.get_data()
	var status = status_data.get("status", -1)

	var action_promise = null

	# Handle different relationship statuses
	match status:
		Global.FriendshipStatus.REQUEST_SENT:
			action_promise = Global.social_service.cancel_friend_request(friend_address)
		Global.FriendshipStatus.REQUEST_RECEIVED:
			action_promise = Global.social_service.reject_friend_request(friend_address)
		Global.FriendshipStatus.ACCEPTED:
			action_promise = Global.social_service.delete_friendship(friend_address)
		_:  # No relationship or other status, nothing to do
			return

	if action_promise != null:
		await PromiseUtils.async_awaiter(action_promise)

		if action_promise.is_rejected():
			printerr(
				"Failed to remove relationship when blocking: ",
				action_promise.get_data().get_error()
			)
		_async_update_buttons_and_lists()


func _get_friends_panel():
	var parent = get_parent()
	while parent != null:
		if parent.has_method("update_all_lists") and parent.has_method("is_friend_online"):
			print("Profile: Found friends panel in parent tree")
			return parent
		parent = parent.get_parent()
	var scene_tree = get_tree()
	if scene_tree == null:
		return null
	var friends_panel = _find_friends_panel_recursive(scene_tree.root)
	if friends_panel != null:
		print("Profile: Found friends panel in scene tree")
	return friends_panel


func _find_friends_panel_recursive(node: Node) -> Node:
	if node == null:
		return null
	if node.has_method("update_all_lists") and node.has_method("is_friend_online"):
		return node
	for child in node.get_children():
		var result = _find_friends_panel_recursive(child)
		if result != null:
			return result
	return null


func _force_update_all_social_lists() -> void:
	var scene_tree = get_tree()
	if scene_tree == null:
		return
	_force_update_social_lists_recursive(scene_tree.root)


func _force_update_social_lists_recursive(node: Node) -> void:
	if node == null:
		return
	if node.has_method("async_update_list") and "player_list_type" in node:
		var list_type = node.get("player_list_type")
		if list_type in [0, 1, 2, 3]:
			node.call_deferred("async_update_list")
	for child in node.get_children():
		_force_update_social_lists_recursive(child)


func _on_button_menu_toggled(toggled_on: bool) -> void:
	if toggled_on:
		menu.show()
	else:
		menu.hide()


func _connect_friendship_signals() -> void:
	if is_own_passport:
		return

	# Connect to friendship status change signals
	if not Global.social_service.friendship_request_received.is_connected(
		_on_friendship_request_received
	):
		Global.social_service.friendship_request_received.connect(_on_friendship_request_received)
	if not Global.social_service.friendship_request_accepted.is_connected(
		_on_friendship_request_accepted
	):
		Global.social_service.friendship_request_accepted.connect(_on_friendship_request_accepted)
	if not Global.social_service.friendship_request_rejected.is_connected(
		_on_friendship_request_rejected
	):
		Global.social_service.friendship_request_rejected.connect(_on_friendship_request_rejected)
	if not Global.social_service.friendship_deleted.is_connected(_on_friendship_deleted):
		Global.social_service.friendship_deleted.connect(_on_friendship_deleted)
	if not Global.social_service.friendship_request_cancelled.is_connected(
		_on_friendship_request_cancelled
	):
		Global.social_service.friendship_request_cancelled.connect(_on_friendship_request_cancelled)


func _disconnect_friendship_signals() -> void:
	# Disconnect all friendship signals
	if Global.social_service.friendship_request_received.is_connected(
		_on_friendship_request_received
	):
		Global.social_service.friendship_request_received.disconnect(
			_on_friendship_request_received
		)
	if Global.social_service.friendship_request_accepted.is_connected(
		_on_friendship_request_accepted
	):
		Global.social_service.friendship_request_accepted.disconnect(
			_on_friendship_request_accepted
		)
	if Global.social_service.friendship_request_rejected.is_connected(
		_on_friendship_request_rejected
	):
		Global.social_service.friendship_request_rejected.disconnect(
			_on_friendship_request_rejected
		)
	if Global.social_service.friendship_deleted.is_connected(_on_friendship_deleted):
		Global.social_service.friendship_deleted.disconnect(_on_friendship_deleted)
	if Global.social_service.friendship_request_cancelled.is_connected(
		_on_friendship_request_cancelled
	):
		Global.social_service.friendship_request_cancelled.disconnect(
			_on_friendship_request_cancelled
		)


func _on_friendship_request_received(friend_address: String, _message: String = "") -> void:
	_handle_friendship_change(friend_address)


func _on_friendship_request_accepted(friend_address: String) -> void:
	_handle_friendship_change(friend_address)


func _on_friendship_request_rejected(friend_address: String) -> void:
	_handle_friendship_change(friend_address)


func _on_friendship_deleted(friend_address: String) -> void:
	print("Profile: _on_friendship_deleted called for address: ", friend_address)
	_handle_friendship_change(friend_address, true)


func _on_friendship_request_cancelled(friend_address: String) -> void:
	_handle_friendship_change(friend_address)


func _handle_friendship_change(friend_address: String, is_deleted: bool = false) -> void:
	if current_profile == null or current_profile.get_ethereum_address() != friend_address:
		return
	if is_deleted:
		print("Profile: Updating buttons and lists after friendship deleted")
	_async_update_buttons_and_lists()


func _on_button_cancel_request_pressed() -> void:
	if is_own_passport or current_profile == null:
		return

	var friend_address = current_profile.get_ethereum_address()
	_async_cancel_friend_request(friend_address)


func _async_cancel_friend_request(friend_address: String) -> void:
	var promise = Global.social_service.cancel_friend_request(friend_address)
	await PromiseUtils.async_awaiter(promise)

	if promise.is_rejected():
		printerr("Failed to cancel friend request: ", promise.get_data().get_error())
		return

	_async_update_buttons_and_lists()


func _on_button_unfriend_pressed() -> void:
	if is_own_passport or current_profile == null:
		return
	button_menu.button_pressed = false
	var friend_address = current_profile.get_ethereum_address()
	_async_unfriend(friend_address)


func _async_unfriend(friend_address: String) -> void:
	print("Profile: _async_unfriend called for address: ", friend_address)
	var promise = Global.social_service.delete_friendship(friend_address)
	await PromiseUtils.async_awaiter(promise)

	if promise.is_rejected():
		printerr("Failed to unfriend: ", promise.get_data().get_error())
		return

	print("Profile: Unfriend successful, waiting for signal to update lists")
	# The signal friendship_deleted will update the UI
	# But also update immediately to ensure UI is responsive
	_async_update_buttons_and_lists()


func _on_button_add_friend_pressed() -> void:
	if is_own_passport or current_profile == null:
		return
	var friend_address = current_profile.get_ethereum_address()
	if current_friendship_status == Global.FriendshipStatus.REQUEST_RECEIVED:
		_async_accept_friend_request(friend_address)
	else:
		_async_send_friend_request(friend_address)


func _async_send_friend_request(friend_address: String) -> void:
	button_add_friend.disabled = true
	var promise = Global.social_service.send_friend_request(friend_address, "")
	await PromiseUtils.async_awaiter(promise)
	button_add_friend.disabled = false
	if promise.is_rejected():
		printerr("Failed to send friend request: ", promise.get_data().get_error())
		return
	_async_update_buttons_and_lists()


func _async_accept_friend_request(friend_address: String) -> void:
	button_add_friend.disabled = true
	var promise = Global.social_service.accept_friend_request(friend_address)
	await PromiseUtils.async_awaiter(promise)
	button_add_friend.disabled = false
	if promise.is_rejected():
		printerr("Failed to accept friend request: ", promise.get_data().get_error())
		return
	_async_update_buttons_and_lists()


func _async_check_friendship_status() -> void:
	if is_own_passport or current_profile == null:
		return

	# Check if social service is available before making the call
	if not _is_social_service_available():
		current_friendship_status = Global.FriendshipStatus.UNKNOWN
		_update_friendship_buttons()
		return

	var friend_address = current_profile.get_ethereum_address()
	var promise = Global.social_service.get_friendship_status(friend_address)
	await PromiseUtils.async_awaiter(promise)

	if promise.is_rejected():
		# On error, service might not be available or there was an error
		# Hide all friendship buttons
		current_friendship_status = Global.FriendshipStatus.UNKNOWN
		_update_friendship_buttons()
		return

	var status_data = promise.get_data()
	current_friendship_status = status_data.get("status", Global.FriendshipStatus.UNKNOWN)
	_update_friendship_buttons()


func _update_friendship_buttons() -> void:
	if is_own_passport or not _is_social_service_available():
		return
	_hide_friendship_buttons()
	match current_friendship_status:
		Global.FriendshipStatus.ACCEPTED:
			button_friend.show()
			button_friend.button_pressed = true
			button_unfriend.show()
		Global.FriendshipStatus.REQUEST_SENT:
			button_cancel_request.show()
		Global.FriendshipStatus.REQUEST_RECEIVED:
			button_add_friend.show()
			button_add_friend.text = "ACCEPT"
		_:  # NONE, UNKNOWN, or other statuses
			if not is_blocked_user:
				button_add_friend.show()
				button_add_friend.text = "ADD FRIEND"


func _is_social_service_available() -> bool:
	return Global.social_service != null


func _async_update_buttons_and_lists():
	_async_check_friendship_status()
	var friends_panel = _get_friends_panel()
	if friends_panel != null and friends_panel.has_method("update_all_lists"):
		friends_panel.update_all_lists()
	else:
		_force_update_all_social_lists()


func _hide_all_social_buttons() -> void:
	_hide_friendship_buttons()
	mutual_friends.hide()


func _hide_friendship_buttons() -> void:
	button_add_friend.hide()
	button_cancel_request.hide()
	button_friend.hide()
	button_unfriend.hide()
