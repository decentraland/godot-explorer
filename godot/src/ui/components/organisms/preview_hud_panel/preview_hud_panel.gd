class_name PreviewHudPanel
extends MarginContainer

## Preview-mode HUD toolbar (issue #2679). Takes the ChatPanel's chatbar slot (top-left,
## next to the menu button) while active: settings Scene Logs, a preview realm, or a
## `scene-stats=true` deep link. Placed statically in explorer.tscn — its console
## (debug_panel) is always present so it keeps capturing scene logs; only the scene-stats
## overlay (and its item) is created on demand, here inside the hub.
##
## The header is one pill with a chevron that expands it to the right, revealing Chat,
## Console, Scene Status (preview/deep-link only) and Reload. Chat, Console
## and Scene Status are mutually exclusive toggles — one body at a time, red while open,
## white while closed. The chat itself is the bound ChatPanel (chatbar replaced by the pill):
## the Chat item drives it through Global.open_chat/close_chat and the item mirrors the
## chat's real open/exit. Collapsing the pill closes whatever is open.

const COLOR_ACTIVE: Color = Color("ff2d55")
const COLOR_DEFAULT: Color = Color("fcfcfc")
# Icon of a momentary button (chevron, Reload) while it is held down.
const COLOR_HELD: Color = Color("df9cff")
const ICON_EXPAND: Texture2D = preload("res://assets/ui/chevron_right.svg")
const ICON_COLLAPSE: Texture2D = preload("res://assets/ui/chevron_left.svg")
const EXPAND_SECONDS: float = 0.25
const COLLAPSE_SECONDS: float = 0.18
# The items start this far under the chevron and slide into place as the pill reveals them.
const ITEMS_SLIDE_OFFSET: float = 24.0
const ACTIVE_DISC_INSET: float = 6.0
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
# Untyped: the ChatPanel script has no class_name. Bound by explorer (bind_chat_panel).
var chat_panel = null
# Set by explorer with the toolbar's availability: only then does the Chat item own the
# ChatPanel's visibility.
var _active: bool = false
var _expanded: bool = false
# The chat's real open state (its on_open_chat / on_exit_chat), not the Chat item's toggle.
var _chat_open: bool = false
var _drawer_tween: Tween = null
# Faint red disc behind an open item; closed items (and the momentary buttons) draw nothing.
var _active_box: StyleBoxFlat = StyleBoxFlat.new()
var _empty_box: StyleBoxEmpty = StyleBoxEmpty.new()

@onready var button_toggle: Button = %Button_Toggle
@onready var drawer: Control = %Control_Drawer
@onready var items: HBoxContainer = %HBoxContainer_Items
@onready var button_chat: Button = %Button_Chat
@onready var button_console: Button = %Button_Console
@onready var button_scene_status: Button = %Button_SceneStatus
@onready var button_reload: Button = %Button_Reload
@onready var body: Control = %Body
# Untyped: the DebugPanel script has no class_name, so its custom methods
# (set_console_visible/reload_current_scene) dispatch cleanly.
@onready var debug_panel = %DebugPanel


func _ready() -> void:
	_active_box.bg_color = Color(COLOR_ACTIVE, 0.2)
	_active_box.set_corner_radius_all(100)
	_active_box.set_expand_margin_all(-ACTIVE_DISC_INSET)
	button_chat.toggle_mode = true
	button_console.toggle_mode = true
	button_scene_status.toggle_mode = true
	button_toggle.pressed.connect(_on_toggle_pressed)
	button_chat.pressed.connect(_on_chat_pressed)
	button_console.pressed.connect(_on_console_pressed)
	button_scene_status.pressed.connect(_on_scene_status_pressed)
	button_reload.pressed.connect(_on_reload_pressed)
	# Console starts collapsed; scene-stats is created on demand via set_scene_status_available.
	debug_panel.hide()
	button_scene_status.hide()
	_setup_momentary_style(button_toggle)
	_setup_momentary_style(button_reload)
	_refresh_toggle_colors()
	# The items row keeps its natural size inside the clipping drawer: the drawer's width is
	# what animates (0 = collapsed, the pill shrinks around it) and its height follows the row.
	items.minimum_size_changed.connect(_fit_items)
	_fit_items()
	_set_expanded(false, false)


## Create or free the scene-stats overlay (and show/hide its item). Only a preview realm or
## a `scene-stats=true` deep link offers it; the settings Scene Logs entry exposes Chat,
## Console and Reload only. The console/debug panel is never torn down — it stays static.
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


## Explorer hands over the ChatPanel the Chat item drives. While active, the panel shows only
## while its chat is open and hides with the toolbar while writing (like the chatbar it
## replaces); the Chat item mirrors the chat's real open/exit, which also comes from the
## dismiss catcher, a slash command or the navbar.
func bind_chat_panel(panel) -> void:
	chat_panel = panel
	chat_panel.chat.on_open_chat.connect(_on_chat_opened)
	chat_panel.chat.on_exit_chat.connect(_on_chat_exited)
	Global.chat_write_mode_changed.connect(_on_chat_write_mode_changed)


## Explorer flips this with the toolbar's availability: the pill replaces the chatbar
## (hidden, footprint kept) so the chat opens under the toolbar from the Chat item.
func set_active(active: bool) -> void:
	_active = active
	chat_panel.set_chatbar_hidden(active)


## Back to header-only: every tool closed (an open chat through its own flow), toggles off.
## The pill keeps its expanded/collapsed state. Called when the toolbar is restored (e.g.
## after the navbar collapses) and on scene changes.
func reset() -> void:
	if not is_node_ready():
		return
	_close_all_tools()
	_refresh_toggle_colors()


