@tool
class_name GuestUpgradeCard
extends MarginContainer

## Screen location for metrics tracking. "discover" auto-differentiates between
## discover_pregame and discover_ingame based on whether explorer is active.
@export_enum("discover", "settings") var shown_in = "discover"
## When true, removes the left and right margins so the card stretches edge to edge.
## Use in settings; leave false in discover where lateral margins are needed.
@export var full_width: bool = false:
	set(value):
		full_width = value
		_apply_full_width()
## Left and right margin (px) applied when full_width is false.
@export var side_margin: int = 48:
	set(value):
		side_margin = value
		_apply_full_width()
## Top margin (px) applied to the settings card while in landscape (in-game). Portrait settings
## gets its top spacing from the settings layout; the tighter landscape layout leaves the card
## flush against the tab buttons, so this restores the separation (issue #2403).
@export var landscape_settings_top_margin: int = 40:
	set(value):
		landscape_settings_top_margin = value
		_apply_top_margin()
## Title font size used while the settings card is in landscape (in-game). Portrait keeps the
## size defined by the scene's LabelSettings (issue #2403 family).
@export var landscape_title_font_size: int = 34:
	set(value):
		landscape_title_font_size = value
		_apply_font_sizes()
## Paragraph font size used while the settings card is in landscape (in-game). See
## landscape_title_font_size.
@export var landscape_paragraph_font_size: int = 24:
	set(value):
		landscape_paragraph_font_size = value
		_apply_font_sizes()

## True once the network check has completed with an authoritative result
## (prevents re-checking every time the parent becomes visible afterwards).
var _upgrade_checked: bool = false
## True while the network check is in flight. Concurrent calls — e.g. the repeated
## orientation_changed emissions Discover fires during a menu transition — must not
## take the cached fast path against the not-yet-populated flag, which defaults to
## "not upgraded" and would flash the card for an already-upgraded user (#2483).
var _upgrade_check_in_flight: bool = false

## Scene-defined (portrait) LabelSettings, kept as the base to derive sized copies from.
var _title_base_settings: LabelSettings = null
var _paragraph_base_settings: LabelSettings = null

@onready var button_add_email: Button = %Button_AddEmail
@onready var _title_label: Label = get_node_or_null("%Label_Title")
@onready var _paragraph_label: Label = get_node_or_null("%Label_Content")


func _apply_full_width() -> void:
	if full_width:
		add_theme_constant_override("margin_left", 0)
		add_theme_constant_override("margin_right", 0)
	else:
		add_theme_constant_override("margin_left", side_margin)
		add_theme_constant_override("margin_right", side_margin)


# Only the landscape (in-game) settings card needs a top margin: portrait settings already gets its
# spacing from the layout, and discover is untouched. Skipped in the editor where Global is inert.
func _apply_top_margin() -> void:
	var top: int = 0
	if (
		not Engine.is_editor_hint()
		and shown_in == "settings"
		and not Global.is_orientation_portrait()
	):
		top = landscape_settings_top_margin
	add_theme_constant_override("margin_top", top)


# Keep the scene's (portrait) LabelSettings as a base. We never mutate it in place — a Label
# only reliably relayouts when its label_settings *reference* changes — so each apply assigns a
# freshly sized duplicate. The base is a resource shared across instances, so duplicating (never
# mutating) also stops the settings card from resizing the discover card's text.
func _setup_font_sizes() -> void:
	if is_instance_valid(_title_label):
		_title_base_settings = _title_label.label_settings
	if is_instance_valid(_paragraph_label):
		_paragraph_base_settings = _paragraph_label.label_settings


# Landscape (in-game) settings uses its own title/paragraph sizes; every other context keeps
# the scene's portrait sizes. Mirrors _apply_top_margin's gating.
func _apply_font_sizes() -> void:
	if Engine.is_editor_hint():
		return
	if _title_base_settings == null or _paragraph_base_settings == null:
		return
	var landscape: bool = shown_in == "settings" and not Global.is_orientation_portrait()
	if landscape:
		_title_label.label_settings = _sized_settings(
			_title_base_settings, landscape_title_font_size
		)
		_paragraph_label.label_settings = _sized_settings(
			_paragraph_base_settings, landscape_paragraph_font_size
		)
	else:
		_title_label.label_settings = _title_base_settings
		_paragraph_label.label_settings = _paragraph_base_settings


# A copy of `base` at `size`, so assigning it forces the Label to relayout (in-place font_size
# mutation on the shared resource does not).
func _sized_settings(base: LabelSettings, size: int) -> LabelSettings:
	var settings: LabelSettings = base.duplicate()
	settings.font_size = size
	return settings


func _ready() -> void:
	_apply_full_width()
	if Engine.is_editor_hint():
		return
	_apply_top_margin()
	_setup_font_sizes()
	_apply_font_sizes()
	button_add_email.pressed.connect(_async_on_add_email_pressed)
	visibility_changed.connect(_on_visibility_changed)
	Global.orientation_changed.connect(_on_orientation_changed)
	Global.guest_upgrade_state_refreshed.connect(_on_guest_upgrade_state_refreshed)
	_async_update_visibility()


