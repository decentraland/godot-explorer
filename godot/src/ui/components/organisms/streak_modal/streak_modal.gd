class_name StreakModal
extends ColorRect

## 7-day login streak modal (issue #3018) — one scene, five copy/config states, each with two
## looks: the "reminder" (setup()'s pending_day > 0 — Figma "D2-Streak", shown landing on
## Discover with a day still pending, i.e. BEFORE that day's jump-in) and the post-credit
## celebration (pending_day == 0, Figma "D2-Streak-Completed" — AFTER it). Same title/subtitle
## copy either way; only the day row's pending-vs-checked pill differs. Always shown on Discover
## only (never in-world), driven by ModalManager.async_show_streak_modal() from StreakManager's
## queued pending state (see discover.gd). Design source: Figma file SKoahICmYk21D8GKErKxYK,
## pages ATLAS (node 112:49801, spec annotations) and HIFI-WIP (node 86:6423, composed screens)
## — NOT the earlier Draft-Proposal page.
##
## .tscn shape — same backdrop pattern as reward_modal.tscn (ColorRect → Blur shader →
## VBoxContainer → PanelContainer):
##   StreakModal (ColorRect, this script)
##   ├─ Blur (ColorRect, reward_modal.tscn's shader setup)
##   └─ VBoxContainer
##      └─ PanelContainer (card chrome)
##         └─ MarginContainer
##            └─ VBoxContainer
##               ├─ TextureRect_Hero       (flame_hero.png)
##               ├─ VBoxContainer "Text"
##               │  ├─ Label_Title         (unique name, auto_translate_mode = Disabled)
##               │  └─ Label_Subtitle      (unique name, auto_translate_mode = Disabled)
##               ├─ RewardShowcase         (Control, unique name — pedestal + placeholder item + chip,
##               │                          always visible, no per-state text)
##               ├─ StreakDayRow           (instance, unique name)
##               ├─ Label_Footer           (unique name, auto_translate_mode = Disabled — guests
##               │                          get an extra bold line inviting them to finish setup,
##               │                          so it's always dynamic, not just a static key)
##               ├─ ClaimButton            (instance, unique name — shown instead of Button_CTA
##               │                          once the D3/D7 reward is actually claimable)
##               └─ Button_CTA             (unique name)

# i18n-keys: STREAK_CTA_CLAIM, STREAK_CTA_FINISH_SETUP, STREAK_WELCOME_SUBTITLE,
# i18n-keys: STREAK_PROGRESS_SUBTITLE_EARLY, STREAK_PROGRESS_SUBTITLE_LATE, STREAK_LOST_SUBTITLE,
# i18n-keys: STREAK_LOST_SUBTITLE_LATE, STREAK_D7_SUBTITLE, STREAK_FOOTER_CAPTION,
# i18n-keys: STREAK_FOOTER_CAPTION_GUEST

enum State { WELCOME, DAY_PROGRESS, LOST, D3_COMPLETE, D7_COMPLETE }

const CHECKPOINT_DAY := 3
const FINAL_DAY := 7

var _state: State = State.WELCOME
var _progress_day: int = 0
var _is_guest: bool = false
# > 0 when this is the "come finish today's mission" reminder (Figma "D2-Streak", shown on a
# Discover landing BEFORE today's jump-in) rather than the post-credit celebration — the day
# number to render as still-pending in the row/title, since _progress_day itself hasn't caught
# up to it yet.
var _pending_day: int = 0
# True while claim_button's claim_action is awaiting the claim (and, for guests, the upgrade
# flow ahead of it) — guards _on_gui_input so tapping outside can't free this modal mid-request
# (same guard reward_modal.gd keeps against its own _claiming).
var _claiming_reward: bool = false

@onready var label_title: Label = %Label_Title
@onready var label_subtitle: RichTextLabel = %Label_Subtitle
@onready var label_footer: RichTextLabel = %Label_Footer
@onready var streak_day_row: StreakDayRow = %StreakDayRow
@onready var button_cta: Button = %Button_CTA
@onready var claim_button: ClaimButton = %ClaimButton


## Maps StreakManager.take_pending_modal()'s string result to this modal's State enum.
static func state_from_pending(pending: String) -> State:
	match pending:
		"welcome":
			return State.WELCOME
		"progress":
			return State.DAY_PROGRESS
		"lost":
			return State.LOST
		"d3":
			return State.D3_COMPLETE
		"d7":
			return State.D7_COMPLETE
	push_error("StreakModal.state_from_pending: unexpected pending value %s" % pending)
	return State.WELCOME


func setup(state: State, progress_day: int, is_guest: bool, pending_day: int = 0) -> void:
	_state = state
	_progress_day = progress_day
	_is_guest = is_guest
	_pending_day = pending_day
	_render()
	show()
	Global.metrics.track_screen_viewed(_screen_view_name(), "")


func _on_gui_input(event: InputEvent) -> void:
	if _claiming_reward:
		return
	if event is InputEventScreenTouch and event.pressed:
		Global.streak_modal_controller.close_streak_modal()


func _notification(what: int) -> void:
	# Godot can send NOTIFICATION_TRANSLATION_CHANGED as soon as this node enters the tree —
	# before _ready() runs and @onready vars (streak_day_row, etc.) are populated. Guard with
	# is_node_ready() so an early fire doesn't call _render() against still-null refs.
	if what == NOTIFICATION_TRANSLATION_CHANGED and is_node_ready():
		_render()