func _on_toggle_pressed() -> void:
	Global.send_haptic_feedback()
	if _expanded:
		_close_all_tools()
		_refresh_toggle_colors()
	_set_expanded(not _expanded, true)


func _on_chat_pressed() -> void:
	Global.send_haptic_feedback()
	# The chat's own open/exit comes back through _on_chat_opened/_exited, settling the toggle.
	if button_chat.button_pressed:
		Global.open_chat.emit()
	else:
		Global.close_chat.emit()


func _on_chat_opened() -> void:
	if _active:
		chat_panel.show()
	_chat_open = true
	_close_console_and_stats()
	button_chat.set_pressed_no_signal(true)
	_refresh_toggle_colors()


func _on_chat_exited() -> void:
	if _active:
		chat_panel.hide()
	_chat_open = false
	button_chat.set_pressed_no_signal(false)
	_refresh_toggle_colors()


func _on_chat_write_mode_changed(is_writing: bool) -> void:
	if _active:
		visible = not is_writing


func _on_console_pressed() -> void:
	Global.send_haptic_feedback()
	var open: bool = button_console.button_pressed
	_close_all_tools()
	if open:
		button_console.set_pressed_no_signal(true)
		debug_panel.show()
		debug_panel.set_console_visible(true)
	_refresh_toggle_colors()


func _on_scene_status_pressed() -> void:
	Global.send_haptic_feedback()
	var open: bool = button_scene_status.button_pressed
	_close_all_tools()
	if open:
		button_scene_status.set_pressed_no_signal(true)
		scene_stats_panel.show()
	_refresh_toggle_colors()


func _on_reload_pressed() -> void:
	Global.send_haptic_feedback()
	debug_panel.reload_current_scene()


## Close whatever is open: the chat through its own flow (its exit comes back through
## _on_chat_exited), the console and scene-stats directly.
func _close_all_tools() -> void:
	if _chat_open:
		Global.close_chat.emit()
	_close_console_and_stats()


func _close_console_and_stats() -> void:
	button_console.set_pressed_no_signal(false)
	button_scene_status.set_pressed_no_signal(false)
	debug_panel.hide()
	if is_instance_valid(scene_stats_panel):
		scene_stats_panel.hide()


## Expand or collapse the pill: the clipping drawer's width sweeps between 0 and the items'
## natural width (the pill grows with it) while the items slide in from under the chevron.
func _set_expanded(expanded: bool, animate: bool) -> void:
	_expanded = expanded
	button_toggle.icon = ICON_COLLAPSE if expanded else ICON_EXPAND
	if _drawer_tween != null and _drawer_tween.is_valid():
		_drawer_tween.kill()
	_drawer_tween = null
	var width: float = items.get_combined_minimum_size().x if expanded else 0.0
	var slide: float = 0.0 if expanded else -ITEMS_SLIDE_OFFSET
	if not animate:
		drawer.custom_minimum_size.x = width
		items.position.x = slide
		return
	var seconds: float = EXPAND_SECONDS if expanded else COLLAPSE_SECONDS
	_drawer_tween = create_tween().set_parallel(true).set_trans(Tween.TRANS_CUBIC)
	_drawer_tween.set_ease(Tween.EASE_OUT if expanded else Tween.EASE_IN)
	_drawer_tween.tween_property(drawer, "custom_minimum_size:x", width, seconds)
	_drawer_tween.tween_property(items, "position:x", slide, seconds)


## The drawer is a plain Control, so the items row sizes itself here (also when the Scene
## Status item appears or goes away) and an expanded drawer follows the new width.
func _fit_items() -> void:
	var natural: Vector2 = items.get_combined_minimum_size()
	items.size = natural
	drawer.custom_minimum_size.y = natural.y
	var animating: bool = _drawer_tween != null and _drawer_tween.is_running()
	if _expanded and not animating:
		drawer.custom_minimum_size.x = natural.x


## Chevron and Reload never show a background: their icon is white and turns lilac (#DF9CFF)
## while held (driven by the momentary pressed draw state).
func _setup_momentary_style(button: Button) -> void:
	_tint_icon(button, COLOR_DEFAULT, COLOR_HELD)
	for state_name in BUTTON_STATES:
		button.add_theme_stylebox_override(state_name, _empty_box)


## Chat/Console/Scene Status: icon red (#FF2D55) on a faint red disc while open, white
## (#FCFCFC) with no background while closed. Re-applied on every open/close (incl. the
## mutually-excluded partners, which are toggled off without a signal).
func _refresh_toggle_colors() -> void:
	_color_toggle(button_chat, button_chat.button_pressed)
	_color_toggle(button_console, button_console.button_pressed)
	_color_toggle(button_scene_status, button_scene_status.button_pressed)


func _color_toggle(button: Button, is_open: bool) -> void:
	var color: Color = COLOR_ACTIVE if is_open else COLOR_DEFAULT
	var box: StyleBox = _active_box if is_open else _empty_box
	for state_name in BUTTON_STATES:
		button.add_theme_stylebox_override(state_name, box)
	_tint_icon(button, color, color)


func _tint_icon(button: Button, normal_color: Color, pressed_color: Color) -> void:
	button.add_theme_color_override("icon_normal_color", normal_color)
	button.add_theme_color_override("icon_hover_color", normal_color)
	button.add_theme_color_override("icon_focus_color", normal_color)
	button.add_theme_color_override("icon_pressed_color", pressed_color)
	button.add_theme_color_override("icon_hover_pressed_color", pressed_color)
