extends Control

const PROFILE_EQUIPPED_ITEM = preload("res://src/ui/components/profile/profile_equipped_item.tscn")
const PROFILE_LINK_BUTTON = preload("res://src/ui/components/profile/profile_link_button.tscn")


#MOCKED DATA:
const links = [{"label":"Instagram", "link":"www.instagram.com"}, {"label":"Facebook", "link":"www.facebook.com"}]

@onready var h_box_container_about_1: HBoxContainer = %HBoxContainer_About1
@onready var label_no_links: Label = %Label_NoLinks
@onready var label_editing_links: Label = %Label_EditingLinks
@onready var scroll_container: ScrollContainer = %ScrollContainer
@onready var avatar_preview_portrait: AvatarPreview = %AvatarPreviewPortrait
@onready var avatar_preview_landscape: AvatarPreview = %AvatarPreviewLandscape
@onready var avatar_loading_landscape: TextureProgressBar = %TextureProgressBar_AvatarLoading
@onready var avatar_loading_portrait: TextureProgressBar = $ColorRect/SafeMarginContainer/Panel/MarginContainer/HBoxContainer/Control_info/ScrollContainer/VBoxContainer/Control_Avatar/TextureProgressBar_AvatarLoading
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


var avatar_loading_counter: int = 0
var isOwnPassport: bool = false

func _ready() -> void:
	scroll_container.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	for link in links:
		var new_link_button = PROFILE_LINK_BUTTON.instantiate()
		h_flow_container_links.add_child(new_link_button)
		new_link_button.text = link.label
	_turn_about_editing(false)
	_turn_links_editing(false)

func _on_color_rect_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if event.pressed:
			hide()


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
	if isOwnPassport:
		button_edit_about.show()
		button_edit_links.show()

	
func async_show_profile(profile: DclUserProfile) -> void:
	isOwnPassport = profile == Global.player_identity.get_profile_or_null()
	var profile_dictionary = profile.to_godot_dictionary()
	var loading_id := _set_avatar_loading()
	
	var equipped_button_group = ButtonGroup.new()
	equipped_button_group.allow_unpress = true
	
	for child in h_flow_container_equipped_wearables.get_children():
		child.queue_free()
	
	await avatar_preview_portrait.avatar.async_update_avatar_from_profile(profile)
	await avatar_preview_landscape.avatar.async_update_avatar_from_profile(profile)
	
	var avatar_data = profile_dictionary.get("content", {}).get("avatar", {})
	var wearables_urns = avatar_data.get("wearables", [])

	if not wearables_urns.is_empty():
		# Solicitar los wearables al servidor
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
			else:
				if Emotes.is_emote_default(emote.urn):
					var emote_item = PROFILE_EQUIPPED_ITEM.instantiate()
					h_flow_container_equipped_wearables.add_child(emote_item)
					emote_item.button_group = equipped_button_group
					emote_item.set_base_emote(emote.urn)
	else:
		printerr("Error getting emotes")
	
	_unset_avatar_loading(loading_id)
	


func _on_button_edit_about_pressed() -> void:
	_turn_about_editing(true)


func _on_button_edit_links_pressed() -> void:
	_turn_links_editing(true)


func _turn_about_editing(editing:bool) -> void:
	if editing:
		label_info_description.show()
		label_info_description_2.show()
		h_separator_1.show()
		v_box_container_about_actions.show()
		button_edit_about.hide()
	else:
		label_info_description.hide()
		label_info_description_2.hide()
		h_separator_1.hide()
		v_box_container_about_actions.hide()
		button_edit_about.show()
		
	for child in h_box_container_about_1.get_children():
		child.emit_signal('change_editing', editing)
	for child in grid_container_about.get_children():
		child.emit_signal('change_editing', editing)
	

func _turn_links_editing(editing:bool) -> void:
	button_add_link.disabled = links.size() >= 5
	if h_flow_container_links.get_child_count() > 0 and h_flow_container_links.get_child(h_flow_container_links.get_child_count() - 1) != button_add_link:
		h_flow_container_links.move_child(button_add_link, h_flow_container_links.get_child_count() - 1)
	for child in h_flow_container_links.get_children():
		if child.is_class("ProfileLinkButton"):
			print("CHANGE EDITING")
			child.emit_signal('change_editing', editing)
	if editing:
		button_add_link.show()
		label_editing_links.show()
		v_box_container_links_actions.show()
		button_edit_links.hide()
	else:
		button_add_link.hide()
		label_editing_links.hide()
		v_box_container_links_actions.hide()
		button_edit_links.show()


func _on_button_about_cancel_pressed() -> void:
	_turn_about_editing(false)


func _on_button_links_cancel_pressed() -> void:
	_turn_links_editing(false)