func _render() -> void:
	var claimed_days: Array[int] = []
	if Global.streak_manager.is_d3_claimed():
		claimed_days.append(CHECKPOINT_DAY)
	if Global.streak_manager.is_d7_claimed():
		claimed_days.append(FINAL_DAY)
	streak_day_row.render(
		_progress_day,
		[CHECKPOINT_DAY, FINAL_DAY],
		claimed_days,
		_pending_day if _pending_day > 0 else -1
	)

	var footer_key := "STREAK_FOOTER_CAPTION_GUEST" if _is_guest else "STREAK_FOOTER_CAPTION"
	label_footer.text = TranslationKey.new(footer_key).text()

	# The reminder (pending, pre-credit) and the celebration (post-credit) share the exact same
	# title/subtitle copy per state — only the row above renders differently (pending vs checked).
	var display_day := _pending_day if _pending_day > 0 else _progress_day

	match _state:
		State.WELCOME:
			label_title.text = TranslationKey.new("STREAK_WELCOME_TITLE").text()
			_set_subtitle("STREAK_WELCOME_SUBTITLE")
			_set_cta(TranslationKey.new("STREAK_CTA_LETS_GO").text(), false)
		State.DAY_PROGRESS:
			label_title.text = (
				TranslationKey
				. new("STREAK_PROGRESS_TITLE")
				. plural(display_day)
				. format({"day": display_day})
			)
			var subtitle_key := (
				"STREAK_PROGRESS_SUBTITLE_EARLY"
				if display_day < CHECKPOINT_DAY
				else "STREAK_PROGRESS_SUBTITLE_LATE"
			)
			_set_subtitle(subtitle_key)
			_set_cta(TranslationKey.new("STREAK_CTA_GOT_IT").text(), true)
		State.LOST:
			label_title.text = TranslationKey.new("STREAK_LOST_TITLE").text()
			# credit_jump_in()'s gap branch always resets to exactly day 1 (never reached the
			# checkpoint) or day CHECKPOINT_DAY + 1 (reset to the checkpoint floor) — Figma has
			# distinct copy for each ("lost before day 3" vs "lost after it").
			var lost_key := (
				"STREAK_LOST_SUBTITLE_LATE" if _progress_day > 1 else "STREAK_LOST_SUBTITLE"
			)
			_set_subtitle(lost_key)
			_set_cta(TranslationKey.new("STREAK_CTA_START_AGAIN").text(), false)
		State.D3_COMPLETE, State.D7_COMPLETE:
			var is_d7 := _state == State.D7_COMPLETE
			var title_key := "STREAK_D7_TITLE" if is_d7 else "STREAK_D3_TITLE"
			label_title.text = TranslationKey.new(title_key).text()
			# Figma's D3 claim screen has no subtitle under the title at all; the guest "finish
			# setup" nudge lives in the footer (see footer_key above), not here.
			_set_subtitle("STREAK_D7_SUBTITLE" if is_d7 else "")
			if _is_guest:
				_show_claim_button("STREAK_CTA_FINISH_SETUP")
			else:
				_show_claim_button("STREAK_CTA_CLAIM")


## Empty key hides the subtitle entirely (Figma's D3 claim screen has none) rather than leaving
## an empty RichTextLabel taking up its row's separation gap.
func _set_subtitle(key: String) -> void:
	label_subtitle.visible = not key.is_empty()
	label_subtitle.text = TranslationKey.new(key).text() if key else ""


## @param secondary: true for the dark/secondary button look (Day-N's "GOT IT", dcl_theme.tres'
## "SecondaryButton" variant), false for the default (primary red) Button look used everywhere
## else. Shows Button_CTA and hides ClaimButton — the reward isn't claimable yet in any state
## that calls this.
func _set_cta(text: String, secondary: bool) -> void:
	button_cta.visible = true
	claim_button.visible = false
	button_cta.text = text
	button_cta.theme_type_variation = &"SecondaryButton" if secondary else &""


## Hides Button_CTA and shows ClaimButton instead — used once the D3/D7 reward is actually
## claimable, so the gradient + claiming/claimed states replace the plain acknowledgement CTA.
## Wires claim_action here (not _ready()) since it needs the day/is_guest for THIS setup() call.
# gdlint:ignore = async-function-name
func _show_claim_button(text_key: String) -> void:
	button_cta.visible = false
	claim_button.visible = true
	claim_button.text_normal = text_key
	var day := FINAL_DAY if _state == State.D7_COMPLETE else CHECKPOINT_DAY
	var is_guest := _is_guest
	claim_button.claim_action = func() -> bool:
		_claiming_reward = true
		var success: bool = await Global.streak_modal_controller.async_claim_reward(day, is_guest)
		_claiming_reward = false
		return success


func _screen_view_name() -> String:
	match _state:
		State.WELCOME:
			return "LOGIN_STREAK_WELCOME_SHOW"
		State.DAY_PROGRESS:
			return "LOGIN_STREAK_PROGRESS_SHOW"
		State.LOST:
			return "LOGIN_STREAK_LOST_SHOW"
	return "LOGIN_STREAK_COMPLETE_SHOW"


func _on_button_cta_pressed() -> void:
	var cta_text := (
		button_cta.text
		if button_cta.visible
		else TranslationKey.new(claim_button.text_normal).text()
	)
	Global.metrics.track_click_button(cta_text, _screen_view_name(), "")
	match _state:
		State.WELCOME, State.DAY_PROGRESS, State.LOST:
			Global.streak_modal_controller.close_streak_modal()
		State.D3_COMPLETE, State.D7_COMPLETE:
			# ClaimButton owns the whole claim lifecycle itself via claim_action (see
			# _show_claim_button) and stays open through CLAIMING/CLAIMED — this modal is never
			# closed or replaced by the claim. Dismissing is the existing "tap outside" gesture
			# (_on_gui_input), matching Figma's "Tap to continue" once CLAIMED.
			pass
