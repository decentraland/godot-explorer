class_name StreakModal
extends ColorRect

## 7-day login streak modal (issue #3018) — one scene, five states (WELCOME/DAY_PROGRESS/LOST/
## D3_COMPLETE/D7_COMPLETE). async_setup()'s pending_day > 0 renders the "come finish today's
## mission" reminder look (Figma "D2-Streak") instead of the post-credit celebration — same
## title/subtitle copy either way, only the day row's pending-vs-checked pill differs. Shown on
## Discover only (never in-world), driven by StreakManager's queued pending state (see discover.gd).

# i18n-keys: STREAK_CTA_CLAIM, STREAK_WELCOME_SUBTITLE,
# i18n-keys: STREAK_PROGRESS_SUBTITLE_EARLY, STREAK_PROGRESS_SUBTITLE_LATE, STREAK_LOST_SUBTITLE,
# i18n-keys: STREAK_LOST_SUBTITLE_LATE, STREAK_D7_SUBTITLE, STREAK_FOOTER_CAPTION,
# i18n-keys: STREAK_FOOTER_CAPTION_GUEST

enum State { WELCOME, DAY_PROGRESS, LOST, D3_COMPLETE, D7_COMPLETE }

const CHECKPOINT_DAY := 3
const FINAL_DAY := 7
const BOUNCE_OFFSET_PX := 10.0
const BOUNCE_HALF_DURATION_SEC := 0.9
const CLAIM_FAILED_SLIDE_OFFSET_PX := 24.0
const CLAIM_FAILED_SLIDE_DURATION_SEC := 0.3

## Shown behind the reward art while it's still unclaimed / once the live claim succeeds.
@export var texture_pedestal_normal: Texture2D
@export var texture_pedestal_claimed: Texture2D

var _state: State = State.WELCOME
var _progress_day: int = 0
var _is_guest: bool = false
# -1 means "not specified, derive from _progress_day". > 0 is "come finish today's mission"
# reminder territory (Figma "D2-Streak", shown on a Discover landing BEFORE today's jump-in) —
# the day number to render as still-pending in the row/title, since _progress_day itself hasn't
# caught up to it yet. 0 ("today's credit already happened, nothing pending") always reads as
# "> 0 is false" everywhere this is used (title's display_day, row's today_day) and falls back to
# _progress_day either way — confirmed against Figma: the just-credited reward day still reads as
# "today" (not "already passed, unclaimed") until the player actually claims it.
var _pending_day: int = -1
# True while claim_button's claim_action is awaiting the claim (and, for guests, the upgrade
# flow ahead of it) — guards _on_gui_input so tapping outside can't free this modal mid-request
# (same guard reward_modal.gd keeps against its own _claiming).
var _claiming_reward: bool = false
# Mirrors claim_failed_panel's own visible state — see _set_claim_failed_visible's doc comment.
# Needed because _render() re-runs on a translation change, and re-deriving "should the
# claim-failed panel be up" from this instead of unconditionally hiding it is what keeps that
# panel up across a language switch instead of silently clearing it.
var _claim_failed_shown: bool = false
# Same reasoning as _claim_failed_shown, for Label_TapToContinue — it also defaults to visible
# in the .tscn and relies entirely on code to hide it initially.
var _tap_to_continue_shown: bool = false
# The CTA's own i18n key (e.g. "STREAK_CTA_LETS_GO"), set alongside button_cta.text/
# claim_button.text_normal by _set_cta()/_show_claim_button() — used by
# _on_button_cta_pressed() so CLICK_BUTTON reports a stable, English identifier instead of
# whatever language the button happens to be rendered in.
var _cta_metrics_key: String = ""

@onready var label_title: Label = %Label_Title
@onready var label_subtitle: RichTextLabel = %Label_Subtitle
@onready var label_footer: RichTextLabel = %Label_Footer
@onready var footer_container: VBoxContainer = %VBoxContainer_Footer
@onready var streak_day_row: StreakDayRow = %StreakDayRow
@onready var button_cta: Button = %Button_CTA
@onready var claim_button: ClaimButton = %ClaimButton
@onready var pedestal: TextureRect = %Pedestal
@onready var h_box_container_rewards: HBoxContainer = %HBoxContainer_Rewards
@onready var reward_art_1: TextureRect = %TextureRect_RewardArt1
@onready var reward_art_2: TextureRect = %TextureRect_RewardArt2
@onready var claim_failed_panel: MarginContainer = %MarginContainer_ClaimFailed
@onready var label_claim_failed: RichTextLabel = %Label_ClaimFailed
## Trailing child of the card's own VBoxContainer, same structure reward_modal.tscn uses for its
## own Label_TapToContinue — only visible once there's nothing left to claim (see
## _show_claim_button).
@onready var label_tap_to_continue: Label = %Label_TapToContinue


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


