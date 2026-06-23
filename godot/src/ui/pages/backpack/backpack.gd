class_name Backpack
extends Control

const WEARABLE_ITEM_INSTANTIABLE = preload(
	"res://src/ui/components/molecules/wearable_item/wearable_item.tscn"
)

const WEARABLE_REFRESH_NOTIFICATION_TYPES = [
	"reward_assignment",
	"reward_in_progress",
	"item_sold",
	"bid_accepted",
]

@export var hide_background: bool = false
@export var hide_navbar: bool = false
@export var default_main_category: String = Wearables.Categories.ALL
## When true, locks the embedded AvatarPreview to rotation only —
## disables mouse-wheel zoom, pinch zoom and pinch vertical-pan. Used by
## the lobby's "Create your avatar" screen.
@export var disable_avatar_zoom: bool = false

var wearable_button_group_per_category: Dictionary = {}
var filtered_data: Array
var current_filter: String = ""
var only_collectibles: bool = false

var base_wearable_request_id: int = -1
var wearable_data: Dictionary = {}

var wearable_filter_buttons: Array[WearableFilterButton] = []
var main_category_selected: String
var request_update_avatar: bool = false  # debounce
var request_show_wearables: bool = false  # debounce

var avatar_wearables_body_shape_cache: Dictionary = {}

var avatar_loading_counter: int = 0
var blacklist_deploy_timer: Timer  # Timer for debounced blacklist changes
var is_loading_profile: bool = false

var _ios_marketplace_section: MarketplaceRecommendedSection = null
var _marketplace_preview_urn: String = ""
var _marketplace_saved_wearables: PackedStringArray = []
var _marketplace_restore_pending: bool = false

var _avatar_update_retries: int = 0
var _is_currently_narrow: bool = false

# "NEW" tag (#2300): item_urn -> current owned count for this load, and item_urn -> bool of
# whether it is tagged new (count grew vs the persisted per-wallet snapshot). No endpoint
# timestamps — see newtag_evaluate.
var _wearable_owned_counts: Dictionary = {}
var _wearable_is_new: Dictionary = {}

@onready var color_carrousel = %ColorCarrousel
@onready var carrousel_separator = %CarrouselSeparator
@onready var grid_container_wearables_list = %GridContainer_WearablesList

@onready var avatar_preview: AvatarPreview = %AvatarPreview
@onready var avatar_loading = %TextureProgressBar_AvatarLoading

@onready var container_main_categories = %HBoxContainer_MainCategories
@onready var container_sub_categories = %HBoxContainer_SubCategories

@onready var vboxcontainer_wearable_selector = %VBoxContainer_WearableSelector

@onready var backpack_loading = %TextureProgressBar_BackpackLoading
@onready var container_backpack = %HBoxContainer_Backpack
@onready var button_back_to_explorer := %Button_BackToExplorer

@onready var wearable_editor = %WearableEditor
@onready var emote_editor = %EmoteEditor
@onready var emote_name_anim = get_node_or_null("%EmoteNameAnim")
@onready var avatar_vfx: AnimatedTextureRect = get_node_or_null("%AvatarVFX")

@onready var container_navbar = %PanelContainer_Navbar
@onready var button_emotes = %Button_Emotes
@onready var button_wearables = %Button_Wearables
@onready var color_rect_background: ColorRect = %ColorRect_Background
@onready var texture_rect_background: TextureRect = %TextureRect_Background
@onready var filter_menu := %FilterMenu
@onready var filter_indicator := %FilterIndicator
@onready var subcategories_container := %SubcategoriesContainer
@onready var subcategories_separator := %SubcategoriesSeparator
@onready var maincategories_container := %MainCategoriesContainer
@onready var filters_menu_checkbox := %CheckBox_OnlyCollectibles
@onready var scroll_container_items: ScrollContainer = %ScrollContainer_Items
@onready var hseparator_extra_space: HSeparator = %HSeparator_ExtraSpace
@onready var hseparator_extra_space_b: HSeparator = %HSeparator_ExtraSpaceB
@onready var hseparator_size_maintainer: HSeparator = get_node_or_null("%HSeparator_SizeMaintainer")
@onready var control_left_bar: Control = get_node_or_null("%Control_LeftBar")
@onready var canary_container: Control = get_node_or_null("%ControlContainer_Canary")
@onready var canary_content: Control = get_node_or_null("%ControlContent_Canary")
@onready var size_canary: Control = get_node_or_null("%HBoxContainer_SizeCanary")
@onready var margin_container_no_items: MarginContainer = %MarginContainer_NoItems

# "NEW" tag (#2300) session state. Per category ("wearable"/"emote"): the baseline snapshot
# (item_urn -> count) each item must exceed to be tagged new, captured once per app session
# from the persisted config, plus whether it has been captured. Static so they survive the
# backpack being freed/recreated on orientation switches (rotation must not wipe the tags
# mid-session).
static var _newtag_session_baseline: Dictionary = {}
static var _newtag_session_captured: Dictionary = {}
# Items that arrived LIVE this session (a marketplace purchase). Shape { category: { urn: true } }.
# OR-ed into every newtag_evaluate so the tag survives later full reloads (which re-run evaluate
# and would otherwise clear a per-item flag). Needed because on a fresh install the first load is
# empty, deferring baseline capture to the arrival itself — the count diff then can never tag it.
# Static (survives the page being recreated) and session-only (never persisted; cleared on app
# restart, where the persisted snapshot takes over).
static var _newtag_forced_new: Dictionary = {}


