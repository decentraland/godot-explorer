class_name PreviewHudPanel
extends MarginContainer

## Preview-mode HUD toolbar (issue #2679). Lives in the same bottom-left slot as the
## ChatPanel and replaces it while active (settings Scene Logs, preview realm, or a
## `scene-stats=true` deep link). Placed statically in explorer.tscn — its console
## (debug_panel) is always present so it keeps capturing scene logs; only the scene-stats
## overlay (and its header button) is created on demand, here inside the hub.
##
## Header buttons: Console (toggles the console), Scene Status (toggles the scene-stats
## overlay; shown only in preview/deep-link), Reload (reloads the current scene). Console
## and Scene Status share the same body position and are mutually exclusive — opening one
## closes the other. Selected buttons tint red, default is white.

const COLOR_ACTIVE: Color = Color("ff2d55")
const COLOR_DEFAULT: Color = Color("fcfcfc")
# Reload icon while the button is held down (it never toggles nor shows a border).
const COLOR_RELOAD_PRESSED: Color = Color("df9cff")
const BUTTON_STATES: Array[String] = [
	"normal",
	"pressed",
	"pressed_mirrored",
	"hover",
	"hover_mirrored",
	"hover_pressed",
	"hover_pressed_mirrored",
	"focus",
]
const SCENE_STATS_SCENE: PackedScene = preload(
	"res://src/ui/components/organisms/scene_stats_panel/scene_stats_panel.tscn"
)

var scene_stats_panel: SceneStatsPanel = null
# Each toggle button gets its own copy of the scene's border stylebox so its border color can
# be set independently (the scene shares one stylebox across both) — recolored per open/close.
var _console_box: StyleBoxFlat = null
var _stats_box: StyleBoxFlat = null

@onready var button_console: Button = %Button_Console
@onready var button_scene_status: Button = %Button_SceneStatus
@onready var button_reload: Button = %Button_Reload
@onready var body: Control = %Body
# Untyped: the DebugPanel script has no class_name, so its custom methods
# (set_console_visible/reload_current_scene) dispatch cleanly.
@onready var debug_panel = %DebugPanel


func _ready() -> void:
	button_console.toggle_mode = true
	button_scene_status.toggle_mode = true
	button_console.button_pressed = false
	button_scene_status.button_pressed = false
	button_console.pressed.connect(_on_console_pressed)
	button_scene_status.pressed.connect(_on_scene_status_pressed)
	button_reload.pressed.connect(_on_reload_pressed)
	# Console starts collapsed; scene-stats is created on demand via set_scene_status_available.
	debug_panel.hide()
	button_scene_status.hide()
	_console_box = _own_border_stylebox(button_console)
	_stats_box = _own_border_stylebox(button_scene_status)
	_setup_reload_style()
	_refresh_toggle_colors()


## Create or free the scene-stats overlay (and show/hide its header button). Only a preview
## realm or a `scene-stats=true` deep link offers it; the settings Scene Logs entry exposes
## Console + Reload only. The console/debug panel is never torn down — it stays static.
func set_scene_status_available(available: bool) -> void:
	button_scene_status.visible = available
	if available:
		if not is_instance_valid(scene_stats_panel):
			scene_stats_panel = SCENE_STATS_SCENE.instantiate()
			scene_stats_panel.hide()
			body.add_child(scene_stats_panel)
	else:
		if button_scene_status.button_pressed:
			button_scene_status.set_pressed_no_signal(false)
		if is_instance_valid(scene_stats_panel):
			scene_stats_panel.queue_free()
			scene_stats_panel = null
		_refresh_toggle_colors()


## Point the scene-stats overlay at the scene being previewed (forwarded from explorer.gd
## on preview / scene changes). No-op until the overlay exists.
func set_scene(scene_id: int) -> void:
	if is_instance_valid(scene_stats_panel):
		scene_stats_panel.set_scene(scene_id)