## Awaits the reward-art fetch before showing, so the modal never shows while a slot is still
## mid-fetch — a failed or empty fetch just leaves that slot empty.
# gdlint:ignore = async-function-name
func async_setup(state: State, progress_day: int, is_guest: bool, pending_day: int = -1) -> void:
	_state = state
	_progress_day = progress_day
	_is_guest = is_guest
	_pending_day = pending_day
	_render()
	if _is_guest:
		_async_confirm_guest()
	await _async_apply_reward_thumbnails()
	if not is_instance_valid(self):
		return
	show()
	Global.metrics.track_screen_viewed(_screen_view_name(), "")


## RewardShowcase is always visible regardless of state, so the bounce runs for the modal's
## whole lifetime rather than being tied to any one state's setup.
func _ready() -> void:
	var base_y := h_box_container_rewards.position.y
	var tween := create_tween()
	tween.set_loops()
	tween.set_trans(Tween.TRANS_SINE).set_ease(Tween.EASE_IN_OUT)
	tween.tween_property(
		h_box_container_rewards, "position:y", base_y - BOUNCE_OFFSET_PX, BOUNCE_HALF_DURATION_SEC
	)
	tween.tween_property(h_box_container_rewards, "position:y", base_y, BOUNCE_HALF_DURATION_SEC)
	# A claim resolving updates StreakManager synchronously before this modal's own claim_action
	# sees the result, so refreshing the row off this signal (rather than only re-rendering locally
	# on success) also covers the day row staying stale otherwise.
	Global.streak_manager.claim_state_changed.connect(_update_day_row)


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


## Split out of _render() for the same reason _update_day_row() is: _is_guest flips to false the
## moment a guest upgrades mid-claim (see _show_claim_button's claim_action), and the footer needs
## to drop its guest-specific caption right then, without re-running _render()'s full state
## machine.
## is_guest comes from a cached flag that reads "guest" for an upgraded guest until thirdweb is
## asked; drop the "finish setting up your account" caption once it confirms.
func _async_confirm_guest() -> void:
	var still_guest := await RewardClaimer.async_is_unlinked_guest()
	if not is_instance_valid(self) or still_guest:
		return
	_is_guest = false
	_update_footer()


func _update_footer() -> void:
	var footer_key := "STREAK_FOOTER_CAPTION_GUEST" if _is_guest else "STREAK_FOOTER_CAPTION"
	label_footer.text = TranslationKey.new(footer_key).text()


## Rebuilds the day row from StreakManager's current claimed state — split out of _render() so
## claim_state_changed can refresh just this, without re-running the rest of _render()'s state
## machine (which would re-enter _show_claim_button() and stomp the in-flight claim_action).
func _update_day_row() -> void:
	var claimed_days: Array[int] = []
	if Global.streak_manager.is_d3_claimed():
		claimed_days.append(CHECKPOINT_DAY)
	if Global.streak_manager.is_d7_claimed():
		claimed_days.append(FINAL_DAY)
	# Same collapse _render()'s own display_day uses below: a genuine 0 ("today's credit already
	# happened") falls back to _progress_day here too, so the row's "today" pill matches the title.
	streak_day_row.render(
		_progress_day,
		[CHECKPOINT_DAY, FINAL_DAY],
		claimed_days,
		_pending_day if _pending_day > 0 else _progress_day
	)


