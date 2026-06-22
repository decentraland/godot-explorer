extends Control

signal set_new_emotes(emotes_run: PackedStringArray)
signal emote_grid_selected(emote_name: String)
signal emote_equipped(equipped: bool)

const EMOTE_SQUARE_ITEM = preload("res://src/ui/components/organisms/emotes/emote_square_item.tscn")

@export var avatar: Avatar = null:
	set(new_value):
		avatar = new_value
		avatar.avatar_loaded.connect(self._on_avatar_loaded)

var last_equipped_emote_urn: String = ""
var avatar_emote_items: Array[EmoteEditorItem] = []
var all_emote_items: Array[EmoteItemUi] = []
var current_selected_index: int = -1

var _currently_selected_emote_item: EmoteItemUi = null
var _equipped_emote_urns: PackedStringArray = []
var _only_collectibles: bool = false
var _ios_marketplace_section: MarketplaceRecommendedSection = null

@onready var container_avatar_emotes = %VBoxContainer_AvatarEmotes
@onready var container_all_emotes = %GridContainer_Emotes
@onready var control_no_emotes = %Control_NoEmotes
@onready var button_group_avatar_emotes = ButtonGroup.new()
@onready var button_group_all_emotes = ButtonGroup.new()
@onready var scroll_container_grid: ScrollContainer = %ScrollContainer_Grid
@onready var inner_margin_container: MarginContainer = %InnerMarginContainer
@onready var emote_grid_outter_margin_container: MarginContainer = %EmoteGridOutterMarginContainer
@onready var external_margin_container: MarginContainer = %ExternalMarginContainer
@onready var outter_margin_container: MarginContainer = %OutterMarginContainer


func _ready():
	var first_button: BaseButton = null
	for child in container_avatar_emotes.get_children():
		if child is EmoteEditorItem:
			if first_button == null:
				first_button = child
			child.button_group = button_group_avatar_emotes
			child.use_equipped_border = true
			var index = avatar_emote_items.size()
			child.select_emote.connect(self._on_emote_editor_item_select_emote.bind(index))
			child.clear_emote.connect(self._on_emote_editor_item_clear_emote.bind(index))
			avatar_emote_items.push_back(child)

	first_button.set_pressed(true)
	current_selected_index = 0

	button_group_all_emotes.allow_unpress = false

	_setup_ios_marketplace_section()
	_async_load_emotes()


func _setup_ios_marketplace_section():
	if not Iap.is_available():
		return

	_ios_marketplace_section = get_node_or_null("%MarketplaceRecommendedSection")
	if _ios_marketplace_section == null:
		return
	# Surface purchaseable emotes at the TOP of the list, above the owned-emotes grid, for
	# discoverability (#2299) — mirrors the wearable carousel move in backpack.gd. The
	# section is the last child of VBoxContainer_EmotesAndSuggestions; move it to the front.
	var section_parent := _ios_marketplace_section.get_parent()
	if section_parent:
		section_parent.move_child(_ios_marketplace_section, 0)
	_ios_marketplace_section.item_selected.connect(_on_marketplace_emote_selected)
	_ios_marketplace_section.update_category("emotes")


func async_set_only_collectibles(new_state: bool):
	_only_collectibles = new_state
	await _async_load_emotes()


func _add_default_emotes():
	for emote_urn in Emotes.DEFAULT_EMOTE_NAMES.keys():
		var emote_item: EmoteItemUi = EMOTE_SQUARE_ITEM.instantiate()
		emote_item.button_group = button_group_all_emotes
		emote_item.async_load_from_urn(emote_urn)
		emote_item.play_emote.connect(self._on_emote_item_play_emote.bind(emote_item))
		emote_item.emote_name_ready.connect(self.emote_grid_selected.emit)
		container_all_emotes.add_child(emote_item)
		all_emote_items.push_back(emote_item)