## Forward a scene console line to the embedded (always-present) debug panel.
func on_console_add(scene_title: String, level: int, timestamp: float, text: String) -> void:
	if not is_node_ready():
		return
	debug_panel.on_console_add(scene_title, level, timestamp, text)


## Collapse back to header-only: both overlays closed, every toggle off. Called when the
## toolbar is restored (e.g. after the navbar collapses), mirroring the chat returning to
## its closed state.
func reset() -> void:
	if not is_node_ready():
		return
	button_console.set_pressed_no_signal(false)
	button_scene_status.set_pressed_no_signal(false)
	debug_panel.hide()
	if is_instance_valid(scene_stats_panel):
		scene_stats_panel.hide()
	_refresh_toggle_colors()


func _on_console_pressed() -> void:
	Global.send_haptic_feedback()
	if button_console.button_pressed:
		button_scene_status.set_pressed_no_signal(false)
		if is_instance_valid(scene_stats_panel):
			scene_stats_panel.hide()
		debug_panel.show()
		debug_panel.set_console_visible(true)
	else:
		debug_panel.hide()
	_refresh_toggle_colors()


func _on_scene_status_pressed() -> void:
	Global.send_haptic_feedback()
	if button_scene_status.button_pressed:
		button_console.set_pressed_no_signal(false)
		debug_panel.hide()
		if is_instance_valid(scene_stats_panel):
			scene_stats_panel.show()
	elif is_instance_valid(scene_stats_panel):
		scene_stats_panel.hide()
	_refresh_toggle_colors()


func _on_reload_pressed() -> void:
	Global.send_haptic_feedback()
	debug_panel.reload_current_scene()


## Give the button its own copy of the scene's border stylebox for every visual state, so its
## border color can be set independently of the other toggle (they share one in the scene).
## Preserves the scene's shape/width — we only recolor the border later.
func _own_border_stylebox(button: Button) -> StyleBoxFlat:
	var src: StyleBox = button.get_theme_stylebox(&"normal")
	var box: StyleBoxFlat
	if src is StyleBoxFlat:
		box = src.duplicate() as StyleBoxFlat
	else:
		box = StyleBoxFlat.new()
	for state_name in BUTTON_STATES:
		button.add_theme_stylebox_override(state_name, box)
	return box


## Reload never shows a border: reuse its borderless scene stylebox for every state. Its icon
## is white and turns lilac (#DF9CFF) while held (driven by the momentary pressed draw state).
func _setup_reload_style() -> void:
	_tint_icon(button_reload, COLOR_DEFAULT, COLOR_RELOAD_PRESSED)
	var borderless: StyleBox = button_reload.get_theme_stylebox(&"normal")
	for state_name in BUTTON_STATES:
		button_reload.add_theme_stylebox_override(state_name, borderless)


## Console/Scene Status: border + icon are red (#FF2D55) while open, white (#FCFCFC) while
## closed. Re-applied on every open/close (incl. the mutually-excluded partner, which is
## toggled off without a signal).
func _refresh_toggle_colors() -> void:
	_color_toggle(button_console, _console_box, button_console.button_pressed)
	_color_toggle(button_scene_status, _stats_box, button_scene_status.button_pressed)


func _color_toggle(button: Button, box: StyleBoxFlat, is_open: bool) -> void:
	var color: Color = COLOR_ACTIVE if is_open else COLOR_DEFAULT
	if box != null:
		box.border_color = color
	_tint_icon(button, color, color)


func _tint_icon(button: Button, normal_color: Color, pressed_color: Color) -> void:
	button.add_theme_color_override("icon_normal_color", normal_color)
	button.add_theme_color_override("icon_hover_color", normal_color)
	button.add_theme_color_override("icon_focus_color", normal_color)
	button.add_theme_color_override("icon_pressed_color", pressed_color)
	button.add_theme_color_override("icon_hover_pressed_color", pressed_color)
