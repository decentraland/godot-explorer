extends Control

const PROFILE_EQUIPPED_ITEM = preload("res://src/ui/components/profile/profile_equipped_item.tscn")
const PROFILE_LINK_BUTTON = preload("res://src/ui/components/profile/profile_link_button.tscn")

const NICK_MAX_LENGTH: int = 15

@onready var h_box_container_about_1: HBoxContainer = %HBoxContainer_About1
@onready var label_no_links: Label = %Label_NoLinks
@onready var label_editing_links: Label = %Label_EditingLinks
@onready var scroll_container: ScrollContainer = %ScrollContainer
@onready var avatar_preview_portrait: AvatarPreview = %AvatarPreviewPortrait
@onready var avatar_preview_landscape: AvatarPreview = %AvatarPreviewLandscape
@onready var avatar_loading_landscape: TextureProgressBar = %TextureProgressBar_AvatarLoading
@onready var avatar_loading_portrait: TextureProgressBar = $ColorRect/SafeMarginContainer/Panel/MarginContainer/HBoxContainer/VBoxContainer_info/ScrollContainer/VBoxContainer/Control_Avatar/TextureProgressBar_AvatarLoading
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
@onready var profile_field_option_relationship_status: MarginContainer = %ProfileFieldOption_RelationshipStatus
@onready var profile_field_option_sexual_orientation: MarginContainer = %ProfileFieldOption_SexualOrientation
@onready var profile_field_option_employment_status: MarginContainer = %ProfileFieldOption_EmploymentStatus
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
@onready var button_send_dm: Button = %Button_SendDM
@onready var label_no_intro: Label = %Label_NoIntro
@onready var label_nick_length: Label = %Label_NickLength
@onready var text_edit_new_nick: TextEdit = %TextEdit_NewNick
@onready var button_nick_save: Button = %Button_NickSave
@onready var color_rect_change_nick: ColorRect = %ColorRect_ChangeNick
@onready var button_claim_name: Button = %Button_ClaimName
@onready var label_new_nick_tag: Label = %Label_NewNickTag
@onready var button_claim_name_2: Button = %Button_ClaimName2
@onready var color_rect_new_link: ColorRect = %ColorRect_NewLink
@onready var url_popup: ColorRect = %UrlPopup
@onready var profile_new_link_popup: ColorRect = %ProfileNewLinkPopup
@onready var change_nick_popup: ColorRect = %ChangeNickPopup


var url_to_visit: String = ""
var links = []
var links_to_save = []
var avatar_loading_counter: int = 0
var isOwnPassport: bool = false
var hasClaimedName: bool = false
var current_profile: DclUserProfile = null
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

func _ready() -> void:
	url_popup.close()
	change_nick_popup.close()
	profile_new_link_popup.close()
	scroll_container.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_turn_about_editing(false)
	_turn_links_editing(false)
	
	button_edit_about.hide()
	button_edit_links.hide()
	
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
	original_sexual_orientation_index = profile_field_option_sexual_orientation.option_button.selected
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


func _save_profile_changes() -> void:
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
		var relationship_text = _get_option_text(profile_field_option_relationship_status, current_relationship_index)
		ProfileHelper.get_mutable_profile().set_relationship_status(relationship_text)
		original_relationship_index = current_relationship_index
	
	var current_sexual_orientation_index = profile_field_option_sexual_orientation.option_button.selected
	if current_sexual_orientation_index != original_sexual_orientation_index:
		var sexual_orientation_text = _get_option_text(profile_field_option_sexual_orientation, current_sexual_orientation_index)
		ProfileHelper.get_mutable_profile().set_sexual_orientation(sexual_orientation_text)
		original_sexual_orientation_index = current_sexual_orientation_index
	
	var current_employment_index = profile_field_option_employment_status.option_button.selected
	if current_employment_index != original_employment_index:
		var employment_text = _get_option_text(profile_field_option_employment_status, current_employment_index)
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
	
	ProfileHelper.save_profile()