# gdlint:ignore = async-function-name
func _ready():
	main_category_selected = default_main_category
	UiSounds.install_audio_recusirve(self)
	color_rect_background.visible = !hide_background
	texture_rect_background.visible = !hide_background
	for category in Wearables.Categories.ALL_CATEGORIES:
		var button_group = ButtonGroup.new()
		button_group.allow_unpress = _can_unequip(category)
		wearable_button_group_per_category[category] = button_group

	if hide_navbar:
		container_navbar.hide()
		# The lobby/FTUE "Create your avatar" flow reuses this backpack but must not
		# surface the IAP credits affordances (#2303): hide the credits balance here and
		# skip the marketplace suggestions setup below (see _setup_ios_marketplace_section).
		var credits_button := get_node_or_null("%Button_Credits")
		if credits_button:
			credits_button.hide()

	if size_canary != null:
		size_canary.show()

	if disable_avatar_zoom:
		avatar_preview.can_drag = false

	emote_editor.avatar = avatar_preview.avatar
	emote_editor.set_new_emotes.connect(self._on_set_new_emotes)
	emote_editor.emote_equipped.connect(self._on_emote_equipped)
	wearable_editor.show()
	emote_editor.hide()
	filter_menu.hide()
	filter_indicator.hide()

	container_backpack.hide()
	backpack_loading.show()
	button_back_to_explorer.hide()

	color_carrousel.hide()
	carrousel_separator.hide()
	subcategories_container.show()
	subcategories_separator.show()
	maincategories_container.show()
	hseparator_extra_space.hide()
	hseparator_extra_space_b.show()

	# Setup blacklist change timer
	blacklist_deploy_timer = Timer.new()
	blacklist_deploy_timer.wait_time = 5.0
	blacklist_deploy_timer.one_shot = true
	blacklist_deploy_timer.timeout.connect(self._on_blacklist_deploy_timer_timeout)
	add_child(blacklist_deploy_timer)

	# Connect to blacklist changes
	Global.social_blacklist.blacklist_changed.connect(self._on_blacklist_changed)

	for wearable_filter_button in container_main_categories.get_children():
		if wearable_filter_button is WearableFilterButton:
			wearable_filter_button.filter_type.connect(self._on_main_category_filter_type)

	for wearable_filter_button in container_sub_categories.get_children():
		if wearable_filter_button is WearableFilterButton:
			wearable_filter_button.filter_type.connect(self._on_wearable_filter_button_filter_type)
			wearable_filter_buttons.push_back(wearable_filter_button)

	# NEW tag (#2300): owned counts are rebuilt below from the owned list, then evaluated
	# against the persisted per-wallet snapshot.
	_wearable_owned_counts.clear()

	# Surface the most-recently-obtained owned wearables from the fast marketplace API
	# first (added only if not already listed), so an item just bought on the web shows
	# up immediately instead of waiting for the catalyst lambda below (which lags
	# minutes). Augments the lambda list — never the sole source.
	var fast_owned_urns: Array = []
	for urn in await MarketplaceTracker.async_fetch_recent_owned("wearable"):
		if not wearable_data.has(urn):
			wearable_data[urn] = null
		# Surfaced before the lambda; counted as one below if the lambda doesn't list it yet.
		fast_owned_urns.append(urn)

	# Load all remote wearables that you own...
	var remote_wearables = await WearableRequest.async_request_all_wearables()
	if remote_wearables != null:
		remote_wearables.elements.sort_custom(func(a, b): return a.transferet_at > b.transferet_at)
		for wearable_item in remote_wearables.elements:
			# The lambda yields the token-instance urn; collapse to the ITEM urn so it
			# dedupes against the recent-owned API / live inject (see _to_item_urn).
			var item_urn := _to_item_urn(wearable_item.urn, wearable_item.token_id)
			wearable_data[item_urn] = null
			# Count owned token instances per item for the NEW tag (#2300).
			_wearable_owned_counts[item_urn] = int(_wearable_owned_counts.get(item_urn, 0)) + 1
	# Fast-API items the lambda hasn't listed yet (just bought) count as one.
	for urn in fast_owned_urns:
		if not _wearable_owned_counts.has(urn):
			_wearable_owned_counts[urn] = 1
	# Evaluate NEW tags only when the owned list actually loaded, so an early/transient/failed
	# load never seeds a bogus baseline.
	if remote_wearables != null:
		_wearable_is_new = newtag_evaluate(
			"wearable", _current_wallet_lower(), _wearable_owned_counts
		)

	# Dev/testing: inject fake-owned wearables from deeplink (see FORCE_DEEPLINK in global.gd).
	for fake_urn in Global.deep_link_obj.fake_owned_wearables:
		if not wearable_data.has(fake_urn):
			wearable_data[fake_urn] = null
			print("[BACKPACK] Injected fake-owned wearable: ", fake_urn)

	# Add base wearables last
	for wearable_id in Wearables.BASE_WEARABLES:
		var key = Wearables.get_base_avatar_urn(wearable_id)
		if not wearable_data.has(key):
			wearable_data[key] = null

	var promise = Global.content_provider.fetch_wearables(
		wearable_data.keys(), Global.realm.get_profile_content_url()
	)
	await PromiseUtils.async_all(promise)

	for wearable_id in wearable_data:
		var wearable = Global.content_provider.get_wearable(wearable_id)
		wearable_data[wearable_id] = wearable
		if wearable == null:
			printerr("Error loading wearable_id ", wearable_id)
	_update_visible_categories()

	request_update_avatar = true

	container_backpack.show()
	backpack_loading.hide()

	_setup_ios_marketplace_section()

	request_show_wearables = true

	# Listen for notifications that may indicate new wearables (e.g. rewards)
	NotificationsManager.new_notifications.connect(self._on_new_notifications)

	# Refresh the inventory live when a marketplace purchase is detected as owned
	# (MarketplaceTracker polls for it after returning from the web checkout).
	MarketplaceTracker.item_arrived.connect(self._on_item_arrived)

	# responsive
	if get_window() != null:
		get_window().size_changed.connect(self._on_size_changed)
	_on_size_changed.call_deferred()


