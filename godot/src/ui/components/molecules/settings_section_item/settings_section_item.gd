class_name SettingsSectionItem
extends Button

## A single row in the Settings section list (left menu in landscape, master list in
## portrait). Text + trailing chevron; the selected row shows a highlight pill via the
## Button's pressed StyleBox. Rows are built at runtime by settings.gd from its section
## registry, so they carry the section key they select.

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

## Translation key for the row label (e.g. "SETTINGS_GRAPHICS"). Resolved with tr() so it
## re-translates from settings.gd on NOTIFICATION_TRANSLATION_CHANGED.
@export var title_key: String = "":
	set(value):
		title_key = value
		_refresh_title()

## Identifies which section this row selects (matches the registry key in settings.gd).
@export var section_key: String = ""


func _ready() -> void:
	if not Global.orientation_changed.is_connected(_on_orientation_changed):
		Global.orientation_changed.connect(_on_orientation_changed)
	_apply_orientation(Global.is_orientation_portrait())
	_refresh_title()


func _exit_tree() -> void:
	if Global.orientation_changed.is_connected(_on_orientation_changed):
		Global.orientation_changed.disconnect(_on_orientation_changed)


func _on_orientation_changed(_is_portrait: bool) -> void:
	_apply_orientation(Global.is_orientation_portrait())


func _apply_orientation(portrait: bool) -> void:
	custom_minimum_size.y = _PORTRAIT_HEIGHT if portrait else _LANDSCAPE_HEIGHT
	add_theme_font_size_override(
		"font_size", _PORTRAIT_FONT_SIZE if portrait else _LANDSCAPE_FONT_SIZE
	)
	add_theme_constant_override(
		"icon_max_width", _PORTRAIT_ICON_WIDTH if portrait else _LANDSCAPE_ICON_WIDTH
	)
	# Portrait rows are momentary — tapping navigates to the section detail, so nothing should
	# stay pressed. Landscape rows toggle to keep the current section highlighted next to the
	# content pane. (Turning toggle_mode off also clears any lingering pressed state.)
	toggle_mode = not portrait


func _refresh_title() -> void:
	if not title_key.is_empty():
		text = tr(title_key)


func retranslate() -> void:
	_refresh_title()
