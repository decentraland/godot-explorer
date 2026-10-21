class_name StreakModalController
extends Node

## Owns the login-streak modal's lifecycle and its reward-claim handoff (issue #3018). Split out
## from ModalManager to stay under its 1900-line gdlint cap (CLAUDE.md "Code hygiene"); mirrors
## ModalManager's own create/show/close skeleton. Claims go through RewardClaimer directly — the
## streak modal's own ClaimButton stays open the whole time, it never navigates to another modal.

const STREAK_MODAL_SCENE_PATH = "res://src/ui/components/organisms/streak_modal/streak_modal.tscn"

var current_streak_modal: StreakModal = null
var streak_modal_scene: PackedScene = null
# Raw rewards-server error code (e.g. "already_claimed") from the most recent failed claim —
# "" after a successful one or before any claim has run. Debug-only surface (see
# StreakModal._set_claim_failed_visible): QA needs this to tell apart WHY a claim failed without
# pulling device logs; a real player only ever sees the generic "Reward claim failed" copy.
var last_claim_error_code: String = ""
var _streak_canvas_layer: CanvasLayer = null


func _ready() -> void:
	streak_modal_scene = load(STREAK_MODAL_SCENE_PATH)
	if not streak_modal_scene:
		push_error(
			(
				"StreakModalController: Could not load streak modal scene at: "
				+ STREAK_MODAL_SCENE_PATH
			)
		)


func is_open() -> bool:
	return is_instance_valid(current_streak_modal) and current_streak_modal.visible


## Shows the login-streak modal for the given state. Returns true only if the modal was actually
## created and set up, mirroring ModalManager.async_show_reward_modal's contract.
## @param state: one of StreakModal.State.
## @param pending_day: -1 ("not specified") falls back to progress_day; any other value (0
## included) renders exactly that day as the row's "today" pill — see StreakModal's own doc
## comment on _pending_day for why 0 can't share the -1 default.
func async_show_streak_modal(
	state: int, progress_day: int, is_guest: bool, pending_day: int = -1
) -> bool:
	var modal = await _async_create_streak_modal()
	if not is_instance_valid(modal):
		return false
	await modal.async_setup(state, progress_day, is_guest, pending_day)
	return true


func close_streak_modal() -> void:
	if is_instance_valid(current_streak_modal):
		current_streak_modal.hide()
		_remove_streak_modal()


func _async_create_streak_modal() -> StreakModal:
	if is_instance_valid(current_streak_modal):
		close_streak_modal()

	if not streak_modal_scene:
		push_error("StreakModalController: Streak modal scene is not loaded")
		return null

	var modal = streak_modal_scene.instantiate() as StreakModal
	if not modal:
		push_error("StreakModalController: Could not instantiate streak modal")
		return null
	# Stays hidden through the NodeGuard frames below and async_setup()'s own render/thumbnail
	# work — otherwise the scene's last-saved look flashes on screen for a couple of frames
	# before _render() overwrites it with the actual state.
	modal.visible = false

	if is_instance_valid(_streak_canvas_layer):
		_streak_canvas_layer.get_parent().remove_child(_streak_canvas_layer)
		_streak_canvas_layer.queue_free()

	_streak_canvas_layer = CanvasLayer.new()
	_streak_canvas_layer.layer = 100

	var root = get_tree().root
	if not root:
		push_error("StreakModalController: Could not get scene tree root")
		return null

	root.add_child(_streak_canvas_layer)
	_streak_canvas_layer.add_child(modal)
	current_streak_modal = modal

	current_streak_modal.tree_exited.connect(_on_streak_modal_tree_exited)

	await get_tree().process_frame
	await get_tree().process_frame

	# Those two frames are a window in which the modal can be freed — see NodeGuard.
	if not NodeGuard.is_alive(modal, "StreakModalController._async_create_streak_modal"):
		return null

	return modal


func _on_streak_modal_tree_exited() -> void:
	current_streak_modal = null


func _remove_streak_modal() -> void:
	if is_instance_valid(current_streak_modal):
		if current_streak_modal.tree_exited.is_connected(_on_streak_modal_tree_exited):
			current_streak_modal.tree_exited.disconnect(_on_streak_modal_tree_exited)
		current_streak_modal.queue_free()
	current_streak_modal = null
	if is_instance_valid(_streak_canvas_layer):
		_streak_canvas_layer.queue_free()
	_streak_canvas_layer = null


## Claims every day in `days` end-to-end — awaits the FULL round trip (not just "a modal was
## shown"), so a ClaimButton's claim_action can safely drive its own CLAIMING.../CLAIMED visual
## state off this call's result. `days` holds 1 or 2 entries — D3 and D7 can be pending at once
## when a player skips D3's claim and reaches D7 before coming back for it. Safe to retry on a
## partial failure: RewardClaimer treats a repeat claim for an already-granted day as a no-op
## success, not a duplicate grant. Assumes the account can already claim — a guest runs
## async_run_guest_upgrade() first (see StreakModal._show_claim_button, which hides the streak
## modal for that step and only calls this once the account is actually upgraded).
func async_claim_rewards(days: Array[int]) -> bool:
	var all_ok := true
	for day in days:
		all_ok = await _async_claim_direct(day) and all_ok
	return all_ok


## Performs the actual claim in place (no modal navigation — the streak modal's own ClaimButton
## drives its CLAIMING/CLAIMED visual off this call's result). Marks it claimed in StreakManager
## and reports LOGIN_STREAK_CLAIM either way.
func _async_claim_direct(day: int) -> bool:
	var key := "LoginStreakD3" if day == 3 else "LoginStreakD7"
	var result := await RewardClaimer.async_claim(RewardCampaigns.CAMPAIGNS[key])
	_on_streak_claim_resolved(result.ok, day, str(result.get("code", "")))
	return result.ok


## Runs the shared OTP upgrade flow once (default grant_campaign_key "MobilePet", so the upgrade
## reward shows first — chain, not suppress), tagged source=SOURCE_LOGIN_STREAK for attribution.
## Awaits AddEmailFlow's own flow_concluded, not Global.guest_upgrade_state_refreshed — that one
## never fires on cancel/error and would leave the ClaimButton stuck on CLAIMING forever.
func async_run_guest_upgrade() -> bool:
	Global.add_email_flow.async_start_add_email_flow("MobilePet", AddEmailFlow.SOURCE_LOGIN_STREAK)
	return await Global.add_email_flow.flow_concluded


func _on_streak_claim_resolved(success: bool, day: int, error_code: String = "") -> void:
	if success:
		Global.streak_manager.mark_claimed(day)
	last_claim_error_code = "" if success else error_code
	Global.metrics.track_login_streak_claim(
		"d%d" % day, "success" if success else "error", error_code
	)