func _on_size_changed():
	var window_size: Vector2i = DisplayServer.window_get_size()
	var portrait = window_size.x < window_size.y
	var right_editor_container: MarginContainer = %RightEditorContainer
	if portrait:
		right_editor_container.add_theme_constant_override("margin_top", 0)
		right_editor_container.add_theme_constant_override("margin_left", 0)
		right_editor_container.add_theme_constant_override("margin_right", 0)
		right_editor_container.add_theme_constant_override("margin_bottom", 0)
	else:
		right_editor_container.add_theme_constant_override("margin_top", 10)
		right_editor_container.add_theme_constant_override("margin_left", 20)
		right_editor_container.add_theme_constant_override("margin_right", 20)
		right_editor_container.add_theme_constant_override("margin_bottom", 10)
		emote_editor._on_landscape()
		color_carrousel.on_landscape()

	_update_grid_columns()


func _update_grid_columns() -> void:
	if not is_node_ready():
		return
	if grid_container_wearables_list == null or emote_editor == null:
		return

	# Check if the Wearables button gets clipped by its container
	var is_narrow := _is_wearables_button_clipped()
	var columns := 2 if is_narrow else 3

	var window_size: Vector2i = DisplayServer.window_get_size()
	var is_portrait := window_size.x < window_size.y
	grid_container_wearables_list.columns = columns
	if _ios_marketplace_section:
		_ios_marketplace_section.set_columns(columns)
	#if emote_editor.container_all_emotes != null:
	emote_editor.container_all_emotes.columns = columns if is_portrait else columns - 1

	if hseparator_size_maintainer != null:
		hseparator_size_maintainer.custom_minimum_size.x = 410.0 if is_narrow else 630.0

	emote_editor.on_narrow(is_narrow)


func _is_wearables_button_clipped() -> bool:
	if canary_container == null or canary_content == null:
		return _is_currently_narrow

	# Compare canary container width vs canary content width
	# These nodes are not affected by column changes, providing stable measurement
	var is_narrow := canary_container.size.x < canary_content.size.x
	_is_currently_narrow = is_narrow
	return is_narrow


func _update_visible_categories():
	#var showed_subcategories: int = 0
	var first_wearable_filter_button: WearableFilterButton = null
	for wearable_filter_button: WearableFilterButton in wearable_filter_buttons:
		var category = wearable_filter_button.get_category_name()
		var filter_categories: Array = Wearables.Categories.MAIN_CATEGORIES.get(
			main_category_selected
		)
		var category_is_visible: bool = (
			filter_categories != null
			and filter_categories.has(category)
			and not (
				main_category_selected == Wearables.Categories.ALL
				and category == Wearables.Categories.ALL
			)
		)
		#prints("BUTTON: ", category, category_is_visible, main_category_selected, filter_categories)
		wearable_filter_button.visible = category_is_visible
		if category_is_visible:
			#showed_subcategories += 1
			if first_wearable_filter_button == null:
				first_wearable_filter_button = wearable_filter_button

	var has_visible := first_wearable_filter_button != null
	subcategories_container.visible = has_visible
	subcategories_separator.visible = has_visible
	hseparator_extra_space.visible = !has_visible
	if main_category_selected == Wearables.Categories.ALL:
		hseparator_extra_space.hide()
	if first_wearable_filter_button:
		first_wearable_filter_button.set_pressed(true)
		_on_wearable_filter_button_filter_type(first_wearable_filter_button.get_category_name())
	elif main_category_selected == Wearables.Categories.ALL:
		_on_wearable_filter_button_filter_type(Wearables.Categories.ALL)


func _on_set_new_emotes(emotes_urns: PackedStringArray):
	Global.player_identity.get_mutable_avatar().set_emotes(emotes_urns)
	# Don't trigger request_update_avatar - emotes are loaded separately by the emote controller
	# and don't require reloading the avatar mesh/wearables


func _physics_process(_delta):
	if request_update_avatar:
		request_update_avatar = false
		_async_update_avatar()

	if request_show_wearables:
		request_show_wearables = false
		_show_wearables()


func _set_avatar_loading() -> int:
	avatar_preview.hide()
	avatar_loading.show()
	avatar_loading_counter += 1
	return avatar_loading_counter


func _unset_avatar_loading(current: int):
	if current != avatar_loading_counter:
		return
	avatar_loading.hide()
	avatar_preview.show()


func _async_update_avatar():
	var mutable_profile = Global.player_identity.get_mutable_profile()
	var mutable_avatar = Global.player_identity.get_mutable_avatar()
	if mutable_profile == null or mutable_avatar == null:
		# Profile not ready - keep retrying, connection_quality_monitor handles modal
		_avatar_update_retries += 1
		var delay := minf(1.0 * _avatar_update_retries, 5.0)  # Cap at 5 seconds
		await get_tree().create_timer(delay).timeout
		request_update_avatar = true
		return
	_avatar_update_retries = 0

	# If marketplace preview is active, rebuild the preview with updated colors
	if not _marketplace_preview_urn.is_empty():
		var wearable = Global.content_provider.get_wearable(_marketplace_preview_urn)
		if wearable != null:
			_async_marketplace_preview_equip(_marketplace_preview_urn, wearable)
			return

	mutable_profile.set_avatar(mutable_avatar)

	var loading_id := _set_avatar_loading()
	await avatar_preview.avatar.async_update_avatar_from_profile(
		Global.player_identity.get_mutable_profile()
	)
	_unset_avatar_loading(loading_id)


