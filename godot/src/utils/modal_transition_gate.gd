class_name ModalTransitionGate
extends RefCounted

## Waits until it is safe to lay out a full-screen modal that was triggered opportunistically
## off a shared signal (a nudge, a reward) rather than as the direct, intended response to the
## thing in flight (#2867). Two signals this waits past fire too early for that:
##
## - `Global.loading_finished` fires before the loading screen's fade-out tween even starts
##   (loading_screen.gd's `async_hide_loading_screen_effect`), so anything built off it runs
##   while the loading screen is still fully visible for up to another second.
## - `Global.orientation_changed` fires the instant a rotation is *requested*, not once the
##   device has actually finished rotating. `DisplayServer.window_get_size()` is not reliable
##   enough on mobile to detect that moment precisely (see hide_orientation.gd), so this only
##   waits a short, bounded time for it to catch up rather than trying to detect it exactly.
##
## Not for modals that ARE the intended response to the thing in flight (a failed teleport, a
## scene crash) — those should interrupt immediately, which is what
## ModalManager._force_hide_loading_screen() is for.

## Same cap `guest_upgrade_card.gd._async_wait_for_portrait()` already ships and was verified
## on-device for the identical class of race: long enough for a normal rotation, short enough
## to never visibly stall the modal if the device is still catching up.
const MAX_ORIENTATION_WAIT_FRAMES := 30

## ~1.5s at 60fps: past the loading screen's own 1s fade-out tween, with headroom so a slightly
## slower device still clears it, but bounded so a stalled fade can never hang the modal forever.
const MAX_LOADING_WAIT_FRAMES := 90


## Call before instantiating the modal, not after — the goal is for it to never lay out against
## a loading screen or window size that is still settling.
static func async_wait_until_settled() -> void:
	await _async_wait_for_loading_screen_hidden()
	await _async_wait_for_orientation_settled()


static func _async_wait_for_loading_screen_hidden() -> void:
	var explorer := Global.get_explorer()
	if not is_instance_valid(explorer):
		return
	var frames := 0
	while explorer.loading_ui.visible and frames < MAX_LOADING_WAIT_FRAMES:
		await Global.get_tree().process_frame
		frames += 1


static func _async_wait_for_orientation_settled() -> void:
	var frames := 0
	while frames < MAX_ORIENTATION_WAIT_FRAMES:
		var window_size: Vector2i = DisplayServer.window_get_size()
		var is_window_portrait := window_size.y >= window_size.x
		if is_window_portrait == Global.is_orientation_portrait():
			return
		await Global.get_tree().process_frame
		frames += 1