func _render() -> void:
	# Re-asserts the current claim-failed/tap-to-continue state rather than forcing it hidden —
	# _render() also re-runs on a translation change (see _notification), and that must not
	# silently clear either one before the player has acted on it.
	_set_claim_failed_visible(_claim_failed_shown)
	label_tap_to_continue.visible = _tap_to_continue_shown
	_update_day_row()
	var d7_claimed := Global.streak_manager.is_d7_claimed()

	# RewardShowcase previews whichever reward is next, independent of _state: D1-D3 preview D3,
	# D4-D7 preview D7 (once D3 is claimed, D3 can never be "next" again — the floor-reset after a
	# later streak loss still only ever lands back at D3, not before it). Only the pedestal's base
	# texture is redone here — the thumbnail fetch itself runs once, from async_setup().
	var preview_claimed := d7_claimed if _preview_campaign_key() == "LoginStreakD7" else false
	pedestal.texture = texture_pedestal_claimed if preview_claimed else texture_pedestal_normal

	# Second reward slot when D3 and D7 are BOTH pending to claim at once (D3's entitlement
	# survives skipping straight to D7), or both are already claimed (revisiting afterward still
	# shows both items earned, regardless of whether they were claimed together or separately) —
	# reward_art_2 stays hidden and reward_art_1 is the sole preview the rest of the time.
	reward_art_2.visible = Global.streak_manager.get_claimable_days().size() > 1 or _both_claimed()

	_update_footer()

	# The reminder (pending, pre-credit) and the celebration (post-credit) share the exact same
	# title/subtitle copy per state — only the row above renders differently (pending vs checked).
	var display_day := _pending_day if _pending_day > 0 else _progress_day

	match _state:
		State.WELCOME:
			label_title.text = TranslationKey.new("STREAK_WELCOME_TITLE").text()
			_set_subtitle("STREAK_WELCOME_SUBTITLE")
			_set_cta("STREAK_CTA_LETS_GO", false)
		State.DAY_PROGRESS:
			label_title.text = (
				TranslationKey
				. new("STREAK_PROGRESS_TITLE")
				. plural(display_day)
				. format({"day": display_day})
			)
			# Keyed off is_d3_claimed(), not display_day < CHECKPOINT_DAY — display_day == 3 means
			# "today is Day 3, working towards it" right up until it's actually claimed, including
			# the exact day-3 case a plain "< 3" comparison would wrongly skip straight to LATE.
			var subtitle_key := (
				"STREAK_PROGRESS_SUBTITLE_LATE"
				if Global.streak_manager.is_d3_claimed()
				else "STREAK_PROGRESS_SUBTITLE_EARLY"
			)
			_set_subtitle(subtitle_key)
			_set_cta("STREAK_CTA_GOT_IT", true)
		State.LOST:
			label_title.text = TranslationKey.new("STREAK_LOST_TITLE").text()
			# credit_jump_in()'s gap branch always resets to exactly day 1 (never reached the
			# checkpoint) or day CHECKPOINT_DAY + 1 (reset to the checkpoint floor) — Figma has
			# distinct copy for each ("lost before day 3" vs "lost after it").
			var lost_key := (
				"STREAK_LOST_SUBTITLE_LATE" if _progress_day > 1 else "STREAK_LOST_SUBTITLE"
			)
			_set_subtitle(lost_key)
			_set_cta("STREAK_CTA_START_AGAIN", false)
		State.D3_COMPLETE, State.D7_COMPLETE:
			var is_d7 := _state == State.D7_COMPLETE
			var title_key := "STREAK_D7_TITLE" if is_d7 else "STREAK_D3_TITLE"
			label_title.text = TranslationKey.new(title_key).text()
			# Figma's D3 claim screen has no subtitle under the title at all; the guest "finish
			# setup" nudge lives in the footer (see footer_key above), not here.
			_set_subtitle("STREAK_D7_SUBTITLE" if is_d7 else "")
			# CLAIM reads the same whether the player is a guest or not — a guest tapping it still
			# runs the upgrade flow first (see _show_claim_button's claim_action), just without
			# advertising that step on the button itself.
			var claim_key := (
				"STREAK_CTA_CLAIM_REWARDS"
				if Global.streak_manager.get_claimable_days().size() > 1
				else "STREAK_CTA_CLAIM"
			)
			_show_claim_button(claim_key)


## RewardCampaigns key for whichever reward is next in line — see _render()'s own comment.
func _preview_campaign_key() -> String:
	return "LoginStreakD7" if Global.streak_manager.is_d3_claimed() else "LoginStreakD3"


## True once both reward days are claimed, however that happened — together in one tap or on
## separate occasions days apart.
func _both_claimed() -> bool:
	return Global.streak_manager.is_d3_claimed() and Global.streak_manager.is_d7_claimed()