func _load_filtered_data(filter: String):
	if Global.player_identity.get_mutable_avatar() == null:
		return

	filtered_data = []
	current_filter = filter
	for wearable_id in wearable_data:
		var wearable = wearable_data[wearable_id]
		if wearable != null:
			var is_filter_all = filter == "all"
			var is_filter_all_extras = filter == "all_extras"
			var is_filter_chest = filter == "chest"
			if (
				(wearable.get_category() == filter or is_filter_all)
				or (
					is_filter_all_extras
					and wearable.get_category() in Wearables.Categories.ALL_EXTRAS_CATEGORIES
				)
				or (
					is_filter_chest
					and wearable.get_category() in Wearables.Categories.CHEST_CATEGORIES
				)
			):
				var is_body_shape = wearable.get_category() == "body_shape"
				var is_equipable = Wearables.can_equip(
					wearable, Global.player_identity.get_mutable_avatar().get_body_shape()
				)
				var is_base_wearable = Wearables.is_base_wearable(wearable_id)
				var can_use = (
					(is_equipable and (!is_base_wearable or !only_collectibles))
					or (is_body_shape and !is_filter_all)
				)
				if can_use:
					filtered_data.push_back(wearable_id)

	request_show_wearables = true


func _can_unequip(category: String) -> bool:
	return (
		category != Wearables.Categories.BODY_SHAPE
		and category != Wearables.Categories.EYES
		and category != Wearables.Categories.MOUTH
	)


func _show_wearables():
	scroll_container_items.scroll_vertical = 0
	for child in grid_container_wearables_list.get_children():
		child.queue_free()

	var has_items = not filtered_data.is_empty()
	margin_container_no_items.visible = not has_items
	grid_container_wearables_list.visible = has_items

	for wearable_id in filtered_data:
		var wearable_item = WEARABLE_ITEM_INSTANTIABLE.instantiate()
		var wearable = wearable_data[wearable_id]
		grid_container_wearables_list.add_child(wearable_item)
		wearable_item.button_group = wearable_button_group_per_category.get(wearable.get_category())
		wearable_item.async_set_wearable(wearable)
		var dbg_is_new := _is_wearable_new(wearable_id)
		if dbg_is_new:
			print("[NEWTAGDBG] render NEW badge urn=", wearable_id)
		wearable_item.set_new_badge(dbg_is_new)

		# Connect signals
		wearable_item.equip.connect(self._on_wearable_equip.bind(wearable_id))
		wearable_item.unequip.connect(self._on_wearable_unequip.bind(wearable_id))

		# Check if the item is equipped
		var is_wearable_pressed = (
			Global.player_identity.get_mutable_avatar().get_wearables().has(wearable_id)
			or Global.player_identity.get_mutable_avatar().get_body_shape() == wearable_id
		)
		wearable_item.set_pressed_no_signal(is_wearable_pressed)
		wearable_item.set_equiped(is_wearable_pressed)


func _setup_ios_marketplace_section():
	# Not shown in the lobby/FTUE "Create your avatar" flow (same context that hides the
	# navbar): only the in-world backpack surfaces purchaseable suggestions (#2303).
	if hide_navbar:
		return
	if not Iap.is_available():
		return

	_ios_marketplace_section = get_node_or_null("%MarketplaceRecommendedSection")
	if _ios_marketplace_section == null:
		return
	# Surface purchaseable items at the TOP of the items list, above the owned-wearables
	# grid, for better discoverability (#2299). The section is the last child of
	# VBoxContainer_ItemsAndSuggestions in the scene; move it to the front.
	var section_parent := _ios_marketplace_section.get_parent()
	if section_parent:
		section_parent.move_child(_ios_marketplace_section, 0)
	_ios_marketplace_section.item_equip.connect(_async_on_marketplace_equip)
	_ios_marketplace_section.item_unequip.connect(_on_marketplace_unequip)


func _on_main_category_filter_type(type: String):
	_marketplace_preview_restore()
	main_category_selected = type
	_update_visible_categories()


func _on_wearable_filter_button_filter_type(type):
	_marketplace_preview_restore()
	_load_filtered_data(type)
	avatar_preview.focus_camera_on(type)
	if _ios_marketplace_section:
		_ios_marketplace_section.update_category(type)
	var color_name := "%s Color" % type.to_pascal_case()
	color_carrousel.set_title(color_name)

	var mutable_avatar = Global.player_identity.get_mutable_avatar()
	if mutable_avatar == null:
		return

	var should_hide = false
	if type == Wearables.Categories.BODY_SHAPE:
		color_carrousel.color_type = color_carrousel.ColorTargetType.SKIN
		color_carrousel.set_color(mutable_avatar.get_skin_color())
	elif (
		type == Wearables.Categories.HAIR
		or type == Wearables.Categories.FACIAL_HAIR
		or type == Wearables.Categories.EYEBROWS
	):
		color_carrousel.color_type = color_carrousel.ColorTargetType.HAIR
		color_carrousel.set_color(mutable_avatar.get_hair_color())
	elif type == Wearables.Categories.EYES:
		color_carrousel.color_type = color_carrousel.ColorTargetType.EYES
		color_carrousel.set_color(mutable_avatar.get_eyes_color())
	else:
		should_hide = true

	if should_hide:
		color_carrousel.hide()
		carrousel_separator.hide()
	else:
		color_carrousel.show()
		carrousel_separator.hide()


