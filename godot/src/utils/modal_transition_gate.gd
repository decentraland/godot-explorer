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
##   waits a short, bounded time for it to catch up rather than trying to detect it exactly —
##   unlike `guest_upgrade_card.gd`'s `_async_wait_for_portrait()`, which waits on the app's own
##   `Global.is_orientation_portrait()` flag (set synchronously, before any rotation) and is
##   verified on-device for that different condition. Only the frame budget is shared here, not
##   the mechanism.
##
## Does not wait for the controls FTUE overlay (`ControlsFtueCoordinator.async_wait_until_clear`,
## uncapped) — not because either modal is confined to Discover (the streak modal is; the upgrade
## nudge is evaluated in any portrait context: menu.gd, chatbar.gd, profile.gd and others all call
## `Global.set_orientation_portrait()`), but because the overlay itself is landscape-only and
## cancels on `_on_orientation_changed(true)` (controls_ftue_coordinator.gd), while both modals
## only ever show in portrait — so the two can never be on screen at the same time. Call it too if
## that invariant changes.
##
## Only covers the explorer's own loading screen: `Global.get_explorer()` is null in the lobby,
## and also goes null/freed mid-teardown during `Global.return_to_discover()` — which itself calls
## `set_orientation_portrait()`, so it can trigger the upgrade nudge's evaluation. Either way the
## loading-screen wait below just no-ops; the actual surface then (`SplashOverlay`, also layer 100)
## is not covered. A real, if narrow, gap — not something this guards against today.
##
## Not for modals that ARE the intended response to the thing in flight (a failed teleport, a
## scene crash) — those should interrupt immediately, which is what
## ModalManager._force_hide_loading_screen() is for.

## Past the loading screen's own 1s fade-out tween, with headroom so a slightly slower device
## still clears it; bounded so a stalled fade can never hang the modal forever. A real-time
## deadline, not a frame count — `project.godot` sets no `max_fps`, so a 90-frame budget would be
## half as long on a 120Hz display as on 60Hz.
const MAX_LOADING_WAIT_MS := 1500

## Long enough for a normal rotation to catch up, short enough to never visibly stall the modal
## if the device is still rotating when the budget runs out.
const MAX_ORIENTATION_WAIT_MS := 500


## Call before instantiating the modal, not after — the goal is for it to never lay out against
## a loading screen or window size that is still settling.
static func async_wait_until_settled() -> void:
	await _async_wait_for_loading_screen_hidden()
	await _async_wait_for_orientation_settled()


static func _async_wait_for_loading_screen_hidden() -> void:
	var explorer := Global.get_explorer()
	if not is_instance_valid(explorer):
		return
	var deadline := Time.get_ticks_msec() + MAX_LOADING_WAIT_MS
	while (
		is_instance_valid(explorer)
		and explorer.loading_ui.visible
		and Time.get_ticks_msec() < deadline
	):
		await Global.get_tree().process_frame


static func _async_wait_for_orientation_settled() -> void:
	var deadline := Time.get_ticks_msec() + MAX_ORIENTATION_WAIT_MS
	while Time.get_ticks_msec() < deadline:
		if _is_orientation_settled(
			DisplayServer.window_get_size(), Global.is_orientation_portrait()
		):
			return
		await Global.get_tree().process_frame


## Split out from the poll above so it can be unit-tested without a window: true once `window_size`
## agrees with `expect_portrait`. A square window reads as portrait — real devices never report one.
static func _is_orientation_settled(window_size: Vector2i, expect_portrait: bool) -> bool:
	var is_window_portrait := window_size.y >= window_size.x
	return is_window_portrait == expect_portrait