## RewardCampaigns keys for the reward-art slot(s) to populate — both D3 and D7 when both are
## pending to claim at once, or both already claimed (see _render()'s reward_art_2 comment),
## otherwise just the single next reward.
func _preview_campaign_keys() -> Array[String]:
	if Global.streak_manager.get_claimable_days().size() > 1 or _both_claimed():
		return ["LoginStreakD3", "LoginStreakD7"]
	return [_preview_campaign_key()]


## Empty key hides the subtitle entirely (Figma's D3 claim screen has none) rather than leaving
## an empty RichTextLabel taking up its row's separation gap.
func _set_subtitle(key: String) -> void:
	label_subtitle.visible = not key.is_empty()
	label_subtitle.text = TranslationKey.new(key).text() if key else ""


## @param secondary: true for the dark/secondary button look (Day-N's "GOT IT", dcl_theme.tres'
## "SecondaryButton" variant), false for the default (primary red) Button look used everywhere
## else. Shows Button_CTA and hides ClaimButton — the reward isn't claimable yet in any state
## that calls this.
func _set_cta(key: String, secondary: bool) -> void:
	button_cta.visible = true
	claim_button.visible = false
	button_cta.text = TranslationKey.new(key).text()
	button_cta.theme_type_variation = &"SecondaryButton" if secondary else &""
	_cta_metrics_key = key


## Hides Button_CTA and shows ClaimButton instead — used once the D3/D7 reward is actually
## claimable, so the gradient + claiming/claimed states replace the plain acknowledgement CTA.
## Wires claim_action here (not _ready()) since it needs the days/is_guest for THIS setup() call.
## pedestal/h_box_container_rewards themselves are driven generally by _render()'s own preview logic, not here.
# gdlint:ignore = async-function-name
func _show_claim_button(text_key: String) -> void:
	button_cta.visible = false
	claim_button.visible = true
	claim_button.text_normal = text_key
	_cta_metrics_key = text_key
	var days := Global.streak_manager.get_claimable_days()
	if days.is_empty():
		# Revisiting D3_COMPLETE/D7_COMPLETE after everything is already claimed (e.g. tapping
		# the day row later) — nothing to claim, so land straight on the terminal CLAIMED look
		# instead of a tappable CLAIM that would do nothing.
		claim_button.finish_claiming()
		_set_tap_to_continue_visible(true)
		return
	claim_button.claim_action = func() -> bool:
		_claiming_reward = true
		# Re-checked on every tap: _is_guest was read when the modal opened, before thirdweb
		# could confirm an upgraded guest, and a retry after upgrading must not re-run Add Email.
		if _is_guest:
			_is_guest = await RewardClaimer.async_is_unlinked_guest()
			if not is_instance_valid(self):
				return false
			_update_footer()
		if _is_guest:
			# Hide for the upgrade step — its own email/code/reward modals are the only thing on
			# screen, and nothing here claims (or even shows CLAIMING…) until the account is
			# actually upgraded. Reappearing already mid-claim, rather than staying up stuck
			# behind those modals the whole time, is what stops a tap meant for them from
			# accidentally landing on (and dismissing) this one.
			hide()
			var upgraded := await Global.streak_modal_controller.async_run_guest_upgrade()
			if not is_instance_valid(self):
				return false
			show()
			if not upgraded:
				_claiming_reward = false
				return false
			# The account is no longer a guest from here on — drop the guest-specific footer
			# caption instead of leaving it stale for the rest of this modal's session.
			_is_guest = false
			_update_footer()
		var ok := await Global.streak_modal_controller.async_claim_rewards(days)
		if not is_instance_valid(self):
			return ok
		_claiming_reward = false
		if ok:
			pedestal.texture = texture_pedestal_claimed
			_set_tap_to_continue_visible(true)
		else:
			_set_claim_failed_visible(true)
		return ok


## Mutually exclusive with the footer caption — the claim-failed panel only appears after an
## actual failed attempt and then stays up for the rest of this modal's lifetime (it does NOT
## clear itself on a retry); it only ever starts hidden again because a brand new StreakModal
## instance is created the next time the modal opens (see
## StreakModalController._async_create_streak_modal), never because this instance resets itself.
func _set_claim_failed_visible(show_failed: bool) -> void:
	var was_shown := _claim_failed_shown
	_claim_failed_shown = show_failed
	claim_failed_panel.visible = show_failed
	# Hides the whole footer block (label + its separators), not just the label — otherwise the
	# separators' own spacing stays reserved and leaves a dead gap under the warning panel.
	footer_container.visible = not show_failed
	_update_claim_failed_label()
	if show_failed and not was_shown:
		_async_slide_in_claim_failed()