func _on_wearable_equip(wearable_id: String):
	var desired_wearable = wearable_data.get(wearable_id)
	if desired_wearable == null:
		return
	var category = desired_wearable.get_category()

	if category == Wearables.Categories.BODY_SHAPE:
		var current_body_shape_id: String = (
			Global.player_identity.get_mutable_avatar().get_body_shape()
		)
		var new_body_shape_id := wearable_id
		if current_body_shape_id != new_body_shape_id:
			avatar_wearables_body_shape_cache[current_body_shape_id] = (
				Global.player_identity.get_mutable_avatar().get_wearables().duplicate()
			)

			Global.player_identity.get_mutable_avatar().set_body_shape(new_body_shape_id)
			var default_wearables: Dictionary = Wearables.DefaultWearables.BY_BODY_SHAPES.get(
				new_body_shape_id
			)
			var new_avatar_wearables = avatar_wearables_body_shape_cache.get(new_body_shape_id, [])
			if new_avatar_wearables.is_empty():
				new_avatar_wearables = default_wearables.values()

			Global.player_identity.get_mutable_avatar().set_wearables(
				PackedStringArray(new_avatar_wearables)
			)
	else:
		var new_avatar_wearables: PackedStringArray = (
			Global.player_identity.get_mutable_avatar().get_wearables()
		)
		var to_remove = []
		# Unequip current wearable with category
		for current_wearable_id in new_avatar_wearables:
			# TODO: put the fetch wearable function
			var wearable = wearable_data.get(current_wearable_id)
			if wearable != null and wearable.get_category() == category:
				to_remove.push_back(current_wearable_id)

		for to_remove_id in to_remove:
			var index = new_avatar_wearables.find(to_remove_id)
			new_avatar_wearables.remove_at(index)

		new_avatar_wearables.append(wearable_id)
		Global.player_identity.get_mutable_avatar().set_wearables(new_avatar_wearables)

	request_update_avatar = true


func _on_wearable_unequip(wearable_id: String):
	var desired_wearable = wearable_data.get(wearable_id)
	if desired_wearable == null:
		return
	var category = desired_wearable.get_category()

	if category == Wearables.Categories.BODY_SHAPE:
		# can not unequip a body shape
		return

	var new_avatar_wearables: PackedStringArray = (
		Global.player_identity.get_mutable_avatar().get_wearables()
	)
	var index = new_avatar_wearables.find(wearable_id)
	if index != -1:
		new_avatar_wearables.remove_at(index)

	Global.player_identity.get_mutable_avatar().set_wearables(new_avatar_wearables)
	request_update_avatar = true


func _async_on_marketplace_equip(urn: String):
	if urn.is_empty():
		return
	# Cancel any pending restore immediately so the deferred call doesn't
	# clear_selection while we await the wearable fetch.
	_marketplace_restore_pending = false
	# Fetch wearable definition — use content_provider cache, don't add to wearable_data
	var wearable = Global.content_provider.get_wearable(urn)
	if wearable == null:
		var promise = Global.content_provider.fetch_wearables(
			[urn], Global.realm.get_profile_content_url()
		)
		await PromiseUtils.async_all(promise)
		wearable = Global.content_provider.get_wearable(urn)
		if wearable == null:
			printerr("[Marketplace] Failed to fetch wearable: ", urn)
			return
	_async_marketplace_preview_equip(urn, wearable)


func _on_marketplace_unequip(_urn: String):
	# When switching cards in the ButtonGroup, unequip fires before the new equip.
	# Defer the restore so equip can cancel it if another card takes over.
	_marketplace_restore_pending = true
	_deferred_marketplace_restore.call_deferred()


## Temporarily equips a marketplace wearable for visual preview only.
## Never touches mutable avatar/profile — only updates the local avatar_preview.
func _async_marketplace_preview_equip(urn: String, wearable: DclItemEntityDefinition):
	# Cancel any pending restore from a ButtonGroup switch
	_marketplace_restore_pending = false

	var mutable_avatar = Global.player_identity.get_mutable_avatar()
	if mutable_avatar == null:
		return

	# Save original wearables on first preview
	if _marketplace_preview_urn.is_empty():
		_marketplace_saved_wearables = mutable_avatar.get_wearables().duplicate()

	_marketplace_preview_urn = urn
	var category = wearable.get_category()

	# Build temporary wearable list: replace same category, add new
	var preview_wearables = _marketplace_saved_wearables.duplicate()
	var to_remove = []
	for current_id in preview_wearables:
		var current_wearable = wearable_data.get(current_id)
		if current_wearable != null and current_wearable.get_category() == category:
			to_remove.push_back(current_id)
	for remove_id in to_remove:
		var idx = preview_wearables.find(remove_id)
		if idx != -1:
			preview_wearables.remove_at(idx)
	preview_wearables.append(urn)

	# Create a temporary avatar wire format for preview — don't touch the real one
	var temp_avatar = DclAvatarWireFormat.new()
	temp_avatar.set_body_shape(mutable_avatar.get_body_shape())
	temp_avatar.set_eyes_color(mutable_avatar.get_eyes_color())
	temp_avatar.set_hair_color(mutable_avatar.get_hair_color())
	temp_avatar.set_skin_color(mutable_avatar.get_skin_color())
	temp_avatar.set_wearables(preview_wearables)
	temp_avatar.set_emotes(mutable_avatar.get_emotes())

	var profile = Global.player_identity.get_mutable_profile()
	var avatar_name = profile.get_name() if profile else ""

	var loading_id := _set_avatar_loading()
	await avatar_preview.avatar.async_update_avatar(temp_avatar, avatar_name)
	_unset_avatar_loading(loading_id)


