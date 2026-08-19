extends Control

signal share_place
signal load_scenes_pressed

# Orb button textures (icon baked in). Swapped by state on the chat toggle:
# Default (closed) → Pressed (while the finger is down) → Hold (chat open, after release).
const CHAT_TEX_DEFAULT: Texture2D = preload(
	"res://src/ui/components/organisms/chatbar/chat_default.png"
)
const CHAT_TEX_PRESSED: Texture2D = preload(
	"res://src/ui/components/organisms/chatbar/chat_pressed.png"
)
const CHAT_TEX_HOLD: Texture2D = preload("res://src/ui/components/organisms/chatbar/chat_hold.png")

var _tooltip_tween: Tween = null
var _tooltip_shown: bool = false

# True while the finger is physically down on the chat button (the transient Pressed look).
var _chat_pressing: bool = false

@onready var hbox: HBoxContainer = $HBoxContainer
@onready var button_chat: TextureButton = %Button_Chat
@onready var button_flip: TextureButton = %Button_Flip
@onready var tooltip: HBoxContainer = %HBoxContainer_Tooltip
@onready var discover_panel: PanelContainer = %DiscoverPanel
@onready var panel_load_scenes: PanelContainer = %Panel_LoadScenes


func _ready() -> void:
	Global.close_chat.connect(_on_chat_closed)
	# Drive the transient Pressed look from raw down/up; the toggle itself stays on the
	# existing `pressed` signal path (_on_button_chat_pressed).
	button_chat.button_down.connect(_on_button_chat_down)
	button_chat.button_up.connect(_on_button_chat_up)
	_update_chat_texture()


func _on_hud_button_discover_pressed() -> void:
	Global.open_discover.emit()
	Global.send_haptic_feedback()


func _on_hud_button_share_pressed() -> void:
	share_place.emit()
	Global.send_haptic_feedback()


func _on_button_chat_pressed() -> void:
	Global.send_haptic_feedback()
	if button_chat.button_pressed:
		_enter_chat_mode()
		Global.open_chat.emit()
	else:
		_exit_chat_mode()
		Global.close_chat.emit()
	_update_chat_texture()


func _on_button_chat_down() -> void:
	_chat_pressing = true
	_update_chat_texture()


func _on_button_chat_up() -> void:
	_chat_pressing = false
	_update_chat_texture()


## Skins the chat button by state: Pressed while the finger is down, else Hold when the
## chat is open, else Default. Called from press/release and from _on_chat_closed().
func _update_chat_texture() -> void:
	if button_chat == null:
		return
	var tex: Texture2D = CHAT_TEX_DEFAULT
	if _chat_pressing:
		tex = CHAT_TEX_PRESSED
	elif button_chat.button_pressed:
		tex = CHAT_TEX_HOLD
	button_chat.texture_normal = tex


func _on_chat_closed() -> void:
	if button_chat.button_pressed:
		button_chat.set_pressed_no_signal(false)
	_exit_chat_mode()
	_update_chat_texture()


func _enter_chat_mode() -> void:
	button_flip.show()
	discover_panel.hide()
	if not _tooltip_shown:
		_tooltip_shown = true
		_show_tooltip()


func _on_button_flip_pressed() -> void:
	Global.send_haptic_feedback()
	if Global.is_orientation_portrait():
		Global.set_orientation_landscape()
	else:
		Global.set_orientation_portrait()


func _exit_chat_mode() -> void:
	button_flip.hide()
	discover_panel.show()
	_kill_tooltip()


func _show_tooltip() -> void:
	_kill_tooltip()
	tooltip.modulate = Color.WHITE
	tooltip.show()
	_tooltip_tween = create_tween()
	_tooltip_tween.tween_interval(10.0)
	_tooltip_tween.tween_property(tooltip, "modulate:a", 0.0, 1.0)
	_tooltip_tween.tween_callback(tooltip.hide)


func _kill_tooltip() -> void:
	if _tooltip_tween and _tooltip_tween.is_valid():
		_tooltip_tween.kill()
		_tooltip_tween = null
	tooltip.hide()


func _on_button_load_scenes_pressed() -> void:
	load_scenes_pressed.emit()


func show_load_scenes_button() -> void:
	panel_load_scenes.show()


func hide_load_scenes_button() -> void:
	panel_load_scenes.hide()
