extends Control

const PROFILE_EQUIPPED_ITEM = preload("res://src/ui/components/profile/profile_equipped_item.tscn")

@onready var h_box_container_about_1: HBoxContainer = %HBoxContainer_About1
@onready var label_no_links: Label = %Label_NoLinks
@onready var label_editing_links: Label = %Label_EditingLinks
@onready var scroll_container: ScrollContainer = %ScrollContainer
@onready var h_flow_container_about: HFlowContainer = %HFlowContainer_About
@onready var avatar_preview_portrait: AvatarPreview = %AvatarPreviewPortrait
@onready var avatar_preview_landscape: AvatarPreview = %AvatarPreviewLandscape
@onready var avatar_loading_landscape: TextureProgressBar = %TextureProgressBar_AvatarLoading
@onready var avatar_loading_portrait: TextureProgressBar = $ColorRect/SafeMarginContainer/Panel/MarginContainer/HBoxContainer/Control_info/ScrollContainer/VBoxContainer/Control_Avatar/TextureProgressBar_AvatarLoading
@onready var button_edit_about: Button = %Button_EditAbout
@onready var button_edit_links: Button = %Button_EditLinks
@onready var h_flow_container_equipped_wearables: HFlowContainer = %HFlowContainer_EquippedWearables

var avatar_loading_counter: int = 0
var isOwnPassport: bool = false

func _ready() -> void:
	scroll_container.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_on_button_edit_about_toggled(false)
	_on_button_edit_links_toggled(false) 


func _on_button_edit_about_toggled(toggled_on: bool) -> void:
	for child in h_box_container_about_1.get_children():
		child.emit_signal('change_editing', toggled_on)
	for child in h_flow_container_about.get_children():
		child.emit_signal('change_editing', toggled_on)


func _on_button_edit_links_toggled(toggled_on: bool) -> void:
	if toggled_on:
		label_editing_links.show()
	else:
		label_editing_links.hide()


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
	