## Restores the avatar preview to the real profile state.
func _marketplace_preview_restore():
	if _marketplace_preview_urn.is_empty():
		return
	_marketplace_restore_pending = false
	_marketplace_preview_urn = ""
	_marketplace_saved_wearables = []
	if _ios_marketplace_section:
		_ios_marketplace_section.clear_selection()
	request_update_avatar = true


## Deferred version — only runs if not cancelled by a new equip.
func _deferred_marketplace_restore():
	if not _marketplace_restore_pending:
		return
	_marketplace_preview_restore()


func _on_button_logout_pressed():
	# Route through the single canonical teardown (kills scenes, closes comms,
	# clears identity, resets realm) instead of just dropping comms.
	Global.sign_out()


func _on_color_picker_panel_pick_color(color: Color):
	match color_carrousel.color_type:
		color_carrousel.ColorTargetType.EYES:
			Global.player_identity.get_mutable_avatar().set_eyes_color(color)
		color_carrousel.ColorTargetType.SKIN:
			Global.player_identity.get_mutable_avatar().set_skin_color(color)
		color_carrousel.ColorTargetType.HAIR:
			Global.player_identity.get_mutable_avatar().set_hair_color(color)

	avatar_preview.avatar.update_colors(
		Global.player_identity.get_mutable_avatar().get_eyes_color(),
		Global.player_identity.get_mutable_avatar().get_skin_color(),
		Global.player_identity.get_mutable_avatar().get_hair_color()
	)
	# NOTE Don't use request_update_avatar here
	# that would make the avatar flash during color picking
	#request_update_avatar = true


func _on_color_set() -> void:
	request_update_avatar = true


func _on_button_wearables_pressed():
	_marketplace_preview_restore()
	avatar_preview.avatar.emote_controller.stop_emote()
	scroll_container_items.scroll_vertical = 0
	wearable_editor.show()
	emote_editor.hide()
	if emote_name_anim != null:
		emote_name_anim.hide()


func _on_button_emotes_pressed():
	_marketplace_preview_restore()
	scroll_container_items.scroll_vertical = 0
	show_emotes()


func show_emotes() -> void:
	avatar_preview.focus_camera_on(Wearables.Categories.BODY_SHAPE)
	wearable_editor.hide()
	emote_editor.show()
	if emote_name_anim != null:
		emote_name_anim.show()


func press_button_emotes() -> void:
	button_emotes.set_pressed_no_signal(true)
	button_wearables.set_pressed_no_signal(false)


func _input(event: InputEvent) -> void:
	if event is InputEventMouseButton:
		if !event.pressed and filter_menu.visible:
			if not filter_menu.get_global_rect().has_point(event.position):
				if not filters_menu_checkbox.get_global_rect().has_point(event.position):
					filters_menu_checkbox.set_pressed(false)


func _on_check_box_only_collectibles_toggled(toggled_on: bool) -> void:
	filter_menu.visible = toggled_on


func _on_collectible_filter_button_toggled(toggled_on: bool) -> void:
	only_collectibles = toggled_on
	emote_editor.async_set_only_collectibles(toggled_on)
	_load_filtered_data(current_filter)
	filter_indicator.visible = toggled_on
	filters_menu_checkbox.set_pressed(false)


func _on_new_notifications(notifications: Array) -> void:
	for notif in notifications:
		var notif_type: String = notif.get("type", "")
		if notif_type in WEARABLE_REFRESH_NOTIFICATION_TYPES:
			_async_refresh_owned_wearables()
			return


func _on_item_arrived(urn: String, category: String) -> void:
	# A marketplace purchase just landed (MarketplaceTracker detected it via the fast
	# marketplace API). Force the matching view + Collectibles filter so it's visible,
	# then inject that exact item directly instead of re-fetching the catalyst lambda,
	# which lags minutes behind. Emotes live in the emote editor, not the wearables
	# grid, so route by category.
	apply_marketplace_arrival_view(category)
	if category == "emote":
		emote_editor.inject_owned_emote(urn)
	else:
		_async_inject_wearable(urn)
	# A marketplace buy consumes credits, but nothing else re-fetches the balance after a
	# (non-IAP) marketplace purchase — so the displayed credits stay stale. Refresh here.
	Iap.async_refresh_balance()


# Force the backpack into the view that surfaces a just-arrived marketplace item:
# wearable → ALL tab; emote → emotes view; both with the Collectibles filter on so the
# purchased NFT shows. Reused by the live arrival handler and by the toast-click path
# (via BackpackResponsive). Also refreshes the relevant list.
func apply_marketplace_arrival_view(category: String) -> void:
	only_collectibles = true
	filter_indicator.visible = true
	emote_editor.async_set_only_collectibles(true)
	if category == "emote":
		show_emotes()
		press_button_emotes()
	else:
		_on_button_wearables_pressed()
		_on_main_category_filter_type(Wearables.Categories.ALL)
		# Reflect ALL as the selected main-category tab; otherwise the tab the user was
		# previously on stays visually highlighted even though ALL is now active.
		for btn in container_main_categories.get_children():
			if btn is WearableFilterButton:
				btn.set_pressed_no_signal(btn.get_category_name() == Wearables.Categories.ALL)


# --- "NEW" tag helpers (#2300) ---
#
# Endpoint-timestamp-free: we keep a per-wallet snapshot of owned item COUNTS (item_urn ->
# count) in the config and tag an item NEW when its current count exceeds the snapshot (a new
# urn, or one extra copy). The first load for a wallet just seeds the snapshot and tags
# nothing. Shared by the wearable grid and the emote grid (category "wearable" / "emote").


func _current_wallet_lower() -> String:
	if Global.player_identity == null:
		return ""
	return Global.player_identity.get_address_str().to_lower()