func _update_elements_visibility() -> void:
	print("isOwnProfile = ", isOwnPassport)
	if isOwnPassport:
		button_add_friend.hide()
		button_block_user.hide()
		button_send_dm.hide()
		button_edit_about.show()
		button_edit_links.show()
		button_edit_nick.show()
		if hasClaimedName:
			button_claim_name.hide()
		else:
			button_claim_name.show()
	else:
		button_add_friend.show()
		button_block_user.show()
		button_send_dm.show()
		button_edit_about.hide()
		button_edit_links.hide()
		button_edit_nick.hide()
		button_claim_name.hide()

	if hasClaimedName:
		texture_rect_claimed_checkmark.show()
		label_tag.text = ""
		label_tag.hide()
		label_new_nick_tag.text = ""
		label_new_nick_tag.hide()
		button_claim_name.hide()
	else:
		texture_rect_claimed_checkmark.hide()
		label_tag.show()
		label_tag.text = "#" + address.substr(address.length() - 4, 4)
		
		if isOwnPassport:
			button_claim_name.show()


func _on_color_rect_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if event.pressed:
			close()


func _set_avatar_loading() -> int:
	button_edit_about.hide()
	button_edit_links.hide()
	avatar_preview_portrait.hide()
	avatar_preview_landscape.hide()
	avatar_loading_landscape.show()
	avatar_loading_portrait.show()
	avatar_loading_counter += 1
	return avatar_loading_counter


func _unset_avatar_loading(current: int):
	if current != avatar_loading_counter:
		return
	avatar_loading_landscape.hide()
	avatar_loading_portrait.hide()
	avatar_preview_portrait.show()
	avatar_preview_landscape.show()
	_on_stop_emote()
	if not avatar_preview_landscape.avatar.emote_controller.is_playing():
		avatar_preview_landscape.avatar.emote_controller.play_emote("wave")
	if not avatar_preview_portrait.avatar.emote_controller.is_playing():
		avatar_preview_portrait.avatar.emote_controller.play_emote("wave")
		
		
