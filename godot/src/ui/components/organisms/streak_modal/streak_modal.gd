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
const REWARD_ART_FADE_DURATION_SEC := 0.3

## Shown behind the reward art while it's still unclaimed / once the live claim succeeds.
@export var texture_pedestal_normal: Texture2D
@export var texture_pedestal_claimed: Texture2D

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
@onready var footer_container: VBoxContainer = %VBoxContainer_Footer
@onready var streak_day_row: StreakDayRow = %StreakDayRow
@onready var button_cta: Button = %Button_CTA
@onready var claim_button: ClaimButton = %ClaimButton
@onready var pedestal: TextureRect = %Pedestal
@onready var h_box_container_rewards: HBoxContainer = %HBoxContainer_Rewards
@onready var reward_art_1: TextureRect = %TextureRect_RewardArt1
@onready var reward_art_2: TextureRect = %TextureRect_RewardArt2
@onready var claim_failed_panel: MarginContainer = %MarginContainer_ClaimFailed


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


## Awaits the reward-art fetch before reveal — real thumbnail vs. baked placeholder is already
## decided by the time the modal shows, so it's never sitting there with a bare/incomplete art
## slot. The art itself still fades in (see _reveal_reward_thumbnails()) right as the modal
## appears, rather than just popping in fully opaque.
# gdlint:ignore = async-function-name
func async_setup(state: State, progress_day: int, is_guest: bool, pending_day: int = 0) -> void:
	_state = state
	_progress_day = progress_day
	_is_guest = is_guest
	_pending_day = pending_day
	_render()
	var reward_targets := await _async_apply_reward_thumbnails()
	if not is_instance_valid(self):
		return
	show()
	Global.metrics.track_screen_viewed(_screen_view_name(), "")
	_reveal_reward_thumbnails(reward_targets)


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


## Rebuilds the day row from StreakManager's current claimed state — split out of _render() so
## claim_state_changed can refresh just this, without re-running the rest of _render()'s state
## machine (which would re-enter _show_claim_button() and stomp the in-flight claim_action).
func _update_day_row() -> void:
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


func _render() -> void:
	# Every full render starts with the standard footer, not the claim-failed warning — only an
	# actual failed attempt (inside _show_claim_button()'s claim_action) turns it back on, and
	# only states that reach _show_claim_button() can ever show it in the first place.
	_set_claim_failed_visible(false)
	_update_day_row()
	var d7_claimed := Global.streak_manager.is_d7_claimed()

	# RewardShowcase previews whichever reward is next, independent of _state: D1-D3 preview D3,
	# D4-D7 preview D7 (once D3 is claimed, D3 can never be "next" again — the floor-reset after a
	# later streak loss still only ever lands back at D3, not before it). Only the pedestal's base
	# texture is redone here — the thumbnail fetch itself runs once, from async_setup().
	var preview_claimed := d7_claimed if _preview_campaign_key() == "LoginStreakD7" else false
	pedestal.texture = texture_pedestal_claimed if preview_claimed else texture_pedestal_normal

	# Second reward slot only when D3 and D7 are BOTH pending to claim at once (D3's entitlement
	# survives skipping straight to D7) — reward_art_2 stays hidden and reward_art_1 is the sole
	# preview the rest of the time.
	reward_art_2.visible = Global.streak_manager.get_claimable_days().size() > 1

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


## RewardCampaigns keys for the reward-art slot(s) to populate — both D3 and D7 when both are
## pending to claim at once (reward_art_2 visible, see _render()), otherwise just the single next
## reward.
func _preview_campaign_keys() -> Array[String]:
	if Global.streak_manager.get_claimable_days().size() > 1:
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
func _set_cta(text: String, secondary: bool) -> void:
	button_cta.visible = true
	claim_button.visible = false
	button_cta.text = text
	button_cta.theme_type_variation = &"SecondaryButton" if secondary else &""