# Collapses a token-instance urn (…:<itemId>:<tokenId>) to its ITEM urn so multiple copies of
# the same item count together. Static so the emote grid can share it. token_id is the parsed
# tokenId; base/off-chain items have none and pass through unchanged.
static func newtag_item_urn(urn: String, token_id: String) -> String:
	if not token_id.is_empty() and urn.ends_with(":" + token_id):
		return urn.trim_suffix(":" + token_id)
	return urn


# Evaluates the NEW tags for a category from the current owned counts and persists the
# snapshot so tags clear next session. Returns { item_urn: bool }. The comparison baseline is
# captured once per app session per category (static), so it stays stable across the grid
# being rebuilt (filter changes, orientation switches). Persists on every load rather than on
# teardown, so killing the app can't strand a stale snapshot. Returns {} without a wallet or
# with empty counts, so an early/transient load never seeds a bogus baseline.
static func newtag_evaluate(
	category: String, wallet: String, current_counts: Dictionary
) -> Dictionary:
	if wallet.is_empty() or current_counts.is_empty():
		return {}
	var stored: Dictionary = _newtag_stored_for(category)
	if not _newtag_session_captured.get(category, false):
		_newtag_session_captured[category] = true
		# First load this session: the baseline is the previously-persisted snapshot, or — on a
		# wallet's first-ever visit — the current inventory itself (so nothing is tagged).
		if stored.has(wallet):
			_newtag_session_baseline[category] = (stored[wallet] as Dictionary).duplicate()
		else:
			_newtag_session_baseline[category] = current_counts.duplicate()
	# Advance the persisted snapshot to the current counts (next session's baseline).
	_newtag_persist(category, wallet, current_counts)
	var baseline: Dictionary = _newtag_session_baseline.get(category, {})
	var forced: Dictionary = _newtag_forced_new.get(category, {})
	var flags := {}
	var new_urns := []
	for urn in current_counts:
		# NEW when the owned count grew vs the baseline, OR it arrived live this session (a
		# purchase the empty-first-load baseline can't tag). forced survives reloads.
		flags[urn] = int(current_counts[urn]) > int(baseline.get(urn, 0)) or forced.has(urn)
		if flags[urn]:
			new_urns.append(urn)
	return flags


# The persisted { wallet_lower: { item_urn: count } } map for a category.
static func _newtag_stored_for(category: String) -> Dictionary:
	var all: Dictionary = Global.get_config().backpack_owned_counts
	return all.get(category, {})


## Marks an item as NEW for the rest of this app session because it arrived live (a marketplace
## purchase). Static so it survives the page being recreated; OR-ed into newtag_evaluate so a
## later full reload can't clear it. Shared by the wearable grid and the emote editor.
static func newtag_mark_arrived(category: String, urn: String) -> void:
	var per_category: Dictionary = _newtag_forced_new.get(category, {})
	per_category[urn] = true
	_newtag_forced_new[category] = per_category


static func _newtag_persist(category: String, wallet: String, counts: Dictionary) -> void:
	var all: Dictionary = Global.get_config().backpack_owned_counts
	var per_category: Dictionary = all.get(category, {})
	per_category[wallet] = counts.duplicate()
	all[category] = per_category
	Global.get_config().backpack_owned_counts = all
	Global.get_config().save_to_settings_file()


func _is_wearable_new(urn: String) -> bool:
	return bool(_wearable_is_new.get(urn, false))


# Owned collectibles enter wearable_data from two sources with different urn forms: the
# catalyst lambda yields the token-instance urn (…:<itemId>:<tokenId>, from
# individualData[].id) while the fast recent-owned API and the live inject yield the ITEM
# urn (…:<itemId>). Keying by both forms lists the same wearable twice — every item shows
# duplicated. Collapse to the ITEM urn, the canonical form get_wearable/can_equip/the
# avatar profile all use, so the two sources dedupe and equipped collectibles match the
# profile's item urns. token_id is the parsed tokenId; base/off-chain items have none.
func _to_item_urn(urn: String, token_id: String) -> String:
	return newtag_item_urn(urn, token_id)


# gdlint:ignore = async-function-name
func _async_inject_wearable(urn: String) -> void:
	if urn.is_empty() or wearable_data.has(urn):
		return
	var wearable = Global.content_provider.get_wearable(urn)
	if wearable == null:
		var content_url := Global.realm.get_profile_content_url()
		var promise = Global.content_provider.fetch_wearables([urn], content_url)
		await PromiseUtils.async_all(promise)
		wearable = Global.content_provider.get_wearable(urn)
	if wearable == null:
		return

	# A live arrival is a fresh acquisition THIS session, so it must show the NEW tag regardless
	# of the owned-count baseline. On a fresh install the first backpack load is empty (the wallet
	# owns nothing yet), so newtag_evaluate bails on its empty-counts guard and never captures the
	# baseline — deferring it to THIS arrival, which would then seed the item into the baseline and
	# tag it count==baseline (not new). Mark it forced-NEW (survives later reloads), bump its count
	# and re-evaluate so the grid tags it. Mirrors the emote path (inject_owned_emote). (#2300)
	newtag_mark_arrived("wearable", urn)
	_wearable_owned_counts[urn] = int(_wearable_owned_counts.get(urn, 0)) + 1
	_wearable_is_new = newtag_evaluate("wearable", _current_wallet_lower(), _wearable_owned_counts)

	# Insert at the front so the just-arrived wearable shows first in the grid.
	var reordered := {urn: wearable}
	for k in wearable_data:
		if k != urn:
			reordered[k] = wearable_data[k]
	wearable_data = reordered
	var cat: String = wearable.get_category()

	# Reload the current view; if the new item isn't visible under the active filter
	# (different category, or no filter set), switch to its category so it shows.
	if not current_filter.is_empty():
		_load_filtered_data(current_filter)
	if current_filter.is_empty() or not filtered_data.has(urn):
		_load_filtered_data(cat)