func _async_load_remote_emotes():
	var remote_emotes = await WearableRequest.async_request_all_emotes()
	var emote_new := {}
	if remote_emotes != null:
		remote_emotes.elements.sort_custom(func(a, b): return a.transferet_at > b.transferet_at)
		# NEW tag (#2300): count owned copies per item urn, then evaluate against the persisted
		# per-wallet snapshot (shared with the wearable grid, no endpoint timestamps).
		var counts := {}
		for emote in remote_emotes.elements:
			var item_urn := Backpack.newtag_item_urn(emote.urn, emote.token_id)
			counts[item_urn] = int(counts.get(item_urn, 0)) + 1
		var wallet := ""
		if Global.player_identity != null:
			wallet = Global.player_identity.get_address_str().to_lower()
		emote_new = Backpack.newtag_evaluate("emote", wallet, counts)
		var count := 0
		for emote in remote_emotes.elements:
			var emote_item: EmoteItemUi = EMOTE_SQUARE_ITEM.instantiate()
			emote_item.button_group = button_group_all_emotes
			emote_item.async_load_from_urn(emote.urn)
			emote_item.play_emote.connect(self._on_emote_item_play_emote.bind(emote_item))
			emote_item.emote_name_ready.connect(self.emote_grid_selected.emit)
			container_all_emotes.add_child(emote_item)
			# Tag emotes whose owned count grew vs the snapshot (#2300).
			var item_urn := Backpack.newtag_item_urn(emote.urn, emote.token_id)
			emote_item.set_new_badge(bool(emote_new.get(item_urn, false)))
			all_emote_items.push_back(emote_item)
			count += 1
			if count % 10 == 0:
				await get_tree().process_frame

	# Surface the most-recently-obtained owned emotes from the fast marketplace API
	# (added only if not already listed via inject_owned_emote's dedupe), so a just-
	# bought emote shows immediately instead of waiting for the catalyst lambda above.
	for urn in await MarketplaceTracker.async_fetch_recent_owned("emote"):
		inject_owned_emote(urn)

	if not _only_collectibles:
		_add_default_emotes()
	_update_grid_equipped_state()


func _async_load_emotes():
	# Clear
	for child in container_all_emotes.get_children():
		container_all_emotes.remove_child(child)
		child.queue_free()

	all_emote_items.clear()

	await _async_load_remote_emotes()
	_update_empty_state()
	_sync_grid_selection()


## Injects a single just-purchased owned emote at the front of the grid, mirroring
## _async_load_remote_emotes' per-item setup. Called by the backpack when the
## MarketplaceTracker detects an emote arrival, so it shows immediately instead of
## waiting for the catalyst lambda to catch up. No-op if already listed.
func inject_owned_emote(urn: String) -> void:
	if urn.is_empty():
		return
	for item in all_emote_items:
		if item.emote_urn == urn:
			return
	var emote_item: EmoteItemUi = EMOTE_SQUARE_ITEM.instantiate()
	emote_item.button_group = button_group_all_emotes
	# Fire-and-forget before add_child (same as the remote load): the await inside
	# resumes only after the item is in the tree and its @onready nodes exist.
	emote_item.async_load_from_urn(urn)
	emote_item.play_emote.connect(self._on_emote_item_play_emote.bind(emote_item))
	emote_item.emote_name_ready.connect(self.emote_grid_selected.emit)
	container_all_emotes.add_child(emote_item)
	container_all_emotes.move_child(emote_item, 0)
	# A live/recent arrival (item_arrived or recent-owned) is brand-new (#2300).
	emote_item.set_new_badge(true)
	all_emote_items.push_front(emote_item)
	_update_empty_state()
	_update_grid_equipped_state()


func _update_empty_state():
	var is_empty := all_emote_items.is_empty()
	if control_no_emotes != null:
		control_no_emotes.visible = is_empty
	container_all_emotes.visible = not is_empty
	if _ios_marketplace_section:
		_ios_marketplace_section.visible = not is_empty


func _on_avatar_loaded():
	_equipped_emote_urns = avatar.avatar_data.get_emotes()

	for i in range(avatar_emote_items.size()):
		# get_emotes() always returns 10 emotes, but just in case
		if i >= _equipped_emote_urns.size():
			# Set default or
			continue

		var emote_editor_item: EmoteEditorItem = avatar_emote_items[i]
		emote_editor_item.async_load_from_urn(_equipped_emote_urns[i], i)  # Forget await

	_update_grid_equipped_state()
	_sync_grid_selection()


func _normalize_emote_urn(urn: String) -> String:
	if Emotes.is_base_emote_urn(urn):
		return Emotes.get_base_emote_id_from_urn(urn)
	return urn


func _on_emote_editor_item_select_emote(_emote_urn: String, index: int):
	if is_instance_valid(avatar) and not _emote_urn.is_empty():
		avatar.async_play_emote(_emote_urn)
	current_selected_index = index
	_sync_grid_selection()


func _on_emote_item_play_emote(_emote_urn: String, emote_item: EmoteItemUi):
	if emote_item == _currently_selected_emote_item:
		_on_emote_item_equip_emote(not emote_item._is_equipped, _emote_urn, emote_item)
		return
	_currently_selected_emote_item = emote_item
	avatar.async_play_emote(_emote_urn)
	emote_grid_selected.emit(emote_item.emote_name)
	var normalized_urn := _normalize_emote_urn(_emote_urn)
	for i in range(_equipped_emote_urns.size()):
		if _normalize_emote_urn(_equipped_emote_urns[i]) == normalized_urn:
			current_selected_index = i
			avatar_emote_items[i].set_pressed(true)
			return