## Hides Button_CTA and shows ClaimButton instead — used once the D3/D7 reward is actually
## claimable, so the gradient + claiming/claimed states replace the plain acknowledgement CTA.
## Wires claim_action here (not _ready()) since it needs the days/is_guest for THIS setup() call.
## pedestal/h_box_container_rewards themselves are driven generally by _render()'s own preview logic, not here.
# gdlint:ignore = async-function-name
func _show_claim_button(text_key: String) -> void:
	button_cta.visible = false
	claim_button.visible = true
	claim_button.text_normal = text_key
	var days := Global.streak_manager.get_claimable_days()
	var is_guest := _is_guest
	claim_button.claim_action = func() -> bool:
		_claiming_reward = true
		var result: Dictionary = await Global.streak_modal_controller.async_claim_rewards(
			days, is_guest
		)
		_claiming_reward = false
		if result.ok:
			pedestal.texture = texture_pedestal_claimed
		elif not result.cancelled:
			_set_claim_failed_visible(true)
		return result.ok


## Mutually exclusive with the footer caption — the claim-failed panel only appears after an
## actual failed attempt and then stays up for the rest of this modal's lifetime (it does NOT
## clear itself on a retry); it only ever starts hidden again because a brand new StreakModal
## instance is created the next time the modal opens (see
## StreakModalController._async_create_streak_modal), never because this instance resets itself.
func _set_claim_failed_visible(show_failed: bool) -> void:
	claim_failed_panel.visible = show_failed
	# Hides the whole footer block (label + its separators), not just the label — otherwise the
	# separators' own spacing stays reserved and leaves a dead gap under the warning panel.
	footer_container.visible = not show_failed
	if show_failed:
		_async_slide_in_claim_failed()


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
## real wearable/emote thumbnail(s), sequentially. A failed or empty fetch on either slot leaves
## whatever placeholder is already on it, it never blocks the claim. Leaves every touched slot at
## alpha 0 — real-vs-placeholder is decided here, but _reveal_reward_thumbnails() does the actual
## fade-in, once the caller is ready to show it. @return the slots this call actually touched, so
## the caller only reveals those (not whichever slot sits unused/hidden this render).
func _async_apply_reward_thumbnails() -> Array[TextureRect]:
	var keys := _preview_campaign_keys()
	var all_targets: Array[TextureRect] = [reward_art_1, reward_art_2]
	var touched: Array[TextureRect] = []
	for i in keys.size():
		var target := all_targets[i]
		touched.append(target)
		await _async_apply_reward_thumbnail(
			str(RewardCampaigns.CAMPAIGNS[keys[i]].get("urn", "")), target
		)
		if not is_instance_valid(self):
			return touched
	return touched


## Resolves `target`'s texture (real thumbnail or the baked placeholder it already has), same
## resolve-then-fetch shape as wearable_item.gd/backpack.gd's own thumbnail loads, but leaves it
## fully transparent either way — the caller fades it in once it's ready to show it.
func _async_apply_reward_thumbnail(urn: String, target: TextureRect) -> void:
	target.modulate.a = 0.0
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


## Fades each of `targets` in from transparent — called once, right as the modal becomes visible,
## so the reveal plays out on-screen instead of finishing invisibly while the modal was still
## hidden during the fetch.
func _reveal_reward_thumbnails(targets: Array[TextureRect]) -> void:
	for target in targets:
		var tween := create_tween()
		tween.set_trans(Tween.TRANS_SINE).set_ease(Tween.EASE_OUT)
		tween.tween_property(target, "modulate:a", 1.0, REWARD_ART_FADE_DURATION_SEC)


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
	# D3_COMPLETE/D7_COMPLETE: ClaimButton owns the claim lifecycle itself (see _show_claim_button)
	# and stays open through CLAIMING/CLAIMED — dismissing is the "tap outside" gesture below.
	if _state in [State.WELCOME, State.DAY_PROGRESS, State.LOST]:
		Global.streak_modal_controller.close_streak_modal()