func _async_refresh_owned_wearables() -> void:
	var remote_wearables = await WearableRequest.async_request_all_wearables()
	if remote_wearables == null:
		return

	# Untyped Array: content_provider.fetch_wearables() expects an untyped array
	# (like the initial load's wearable_data.keys()). Passing a typed Array[String]
	# makes the Rust binding panic with BadArrayType and crashes the app.
	var new_keys: Array = []
	# Rebuild owned counts from the authoritative full list, then re-evaluate the NEW tags.
	_wearable_owned_counts.clear()
	for wearable_item in remote_wearables.elements:
		# Collapse the lambda's token-instance urn to the ITEM urn (see _to_item_urn) so it
		# dedupes against entries already added by the recent-owned API / live inject.
		var item_urn := _to_item_urn(wearable_item.urn, wearable_item.token_id)
		_wearable_owned_counts[item_urn] = int(_wearable_owned_counts.get(item_urn, 0)) + 1
		if not wearable_data.has(item_urn):
			wearable_data[item_urn] = null
			new_keys.append(item_urn)
	_wearable_is_new = newtag_evaluate("wearable", _current_wallet_lower(), _wearable_owned_counts)

	if new_keys.is_empty():
		return

	var promise = Global.content_provider.fetch_wearables(
		new_keys, Global.realm.get_profile_content_url()
	)
	await PromiseUtils.async_all(promise)

	for wearable_id in new_keys:
		var wearable = Global.content_provider.get_wearable(wearable_id)
		wearable_data[wearable_id] = wearable
		if wearable == null:
			printerr("Error loading new wearable_id ", wearable_id)

	# Show the just-arrived wearables first: rebuild wearable_data with the new keys
	# at the front. Grid order follows wearable_data insertion order (via
	# _load_filtered_data → _show_wearables), and the new keys were appended last.
	var reordered := {}
	for k in new_keys:
		reordered[k] = wearable_data[k]
	for k in wearable_data:
		if not reordered.has(k):
			reordered[k] = wearable_data[k]
	wearable_data = reordered

	# Refresh the current view to show newly available wearables. Fall back to
	# rebuilding the visible categories when no explicit filter is set, so the new
	# item shows up regardless of how the grid was last populated.
	if not current_filter.is_empty():
		_load_filtered_data(current_filter)
	else:
		_update_visible_categories()


func _exit_tree():
	# Clean up timer and disconnect signals
	if blacklist_deploy_timer:
		blacklist_deploy_timer.stop()
		blacklist_deploy_timer.queue_free()

	if NotificationsManager.new_notifications.is_connected(self._on_new_notifications):
		NotificationsManager.new_notifications.disconnect(self._on_new_notifications)

	if MarketplaceTracker.item_arrived.is_connected(self._on_item_arrived):
		MarketplaceTracker.item_arrived.disconnect(self._on_item_arrived)

	if Global.social_blacklist.blacklist_changed.is_connected(self._on_blacklist_changed):
		Global.social_blacklist.blacklist_changed.disconnect(self._on_blacklist_changed)


func _on_blacklist_changed():
	# Don't trigger deployment if we're loading a profile from server
	if is_loading_profile:
		return
	# Reset the timer if it's already running
	blacklist_deploy_timer.stop()
	blacklist_deploy_timer.start()


func _on_blacklist_deploy_timer_timeout():
	# Update the mutable profile with current blacklist before deploying
	Global.player_identity.get_mutable_profile().set_blocked(
		Global.social_blacklist.get_blocked_list()
	)
	Global.player_identity.get_mutable_profile().set_muted(Global.social_blacklist.get_muted_list())
	# Deploy without incrementing version for blacklist changes (ADR-290: no snapshots)
	ProfileService.async_deploy_profile_with_version_control(
		Global.player_identity.get_mutable_profile(), false
	)


func _on_color_carrousel_toggle_color_picker(toggle: bool) -> void:
	if toggle:
		%MarginItemsContainer.hide()
		subcategories_container.hide()
		subcategories_separator.hide()
		maincategories_container.hide()
		hseparator_extra_space.hide()
		hseparator_extra_space_b.hide()
	else:
		%MarginItemsContainer.show()
		subcategories_container.show()
		subcategories_separator.show()
		maincategories_container.show()
		hseparator_extra_space.hide()
		hseparator_extra_space_b.show()


func _on_visibility_changed() -> void:
	if not is_node_ready() or not is_inside_tree():
		return
	if is_visible_in_tree():
		if Global.get_explorer():
			if button_back_to_explorer:
				button_back_to_explorer.hide()
	else:
		# Leaving backpack — restore avatar preview to real state
		_marketplace_preview_restore()


func _on_button_back_to_explorer_pressed() -> void:
	if Global.get_explorer():
		Global.close_menu.emit()
		Global.set_orientation_landscape()


func _on_emote_equipped(equipped: bool) -> void:
	if not equipped:
		return
	Global.send_haptic_feedback(80, 0.5)
	var tween := create_tween()
	tween.tween_property(avatar_preview, "modulate:a", 0.0, 0.2)
	tween.tween_callback(
		func() -> void:
			if avatar_vfx != null:
				avatar_vfx.play()
			var urn: String = emote_editor.last_equipped_emote_urn
			if not urn.is_empty():
				avatar_preview.avatar.async_play_emote(urn)
	)
	tween.tween_property(avatar_preview, "modulate:a", 1.0, 0.3)