func async_show_profile(profile: DclUserProfile) -> void:
	current_profile = profile
	
	
	if player_profile != null:
		isOwnPassport = profile.get_ethereum_address() == player_profile.get_ethereum_address()
	else:
		isOwnPassport = false

	hasClaimedName = profile.has_claimed_name()
	
	var name = profile.get_name()
	label_nickname.text = name
	
	
	address = profile.get_ethereum_address()
	label_address.text = Global.shorten_address(address)
	
	var loading_id := _set_avatar_loading()
	
	var country = profile.get_country()
	var country_index = _find_option_index(country, ProfileConstants.COUNTRIES)
	profile_field_option_country.select_option(country_index)
	
	var language = profile.get_language()
	var language_index = _find_option_index(language, ProfileConstants.LANGUAGES)
	profile_field_option_language.select_option(language_index)
	
	var pronouns = profile.get_pronouns()
	var pronouns_index = _find_option_index(pronouns, ProfileConstants.PRONOUNS)
	profile_field_option_pronouns.select_option(pronouns_index)
	
	var gender = profile.get_gender()
	var gender_index = _find_option_index(gender, ProfileConstants.GENDERS)
	profile_field_option_gender.select_option(gender_index)
	
	var relationship_status = profile.get_relationship_status()
	var relationship_index = _find_option_index(relationship_status, ProfileConstants.RELATIONSHIP_STATUS)
	profile_field_option_relationship_status.select_option(relationship_index)
	
	var sexual_orientation = profile.get_sexual_orientation()
	var sexual_orientation_index = _find_option_index(sexual_orientation, ProfileConstants.SEXUAL_ORIENTATIONS)
	profile_field_option_sexual_orientation.select_option(sexual_orientation_index)
	
	var employment_status = profile.get_employment_status()
	var employment_index = _find_option_index(employment_status, ProfileConstants.EMPLOYMENT_STATUS)
	profile_field_option_employment_status.select_option(employment_index)
	
	var profession = profile.get_profession()
	profile_field_text_profession.set_text(profession, true)
	
	var real_name = profile.get_real_name()
	profile_field_text_real_name.set_text(real_name, true)
	
	var hobbies = profile.get_hobbies()
	profile_field_text_hobbies.set_text(hobbies, true)
	
	var about_me = profile.get_description()
	profile_field_text_about_me.set_text(about_me, true)
	
	
	_refresh_links(profile)
		
	var equipped_button_group = ButtonGroup.new()
	equipped_button_group.allow_unpress = true
	
	
		
	for child in h_flow_container_equipped_wearables.get_children():
		child.queue_free()
	
	await avatar_preview_portrait.avatar.async_update_avatar_from_profile(profile)
	await avatar_preview_landscape.avatar.async_update_avatar_from_profile(profile)
	
	var nickname_color = avatar_preview_landscape.avatar.get_nickname_color(profile.get_name())
	label_nickname.add_theme_color_override("font_color", nickname_color)
	
	var profile_dictionary = profile.to_godot_dictionary()
	var avatar_data = profile_dictionary.get("content", {}).get("avatar", {})
	var wearables_urns = avatar_data.get("wearables", [])

	if not wearables_urns.is_empty():
		var equipped_wearables_promises = Global.content_provider.fetch_wearables(
			wearables_urns, 
			Global.realm.get_profile_content_url()
		)
		await PromiseUtils.async_all(equipped_wearables_promises)
		
		for wearable_urn in wearables_urns:
			var wearable_definition: DclItemEntityDefinition = Global.content_provider.get_wearable(wearable_urn)
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
			var emote_definition: DclItemEntityDefinition = Global.content_provider.get_wearable(emote.urn)
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
	
	change_nick_popup.close()
	_unset_avatar_loading(loading_id)
	_turn_about_editing(false)
	_turn_links_editing(false)
	_update_elements_visibility()

func _on_emote_pressed(urn: String) -> void:
	avatar_preview_landscape.avatar.emote_controller.stop_emote()
	if not avatar_preview_landscape.avatar.emote_controller.is_playing():
		avatar_preview_landscape.avatar.emote_controller.play_emote(urn)
	avatar_preview_portrait.avatar.emote_controller.stop_emote()
	if not avatar_preview_portrait.avatar.emote_controller.is_playing():
		avatar_preview_portrait.avatar.emote_controller.play_emote(urn)


func _on_stop_emote() -> void:
	avatar_preview_landscape.avatar.emote_controller.stop_emote()
	avatar_preview_portrait.avatar.emote_controller.stop_emote()


func _on_button_edit_about_pressed() -> void:
	_save_original_values()
	_turn_about_editing(true)


func _on_button_edit_links_pressed() -> void:
	_turn_links_editing(true)


func _turn_about_editing(editing:bool) -> void:
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
		if isOwnPassport:
			button_edit_about.show()

	for child in h_box_container_about_1.get_children():
		child.emit_signal('change_editing', editing)
	for child in grid_container_about.get_children():
		child.emit_signal('change_editing', editing)
	

func _turn_links_editing(editing:bool) -> void:
	button_add_link.disabled = links.size() >= 5 
	_reorder_add_link_button()
	for child in h_flow_container_links.get_children():
		if child.is_class("ProfileLinkButton"):
			child.emit_signal('change_editing', editing)
	if editing:
		links_to_save = links
		button_add_link.show()
		label_editing_links.show()
		v_box_container_links_actions.show()
		button_edit_links.hide()
		label_no_links.hide()
	else:
		if links.size() == 0:
			label_no_links.show()
		else:
			label_no_links.hide()
		button_add_link.hide()
		label_editing_links.hide()
		v_box_container_links_actions.hide()
		if isOwnPassport:
			button_edit_links.show()


