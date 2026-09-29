class_name SettingsSectionItem
extends Button

## A single row in the Settings section list (left menu in landscape, master list in
## portrait). Title label + trailing chevron; the selected row shows a highlight pill via the
## Button's pressed StyleBox. Rows are built at runtime by settings.gd from its section registry,
## so they carry the section key they select. The Account row additionally shows an UpgradeBadge
## pinned to the end of its title text; every other row frees it.

## Row height differs by orientation (portrait rows are taller; landscape rows are more compact
## since the list is a narrow side menu).
const _PORTRAIT_HEIGHT: float = 72.0
const _LANDSCAPE_HEIGHT: float = 60.0

## Label font size, larger in portrait where the list is the full-width master screen. The chevron
## icon scales with it (same 36/30 ratio) so the trailing glyph keeps its proportion to the text.
const _PORTRAIT_FONT_SIZE: int = 36
const _LANDSCAPE_FONT_SIZE: int = 30
const _PORTRAIT_ICON_WIDTH: int = 19
const _LANDSCAPE_ICON_WIDTH: int = 16

## Label weight follows the pressed/selected state: bold (700) when selected, medium (500) otherwise.
const _FONT_UNPRESSED: FontFile = preload("res://assets/themes/fonts/inter/Inter-Medium.ttf")
const _FONT_PRESSED: FontFile = preload("res://assets/themes/fonts/inter/Inter-Bold.ttf")

## Translation key for the row label (e.g. "SETTINGS_GRAPHICS"). Resolved with tr() so it
## re-translates from settings.gd on NOTIFICATION_TRANSLATION_CHANGED.
@export var title_key: String = "":
	set(value):
		title_key = value
		_refresh_title()

## Identifies which section this row selects (matches the registry key in settings.gd).
@export var section_key: String = "":
	set(value):
		section_key = value
		_update_badge()

@onready var _title_label: Label = $MarginContainer/Label_Title
@onready var _upgrade_badge: UpgradeBadge = $MarginContainer/Label_Title/UpgradeBadge


func _ready() -> void:
	# Duplicate the (scene-shared) LabelSettings so this row's orientation size / pressed weight
	# don't clobber sibling rows, and rotations just mutate it in place.
	if _title_label.label_settings:
		_title_label.label_settings = _title_label.label_settings.duplicate()
	if not Global.orientation_changed.is_connected(_on_orientation_changed):
		Global.orientation_changed.connect(_on_orientation_changed)
	if not toggled.is_connected(_on_toggled):
		toggled.connect(_on_toggled)
	_apply_orientation(Global.is_orientation_portrait())
	_refresh_title()
	refresh_weight()


func _exit_tree() -> void:
	if Global.orientation_changed.is_connected(_on_orientation_changed):
		Global.orientation_changed.disconnect(_on_orientation_changed)


func _on_orientation_changed(_is_portrait: bool) -> void:
	_apply_orientation(Global.is_orientation_portrait())


func _on_toggled(_pressed: bool) -> void:
	refresh_weight()


## Syncs the font weight to the current pressed state. Public so settings.gd can call it after
## set_pressed_no_signal / ButtonGroup deselects, which don't emit `toggled`.
func refresh_weight() -> void:
	if _title_label and _title_label.label_settings:
		_title_label.label_settings.font = _FONT_PRESSED if button_pressed else _FONT_UNPRESSED


func _apply_orientation(portrait: bool) -> void:
	custom_minimum_size.y = _PORTRAIT_HEIGHT if portrait else _LANDSCAPE_HEIGHT
	if _title_label and _title_label.label_settings:
		_title_label.label_settings.font_size = (
			_PORTRAIT_FONT_SIZE if portrait else _LANDSCAPE_FONT_SIZE
		)
	add_theme_constant_override(
		"icon_max_width", _PORTRAIT_ICON_WIDTH if portrait else _LANDSCAPE_ICON_WIDTH
	)
	# Portrait rows are momentary — tapping navigates to the section detail, so nothing should
	# stay pressed. Landscape rows toggle to keep the current section highlighted next to the
	# content pane. (Turning toggle_mode off also clears any lingering pressed state.)
	toggle_mode = not portrait
	refresh_weight()


func _refresh_title() -> void:
	if not title_key.is_empty() and _title_label:
		_title_label.text = tr(title_key)


## Only the Account row keeps the upgrade badge (which then self-manages its guest-upgrade
## visibility); every other row frees it so it never lights up there.
func _update_badge() -> void:
	if not is_node_ready() or not is_instance_valid(_upgrade_badge):
		return
	if section_key == "account":
		_upgrade_badge.refresh_visibility()
	else:
		_upgrade_badge.queue_free()


func retranslate() -> void:
	_refresh_title()
