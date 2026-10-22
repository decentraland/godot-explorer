class_name VideoFocusButton
extends Control

## The focus-mode trigger (issue #1381): a HUD button in the right safe-area column, outside
## the area scenes get for their UI, shown only while the crosshair points at a playing video
## screen. Client UI rather than a pointer event, so the scene keeps every prompt and handler
## it has on the screen, and the crosshair picks the screen where several are in view.

const FADE_SECONDS := 0.15

var _video_player: Node = null
var _overlay: Control = null
var _hud: Control = null
var _fade: Tween = null

@onready var _button: HudButton = %Button_Focus


func _ready() -> void:
	hide()
	_button.pressed.connect(_on_button_pressed)
	# Both live next to this node in explorer.tscn; absent in a bare instance.
	_overlay = get_node_or_null("%VideoFocusMode")
	_hud = get_node_or_null("%InteractableHUD")


func _process(_delta: float) -> void:
	var video_player: Node = Global.scene_runner.hovered_video_player
	var overlay_open: bool = _overlay != null and _overlay.visible
	var hud_hidden: bool = _hud != null and not _hud.visible
	if video_player == null or overlay_open or hud_hidden:
		_set_shown(false)
		return
	_video_player = video_player
	_set_shown(true)


## Fades in so the button is noticed when it appears; hides at once so it never lingers.
func _set_shown(shown: bool) -> void:
	if shown == visible:
		return
	if _fade != null:
		_fade.kill()
		_fade = null
	if shown:
		modulate.a = 0.0
		show()
		_fade = create_tween()
		_fade.tween_property(self, "modulate:a", 1.0, FADE_SECONDS)
	else:
		_video_player = null
		hide()


func _on_button_pressed() -> void:
	if _overlay == null or _video_player == null:
		return
	Global.send_haptic_feedback()
	_overlay.enter(_video_player)
	_set_shown(false)