func _on_button_about_cancel_pressed() -> void:
	_restore_original_values()
	_turn_about_editing(false)


func _on_button_links_cancel_pressed() -> void:
	_turn_links_editing(false)
	_refresh_links(player_profile)
	


func _on_button_about_save_pressed() -> void:
	if current_profile != null:
		_save_profile_changes()
		_turn_about_editing(false)
	else:
		printerr("No current profile to save")


func _on_button_copy_nick_pressed() -> void:
	DisplayServer.clipboard_set(label_nickname.text + label_tag.text)


func _on_button_copy_address_pressed() -> void:
	DisplayServer.clipboard_set(address)

	

func close() -> void:
	hide()
	_turn_links_editing(false)
	_turn_about_editing(false)


func _on_button_close_profile_pressed() -> void:
	close()


func _on_button_claim_name_pressed() -> void:
	Global.open_url("https://decentraland.org/marketplace/names/claim")
	


func _on_button_edit_nick_pressed() -> void:
	change_nick_popup.open()


func _on_color_rect_change_nick_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if event.pressed:
			color_rect_change_nick.hide()


func _on_button_nick_cancel_pressed() -> void:
	color_rect_change_nick.hide() 


func _on_button_nick_save_pressed() -> void:
	ProfileHelper.get_mutable_profile().set_name(text_edit_new_nick.text)
	ProfileHelper.save_profile()
	label_nickname.text = text_edit_new_nick.text
	color_rect_change_nick.hide()


func _on_text_edit_new_nick_text_changed() -> void:
	label_nick_length.text = str(text_edit_new_nick.text.length()) + "/" + str(NICK_MAX_LENGTH)
	button_nick_save.disabled = text_edit_new_nick.text.length() > NICK_MAX_LENGTH


func _refresh_links(profile:DclUserProfile):
	links = profile.get_links()
	var children_to_remove = []
	
	for child in h_flow_container_links.get_children():
		if child.is_in_group("profile_link_buttons"):
			children_to_remove.append(child)
			
	for child in children_to_remove:
		h_flow_container_links.remove_child(child)
		child.queue_free()
		
	for link in links:
		var new_link_button = PROFILE_LINK_BUTTON.instantiate()
		h_flow_container_links.add_child(new_link_button)
		new_link_button.try_open_link.connect(_open_go_to_link)
		new_link_button.text = link.title
		new_link_button.url = link.url



func _on_button_add_link_pressed() -> void:
	profile_new_link_popup.open()


func _open_go_to_link(link_url:String)->void:
	url_popup.open(link_url)


func _on_profile_new_link_popup_add_link(title:String, url:String) -> void:
	links_to_save.append({"title":title, "url":url})
	var new_link_button = PROFILE_LINK_BUTTON.instantiate()
	h_flow_container_links.add_child(new_link_button)
	new_link_button.try_open_link.connect(_open_go_to_link)
	new_link_button.text = title
	new_link_button.url = url
	_reorder_add_link_button()

func _on_button_links_save_pressed() -> void:
	print("Player Profile Links: ", player_profile.get_links())
	ProfileHelper.get_mutable_profile().set_links(links_to_save)
	print("Mutable Profile Links: ",ProfileHelper.get_mutable_profile().get_links())
	ProfileHelper.save_profile(false)
	print("Saved Profile Links: ", Global.player_identity.get_profile_or_null().get_links())
	_refresh_links(player_profile)
	_turn_links_editing(false)


func _reorder_add_link_button() -> void:
	if h_flow_container_links.get_child_count() > 0 and h_flow_container_links.get_child(h_flow_container_links.get_child_count() - 1) != button_add_link:
		h_flow_container_links.move_child(button_add_link, h_flow_container_links.get_child_count() - 1)


func _on_change_nick_popup_update_name_on_profile(nickname: String) -> void:
	label_nickname.text = nickname
