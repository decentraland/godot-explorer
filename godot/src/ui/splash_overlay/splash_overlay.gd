extends CanvasLayer

## Global startup splash overlay (issue #2386).
##
## Registered as the `SplashOverlay` autoload, so it lives on a CanvasLayer above the
## whole scene tree and PERSISTS across `change_scene_to_file` (main -> lobby -> explorer).
## The Decentraland isotype therefore never reloads, moves, or changes size/background
## between the startup stages: it shows a flat-purple background + centered symbol from the
## first frame, the radial spinner is added AROUND the same symbol, and the whole overlay
## fades out once real content is ready (driven by the lobby, see lobby.gd).
##
## This replaces the per-scene `dcl_splash.tscn` instances that used to live inside
## main.tscn and lobby.tscn (which caused a gray scene-swap flash + a size pop).

const PURPLE := Color(0.411765, 0.121569, 0.662745, 1.0)
const LOGO_SIZE := Vector2(96, 96)
const SPINNER_SIZE := Vector2(128, 128)
const FADE_SECONDS := 0.4

const SpinnerScene := preload(
	"res://src/ui/components/atoms/controls/loading_spinner/loading_spinner.tscn"
)
# DEBUG-2386-DIAG: CYAN logo = SplashOverlay. Revert to res://decentraland_logo.png after.
const LogoTexture := preload("res://assets/ui/splash-diag-overlay.png")

var _content: Control
var _spinner: Control
var _fading := false
var _dismissed := false


func _ready() -> void:
	# Above scene/UI CanvasLayers (small layer numbers), below TouchFeedback (128).
	layer = 100
	follow_viewport_enabled = false

	# STOP so stray taps never reach the loading scene underneath the overlay.
	_content = Control.new()
	_content.name = "Content"
	_content.set_anchors_preset(Control.PRESET_FULL_RECT)
	_content.mouse_filter = Control.MOUSE_FILTER_STOP
	add_child(_content)

	var bg := ColorRect.new()
	bg.name = "Background"
	bg.set_anchors_preset(Control.PRESET_FULL_RECT)
	bg.color = PURPLE
	bg.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_content.add_child(bg)

	# CenterContainer keeps the symbol and the spinner concentric at the screen center,
	# so the spinner appears "around" the exact same symbol without moving it.
	var center := CenterContainer.new()
	center.name = "Center"
	center.set_anchors_preset(Control.PRESET_FULL_RECT)
	center.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_content.add_child(center)

	var logo := TextureRect.new()
	logo.name = "Logo"
	logo.texture = LogoTexture
	logo.custom_minimum_size = LOGO_SIZE
	logo.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	logo.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	logo.mouse_filter = Control.MOUSE_FILTER_IGNORE
	center.add_child(logo)

	_spinner = SpinnerScene.instantiate()
	_spinner.name = "Spinner"
	_spinner.custom_minimum_size = SPINNER_SIZE
	_spinner.speed_scale = 2.0
	_spinner.visible = false
	center.add_child(_spinner)

	# Visible from the first frame with just the symbol (spinner added later).
	show_logo()


## Purple + symbol, no spinner. This is the initial/idle splash state.
func show_logo() -> void:
	_dismissed = false
	_fading = false
	visible = true
	_content.modulate.a = 1.0
	_spinner.visible = false


## Purple + symbol + radial spinner around it. Called when actual loading begins.
func show_spinner() -> void:
	_dismissed = false
	_fading = false
	visible = true
	_content.modulate.a = 1.0
	_spinner.visible = true


## Fade the whole overlay out to reveal the underlying content. Idempotent.
func fade_out() -> void:
	if _dismissed or _fading:
		return
	_fading = true
	var tween := create_tween()
	tween.tween_property(_content, "modulate:a", 0.0, FADE_SECONDS)
	tween.tween_callback(_on_faded)


func _on_faded() -> void:
	visible = false
	_dismissed = true
	_fading = false
	_spinner.visible = false