func _on_emote_item_equip_emote(equip: bool, _emote_urn: String, emote_item: EmoteItemUi):
	if not equip:
		_clear_slot(current_selected_index)
		emote_item.set_pressed(true)
		emote_equipped.emit(false)
		return
	var emote_urns = avatar.avatar_data.get_emotes()
	emote_urns[current_selected_index] = _emote_urn
	avatar.avatar_data.set_emotes(emote_urns)
	set_new_emotes.emit(emote_urns)
	last_equipped_emote_urn = _emote_urn
	_on_avatar_loaded()
	emote_equipped.emit(true)


func _on_emote_editor_item_clear_emote(index: int):
	_clear_slot(index)


func _clear_slot(index: int) -> void:
	if index < 0 or index >= _equipped_emote_urns.size():
		return
	_equipped_emote_urns[index] = ""
	var emote_urns = avatar.avatar_data.get_emotes()
	emote_urns[index] = ""
	avatar.avatar_data.set_emotes(emote_urns)
	set_new_emotes.emit(emote_urns)
	# Update the VBox slot display without re-emitting clear_emote
	avatar_emote_items[index].set_empty()
	_update_grid_equipped_state()
	_sync_grid_selection()


func _update_grid_equipped_state():
	var normalized_equipped: Array[String] = []
	for urn in _equipped_emote_urns:
		var normalized := _normalize_emote_urn(urn)
		if not normalized.is_empty():
			normalized_equipped.append(normalized)
	for emote_item in all_emote_items:
		if emote_item is EmoteItemUi:
			var item_urn := _normalize_emote_urn(emote_item.emote_urn)
			emote_item.set_equipped(not item_urn.is_empty() and item_urn in normalized_equipped)


func _sync_grid_selection():
	if current_selected_index < 0 or current_selected_index >= _equipped_emote_urns.size():
		return
	var selected_urn = _normalize_emote_urn(_equipped_emote_urns[current_selected_index])
	if selected_urn.is_empty():
		# Empty slot — unpress the currently pressed grid item (if any)
		for emote_item in all_emote_items:
			if emote_item is EmoteItemUi and emote_item.button_pressed:
				emote_item.set_pressed(false)
		return
	for emote_item in all_emote_items:
		if emote_item is EmoteItemUi:
			if _normalize_emote_urn(emote_item.emote_urn) == selected_urn:
				emote_item.set_pressed(true)
				_currently_selected_emote_item = emote_item
				if scroll_container_grid != null:
					_scroll_to_item_with_margin(emote_item, 20)
				if not emote_item.emote_name.is_empty():
					emote_grid_selected.emit(emote_item.emote_name)
				break


func _scroll_to_item_with_margin(item: Control, margin: float) -> void:
	var item_top := item.get_global_rect().position.y
	var item_bottom := item_top + item.size.y
	var scroll_top := scroll_container_grid.get_global_rect().position.y
	var scroll_bottom := scroll_top + scroll_container_grid.size.y

	if item_top < scroll_top + margin:
		var offset := scroll_top + margin - item_top
		scroll_container_grid.scroll_vertical -= int(offset)
	elif item_bottom > scroll_bottom - margin:
		var offset := item_bottom - (scroll_bottom - margin)
		scroll_container_grid.scroll_vertical += int(offset)


func _on_visibility_changed() -> void:
	if not is_node_ready():
		return
	if scroll_container_grid != null:
		scroll_container_grid.scroll_vertical = 0
	if is_visible_in_tree() and _ios_marketplace_section:
		_ios_marketplace_section.refresh()


func on_narrow(is_narrow: bool) -> void:
	emote_grid_outter_margin_container.custom_minimum_size.x = 274.0 if is_narrow else 494.0


func _on_landscape() -> void:
	outter_margin_container.add_theme_constant_override("margin_right", 48)
	outter_margin_container.add_theme_constant_override("margin_left", 60)
	inner_margin_container.add_theme_constant_override("margin_right", -20)
	inner_margin_container.add_theme_constant_override("margin_left", -20)
	emote_grid_outter_margin_container.add_theme_constant_override("margin_top", 0)
	external_margin_container.add_theme_constant_override("margin_top", 0)
	container_all_emotes.columns = 2
	if _ios_marketplace_section:
		_ios_marketplace_section.set_columns(2)
	for emote_item in avatar_emote_items:
		emote_item.custom_minimum_size = Vector2(138, 138)


func _on_marketplace_emote_selected(urn: String, emote_name: String):
	if is_instance_valid(avatar) and not urn.is_empty():
		avatar.async_play_emote(urn)
	emote_grid_selected.emit(emote_name)
