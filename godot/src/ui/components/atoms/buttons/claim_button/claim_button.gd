@tool
class_name ClaimButton
extends Button

## Button with a background texture (e.g. a gradient) clipped to its own bounds — the texture
## shows while the button is at rest and hides on press/hover-press, letting the theme's own
## pressed stylebox read through underneath instead of stacking on top of it. Also owns the
## async reward-claim lifecycle: start_claiming() dims the whole node and loops "CLAIMING." /
## ".." / "..."; finish_claiming() is the terminal state — "CLAIMED", disabled, background hidden.
##
## Self-contained: assign claim_action to an async Callable that performs the actual claim and
## returns true on success (the owner's business logic — which campaign, guest vs not, closing a
## parent modal — lives in that Callable, never in this atom). On press this button disables
## itself, runs start_claiming(), awaits claim_action, then finish_claiming() on success or
## resets to idle (re-enabled, claimable again) on failure. Deciding WHETHER this button should be
## visible at all (e.g. "is day 3 actually claimable right now") stays the owner's job — that's
## business state this atom has no business knowing.

enum ClaimState { IDLE, CLAIMING, CLAIMED }

const CLAIMING_OPACITY := 0.7
const DOT_INTERVAL := 0.4
const MAX_DOTS := 3

@export var background_texture: Texture2D:
	set(value):
		background_texture = value
		if is_node_ready():
			_update_background()

## Translation key for the idle (pre-claim) label — e.g. "STREAK_CTA_CLAIM". Ignored once
## start_claiming()/finish_claiming() has moved the button past the idle state.
@export var custom_text: String:
	set(value):
		custom_text = value
		if is_node_ready() and _claim_state == ClaimState.IDLE:
			_refresh_text()

## Assigned by the owner — an async function that performs the real claim and returns true on
## success. Left unassigned, this button just emits the inherited `pressed` signal and does
## nothing else (callers that don't want the self-managed lifecycle can still connect to
## `pressed` directly, same as any other Button).
var claim_action: Callable

var _claim_state: ClaimState = ClaimState.IDLE
var _dot_count := 1

@onready var _background: TextureRect = %Background
@onready var _label: Label = %Label_Text
@onready var _dot_timer: Timer = %DotTimer


func _ready() -> void:
	_refresh_text()
	_update_background()
	pressed.connect(_on_pressed)


func _notification(what: int) -> void:
	if what == NOTIFICATION_DRAW:
		_update_background()
	elif what == NOTIFICATION_TRANSLATION_CHANGED and is_node_ready():
		_refresh_text()


func start_claiming() -> void:
	_claim_state = ClaimState.CLAIMING
	_dot_count = 1
	disabled = true
	modulate.a = CLAIMING_OPACITY
	_refresh_text()
	_dot_timer.start(DOT_INTERVAL)


func finish_claiming() -> void:
	_claim_state = ClaimState.CLAIMED
	_dot_timer.stop()
	modulate.a = 1.0
	disabled = true
	_refresh_text()
	_update_background()


func _on_dot_timer_timeout() -> void:
	_dot_count = (_dot_count % MAX_DOTS) + 1
	_refresh_text()


func _on_pressed() -> void:
	if not claim_action.is_valid() or _claim_state != ClaimState.IDLE:
		return
	start_claiming()
	var success: bool = await claim_action.call()
	if success:
		finish_claiming()
	else:
		_reset_to_idle()


func _reset_to_idle() -> void:
	_claim_state = ClaimState.IDLE
	disabled = false
	modulate.a = 1.0
	_dot_timer.stop()
	_refresh_text()
	_update_background()


func _refresh_text() -> void:
	match _claim_state:
		ClaimState.CLAIMING:
			_label.text = (
				TranslationKey.new("CLAIM_BUTTON_CLAIMING").text() + ".".repeat(_dot_count)
			)
		ClaimState.CLAIMED:
			_label.text = TranslationKey.new("CLAIM_BUTTON_CLAIMED").text()
		ClaimState.IDLE:
			_label.text = TranslationKey.new(custom_text).text() if custom_text else ""


func _update_background() -> void:
	if not _background:
		return
	if _claim_state == ClaimState.CLAIMED:
		_background.visible = false
		return
	_background.texture = background_texture
	var mode := get_draw_mode()
	_background.visible = (
		background_texture != null and mode != DRAW_PRESSED and mode != DRAW_HOVER_PRESSED
	)