## Adds the plain-language reason under the headline, and on non-production builds the raw
## rewards-server code too, so QA can tell failures apart without device logs.
func _update_claim_failed_label() -> void:
	label_claim_failed.text = TranslationKey.new("STREAK_CLAIM_FAILED").text()
	var reason := Global.streak_modal_controller.last_claim_error_message
	if _claim_failed_shown and not reason.is_empty():
		label_claim_failed.text += "\n" + reason
	var error_code := Global.streak_modal_controller.last_claim_error_code
	if _claim_failed_shown and not Global.is_production() and not error_code.is_empty():
		label_claim_failed.text += (
			"\n[font_size=14][color=gray]code: %s[/color][/font_size]" % error_code
		)


func _set_tap_to_continue_visible(show_it: bool) -> void:
	_tap_to_continue_shown = show_it
	label_tap_to_continue.visible = show_it


## Slides the panel up from below + fades it in. Awaits one frame first so the VBoxContainer
## above it has already re-sorted off the fresh `visible = true` and claim_failed_panel.position
## reflects its real resting spot — animating from a stale pre-layout position would start the
## slide from the wrong place.
func _async_slide_in_claim_failed() -> void:
	await get_tree().process_frame
	if not is_instance_valid(self):
		return
	var resting_y := claim_failed_panel.position.y
	claim_failed_panel.position.y = resting_y + CLAIM_FAILED_SLIDE_OFFSET_PX
	claim_failed_panel.modulate.a = 0.0
	var tween := create_tween()
	tween.set_trans(Tween.TRANS_SINE).set_ease(Tween.EASE_OUT)
	tween.set_parallel(true)
	tween.tween_property(
		claim_failed_panel, "position:y", resting_y, CLAIM_FAILED_SLIDE_DURATION_SEC
	)
	tween.tween_property(claim_failed_panel, "modulate:a", 1.0, CLAIM_FAILED_SLIDE_DURATION_SEC)


## Fills reward_art_1 (and reward_art_2 when both D3 and D7 are pending — see _render()) with the
## real wearable/emote thumbnail(s), sequentially. A failed or empty fetch on either slot just
## leaves it empty — there's no baked fallback art — it never blocks the claim.
func _async_apply_reward_thumbnails() -> void:
	var keys := _preview_campaign_keys()
	var all_targets: Array[TextureRect] = [reward_art_1, reward_art_2]
	for i in keys.size():
		await _async_apply_reward_thumbnail(
			str(RewardCampaigns.CAMPAIGNS[keys[i]].get("urn", "")), all_targets[i]
		)
		if not is_instance_valid(self):
			return


## Resolves `target`'s texture to the real thumbnail, or leaves it untouched (empty) when `urn`
## is blank or the fetch fails — same resolve-then-fetch shape as wearable_item.gd/backpack.gd's
## own thumbnail loads.
func _async_apply_reward_thumbnail(urn: String, target: TextureRect) -> void:
	if urn.is_empty():
		return
	var wearable = Global.content_provider.get_wearable(urn)
	if wearable == null:
		var content_url := Global.realm.get_profile_content_url()
		await PromiseUtils.async_all(Global.content_provider.fetch_wearables([urn], content_url))
		if not is_instance_valid(self):
			return
		wearable = Global.content_provider.get_wearable(urn)
	if wearable == null:
		push_warning("StreakModal: could not resolve wearable entity for urn: ", urn)
		return
	var promise: Promise = Global.content_provider.fetch_texture(
		wearable.get_thumbnail(), wearable.get_content_mapping()
	)
	var res = await PromiseUtils.async_awaiter(promise)
	if not is_instance_valid(self):
		return
	if res is PromiseError:
		push_warning("StreakModal: reward thumbnail fetch failed for ", urn, ": ", res.get_error())
		return
	target.texture = res.texture


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
	Global.metrics.track_click_button(_cta_metrics_key, _screen_view_name(), "")
	# D3_COMPLETE/D7_COMPLETE: ClaimButton owns the claim lifecycle itself (see _show_claim_button)
	# and stays open through CLAIMING/CLAIMED — dismissing is the "tap outside" gesture below.
	if _state in [State.WELCOME, State.DAY_PROGRESS, State.LOST]:
		Global.streak_modal_controller.close_streak_modal()
