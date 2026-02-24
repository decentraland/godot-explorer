extends VBoxContainer

signal emote_pressed(urn: String)
signal stop_emote

const PROFILE_EQUIPPED_ITEM = preload("res://src/ui/components/profile/profile_equipped_item.tscn")

@onready var h_box_container_equipped_wearables: HBoxContainer = %HBoxContainer_EquippedWearables
@onready
var scroll_container_equipped_wearables: ScrollContainer = %ScrollContainer_EquippedWearables


func async_refresh(profile: DclUserProfile) -> void:
	var equipped_button_group = ButtonGroup.new()
	equipped_button_group.allow_unpress = true

	for child in h_box_container_equipped_wearables.get_children():
		child.queue_free()

	var profile_dictionary = profile.to_godot_dictionary()
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
				h_box_container_equipped_wearables.add_child(wearable_item)
				wearable_item.button_group = equipped_button_group
				wearable_item.async_set_item(wearable_definition)
			else:
				printerr("Error getting wearable: ", wearable_urn)
	else:
		printerr("Error getting wearables")

	var emotes = avatar_data.get("emotes", [])

	if not emotes.is_empty():
		scroll_container_equipped_wearables.show()
		for emote in emotes:
			var emote_urn = emote.urn
			if not emote_urn.begins_with("urn") and Emotes.is_emote_default(emote_urn):
				emote_urn = Emotes.get_base_emote_urn(emote_urn)

			var emote_definition: DclItemEntityDefinition = Global.content_provider.get_wearable(
				emote_urn
			)
			if emote_definition != null:
				var emote_item = PROFILE_EQUIPPED_ITEM.instantiate()
				h_box_container_equipped_wearables.add_child(emote_item)
				emote_item.button_group = equipped_button_group
				emote_item.async_set_item(emote_definition)
				emote_item.set_as_emote(emote.urn)
				emote_item.emote_pressed.connect(func(urn): emote_pressed.emit(urn))
				emote_item.stop_emote.connect(func(): stop_emote.emit())
	else:
		scroll_container_equipped_wearables.hide()
