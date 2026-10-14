@tool
class_name ClaimButton
extends Button

## Button with one background texture per visual state (texture_normal/_tapped/_waiting/_done,
## each also dimmed to DIMMED_OPACITY except texture_normal/_tapped), clipped to its own bounds.
## Also owns the async reward-claim lifecycle: start_claiming() dims the label and loops
## "CLAIMING." / ".." / "..."; finish_claiming() is the terminal state — "CLAIMED", disabled.
##
## Self-contained: assign claim_action to an async Callable that performs the actual claim and
## returns true on success (the owner's business logic — which campaign, guest vs not, closing a
## parent modal — lives in that Callable, never in this atom). On press this button disables
## itself, runs start_claiming(), awaits claim_action, then finish_claiming() on success or
## resets to idle (re-enabled, claimable again) on failure. Deciding WHETHER this button should be
## visible at all (e.g. "is day 3 actually claimable right now") stays the owner's job — that's
## business state this atom has no business knowing.

enum ClaimState { IDLE, CLAIMING, CLAIMED }

## Shared by the label (while CLAIMING) and the background (while CLAIMING or CLAIMED).
const DIMMED_OPACITY := 0.7
const DOT_INTERVAL := 0.4
const MAX_DOTS := 3
# A claim that fails instantly (e.g. the campaign isn't published yet — see RewardClaimer's
# empty-campaign check) would otherwise flash CLAIMING for a single frame, reading as "the button
# did nothing" rather than "the claim failed" (issue #3018). Floor the visible CLAIMING time.
const MIN_CLAIMING_DURATION_SEC := 1.0

@export var texture_normal: Texture2D:
	set(value):
		texture_normal = value
		if is_node_ready():
			_update_background()

## Momentary press/hover-press feedback while still IDLE — the button is disabled once CLAIMING
## actually starts, so get_draw_mode() never reports pressed there; this is IDLE-only.
@export var texture_tapped: Texture2D:
	set(value):
		texture_tapped = value
		if is_node_ready():
			_update_background()

@export var texture_waiting: Texture2D:
	set(value):
		texture_waiting = value
		if is_node_ready():
			_update_background()

@export var texture_done: Texture2D:
	set(value):
		texture_done = value
		if is_node_ready():
			_update_background()

## Translation key for the idle (pre-claim) label. Ignored once start_claiming()/finish_claiming()
## has moved the button past the idle state.
@export var text_normal: String = "STREAK_CTA_CLAIM":
	set(value):
		text_normal = value
		if is_node_ready() and _claim_state == ClaimState.IDLE:
			_refresh_text()

## Translation key for the CLAIMING label, before the animated dots (see show_waiting_dots).
@export var text_waiting: String = "CLAIM_BUTTON_CLAIMING":
	set(value):
		text_waiting = value
		if is_node_ready() and _claim_state == ClaimState.CLAIMING:
			_refresh_text()

## Translation key for the terminal CLAIMED label.
@export var text_done: String = "CLAIM_BUTTON_CLAIMED":
	set(value):
		text_done = value
		if is_node_ready() and _claim_state == ClaimState.CLAIMED:
			_refresh_text()

## Whether the CLAIMING label animates "." / ".." / "..." while waiting. False shows text_waiting
## as a static string instead (and skips starting the dot timer entirely).
@export var show_waiting_dots: bool = true

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
	_label.modulate.a = DIMMED_OPACITY
	_refresh_text()
	_update_background()
	if show_waiting_dots:
		_dot_timer.start(DOT_INTERVAL)


func finish_claiming() -> void:
	_claim_state = ClaimState.CLAIMED
	_dot_timer.stop()
	_label.modulate.a = 1.0
	disabled = true
	_refresh_text()
	_update_background()


func _on_dot_timer_timeout() -> void:
	_dot_count = (_dot_count % MAX_DOTS) + 1
	_refresh_text()


# gdlint:ignore = async-function-name
func _on_pressed() -> void:
	if not claim_action.is_valid() or _claim_state != ClaimState.IDLE:
		return
	start_claiming()
	var started_at := Time.get_ticks_msec()
	var success: bool = await claim_action.call()
	if not is_instance_valid(self):
		return
	var elapsed_sec := (Time.get_ticks_msec() - started_at) / 1000.0
	if elapsed_sec < MIN_CLAIMING_DURATION_SEC:
		await get_tree().create_timer(MIN_CLAIMING_DURATION_SEC - elapsed_sec).timeout
	if not is_instance_valid(self):
		return
	if success:
		finish_claiming()
	else:
		_reset_to_idle()


func _reset_to_idle() -> void:
	_claim_state = ClaimState.IDLE
	disabled = false
	_label.modulate.a = 1.0
	_dot_timer.stop()
	_refresh_text()
	_update_background()


func _refresh_text() -> void:
	match _claim_state:
		ClaimState.CLAIMING:
			var dots := ".".repeat(_dot_count) if show_waiting_dots else ""
			_label.text = TranslationKey.new(text_waiting).text() + dots
		ClaimState.CLAIMED:
			_label.text = TranslationKey.new(text_done).text()
		ClaimState.IDLE:
			_label.text = TranslationKey.new(text_normal).text() if text_normal else ""


func _update_background() -> void:
	if not _background:
		return
	match _claim_state:
		ClaimState.IDLE:
			var mode := get_draw_mode()
			var is_pressed := mode == DRAW_PRESSED or mode == DRAW_HOVER_PRESSED
			_background.texture = texture_tapped if is_pressed else texture_normal
		ClaimState.CLAIMING:
			_background.texture = texture_waiting
		ClaimState.CLAIMED:
			_background.texture = texture_done
	_background.visible = _background.texture != null
	_background.modulate.a = 1.0 if _claim_state == ClaimState.IDLE else DIMMED_OPACITY
