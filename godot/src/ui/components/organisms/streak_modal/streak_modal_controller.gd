class_name StreakModalController
extends Node

## Owns the login-streak modal's lifecycle and its reward-claim handoff (issue #3018). Split out
## from ModalManager — which sits at the repo's 1900-line gdlint cap — rather than growing that
## file further; see godot/src/ui/COMPONENT_AUDIT.md and CLAUDE.md "Code hygiene" for the
## convention this follows (new behaviour gets its own file, not a block bolted onto the big one).
##
## Mirrors ModalManager's own create/show/close/tree_exited/remove skeleton (see
## async_show_reward_modal et al. there) for the streak modal itself, and leans on ModalManager's
## existing reward_modal + add-email OTP flow for the actual claim rather than duplicating either.

signal guest_claim_ready(upgraded: bool)

const STREAK_MODAL_SCENE_PATH = "res://src/ui/components/organisms/streak_modal/streak_modal.tscn"

var current_streak_modal: StreakModal = null
var streak_modal_scene: PackedScene = null
var _streak_canvas_layer: CanvasLayer = null

# Set by _async_claim_guest() while a guest is running the shared OTP upgrade flow to claim a
# streak reward; consumed by _on_guest_upgrade_state_refreshed() once that flow resolves. 0 means
# "no streak claim is riding along" — an unrelated upgrade (e.g. the aspirational nudge) must not
# be mistaken for this one.
var _pending_guest_claim_day: int = 0


func _ready() -> void:
	streak_modal_scene = load(STREAK_MODAL_SCENE_PATH)
	if not streak_modal_scene:
		push_error(
			(
				"StreakModalController: Could not load streak modal scene at: "
				+ STREAK_MODAL_SCENE_PATH
			)
		)
	Global.guest_upgrade_state_refreshed.connect(_on_guest_upgrade_state_refreshed)


func is_open() -> bool:
	return is_instance_valid(current_streak_modal) and current_streak_modal.visible


## Shows the login-streak modal for the given state. Returns true only if the modal was actually
## created and set up, mirroring ModalManager.async_show_reward_modal's contract.
## @param state: one of StreakModal.State.
## @param pending_day: > 0 renders the "come finish today's mission" reminder look instead of the
## post-credit celebration — see StreakModal's own doc comment.
func async_show_streak_modal(
	state: int, progress_day: int, is_guest: bool, pending_day: int = 0
) -> bool:
	var modal = await _async_create_streak_modal()
	if not is_instance_valid(modal):
		return false
	modal.setup(state, progress_day, is_guest, pending_day)
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


## Performs a D3/D7 claim end-to-end and returns true on success — awaits the FULL round trip
## (not just "a modal was shown"), so a ClaimButton's claim_action can safely drive its own
## CLAIMING.../CLAIMED visual state off this call's result.
func async_claim_reward(day: int, is_guest: bool) -> bool:
	if is_guest:
		return await _async_claim_guest(day)
	return await _async_claim_direct(day)


## Shows the reward modal for a login-streak entitlement and awaits its actual resolution (the
## player still has to tap Claim inside the modal — "shown" isn't "resolved"). Marks it claimed in
## StreakManager and reports LOGIN_STREAK_CLAIM either way.
func _async_claim_direct(day: int) -> bool:
	var key := "LoginStreakD3" if day == 3 else "LoginStreakD7"
	var shown := await Global.modal_manager.async_show_reward_modal(RewardCampaigns.CAMPAIGNS[key])
	if not shown or not is_instance_valid(Global.modal_manager.current_reward_modal):
		return false
	var success: bool = await Global.modal_manager.current_reward_modal.claim_resolved
	_on_streak_claim_resolved(success, day)
	return success


## Starts the SAME shared OTP upgrade flow as every other guest-upgrade entry point
## (ModalManager.async_start_add_email_flow — untouched, not parameterized for this) and chains
## into this entitlement's own reward modal once it succeeds. The flow's own MobilePet reward
## (issue #2372) shows first — a deliberate "chain, don't merge or suppress" answer to the
## collision the issue itself flags as open ("guest-upgrade reward fires too").
func _async_claim_guest(day: int) -> bool:
	_pending_guest_claim_day = day
	Global.modal_manager.async_start_add_email_flow()
	var upgraded: bool = await guest_claim_ready
	_pending_guest_claim_day = 0
	if not upgraded:
		return false
	return await _async_claim_direct(day)


func _on_guest_upgrade_state_refreshed(is_upgraded: bool) -> void:
	if _pending_guest_claim_day == 0:
		return
	guest_claim_ready.emit(is_upgraded)


func _on_streak_claim_resolved(success: bool, day: int) -> void:
	if success:
		Global.streak_manager.mark_claimed(day)
	Global.metrics.track_login_streak_claim("d%d" % day, "success" if success else "error")