func _on_orientation_changed(_is_portrait: bool) -> void:
	_apply_top_margin()
	_apply_font_sizes()
	_async_update_visibility()


func refresh_visibility() -> void:
	_async_update_visibility()


# The upgrade affordance is only meaningful for a thirdweb guest that hasn't
# linked anything yet. Real wallets, disposable LocalWallets, or already-upgraded
# guests (email/social linked) must not see it. The "already upgraded" bit can't
# be known locally — a recovered session doesn't record it — so we ask thirdweb
# for the linked profiles. Start hidden, then reveal only once confirmed.
# After the first network check, subsequent calls use the local cached flag.
# gdlint:ignore = async-function-name
func _async_update_visibility() -> void:
	visible = false
	if Global.player_identity == null or not Global.player_identity.is_thirdweb_guest():
		return

	if _upgrade_checked:
		# Already have an authoritative result — use the Rust-cached flag.
		visible = not Global.player_identity.is_thirdweb_guest_upgraded()
		return

	if _upgrade_check_in_flight:
		# A check is already running; stay hidden (the default) until it resolves.
		# Reading the cached flag now would return the default "not upgraded" and
		# wrongly flash the card for an already-upgraded user (#2483).
		return

	_upgrade_check_in_flight = true
	var anchor: String = Global.get_device_anchor_id()
	var promise: Promise = Global.player_identity.async_refresh_thirdweb_upgrade_state(anchor)
	var result = await PromiseUtils.async_awaiter(promise)
	_upgrade_check_in_flight = false
	_upgrade_checked = true
	var is_upgraded: bool
	if result is PromiseError:
		# Couldn't confirm against thirdweb — fall back to the last-known cached
		# flag rather than assume "not upgraded" and wrongly offer the upgrade.
		is_upgraded = Global.player_identity.is_thirdweb_guest_upgraded()
	else:
		# result is the authoritative upgraded bool.
		is_upgraded = bool(result)
	visible = not is_upgraded
	Global.guest_upgrade_state_refreshed.emit(is_upgraded)


func _get_shown_in() -> String:
	if shown_in == "discover":
		return "discover_ingame" if Global.get_explorer() else "discover_pregame"
	return shown_in


func _on_visibility_changed() -> void:
	if not is_visible_in_tree():
		return
	# During menu page transitions this card can briefly become visible while another
	# screen is actually the active one (e.g. the discover card flips visible mid-crossfade
	# to settings). Only emit when the host menu's current screen matches this card's
	# location, so UPGRADE_NOTICE_SHOW never reports the wrong screen (issue #2377).
	if not _is_on_active_menu_screen():
		return
	Global.metrics.track_screen_viewed(
		"UPGRADE_NOTICE_SHOW", JSON.stringify({"shown_in": _get_shown_in()})
	)


# Walks up to the host menu (duck-typed via `current_screen_name`) and checks that its
# active screen matches this card's `shown_in` location. Returns true when no such menu
# ancestor exists (card used standalone), preserving the prior behavior in that case.
func _is_on_active_menu_screen() -> bool:
	var node: Node = get_parent()
	while node != null:
		if "current_screen_name" in node:
			var prefix := "SETTINGS" if shown_in == "settings" else tr("GUEST_UPGRADE_DISCOVER")
			return String(node.current_screen_name).begins_with(prefix)
		node = node.get_parent()
	return true


func _async_on_add_email_pressed() -> void:
	Global.metrics.track_click_button(
		"UPGRADE_NOTICE_TAP", "UPGRADE_NOTICE_SHOW", JSON.stringify({"shown_in": _get_shown_in()})
	)
	# In-game the Settings panel is landscape but the Add Email modal is portrait-only, so route the
	# guest through Discover first (which closes Settings and switches the app to portrait) before
	# starting the flow, so the modal renders in portrait like everywhere else (issue #2403).
	if shown_in == "settings" and Global.get_explorer():
		Global.open_discover.emit()
		await _async_wait_for_portrait()
	# The full Add Email → OTP → reward flow lives in the modal manager so this notice and the
	# aspirational upgrade modal share one identical experience (issues #2377 / #2372).
	await Global.modal_manager.async_start_add_email_flow()


# Discover switches the app to portrait asynchronously; wait a few frames so the Add Email modal is
# created at the portrait size instead of mid-rotation. Bail out after a short cap so we never hang
# if the orientation never changes.
func _async_wait_for_portrait() -> void:
	var frames: int = 0
	while not Global.is_orientation_portrait() and frames < 30:
		await get_tree().process_frame
		frames += 1


# The shared flow emits guest_upgrade_state_refreshed(true) once the guest upgrades — hide the
# notice so it doesn't linger over the now-registered account.
func _on_guest_upgrade_state_refreshed(is_upgraded: bool) -> void:
	if is_upgraded:
		visible = false
