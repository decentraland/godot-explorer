class_name ModalTransitionGate
extends RefCounted

## Waits until it is safe to lay out a full-screen modal triggered opportunistically off
## `Global.loading_finished` / `Global.orientation_changed` — both fire before the thing they
## announce has actually settled on screen (#2867). Not for modals that are the direct, intended
## response to what's in flight (a failed teleport, a crash) — those should interrupt immediately
## via ModalManager._force_hide_loading_screen() instead of waiting.

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
	# Matches loading_screen.gd's own absolute cap: a load that long force-resolves on its own
	# (timeout modal, inactivity), so this never waits past the screen's own worst case.
	var deadline := (
		Time.get_ticks_msec() + int(explorer.loading_ui.MAX_LOADING_TIME_SECONDS * 1000.0)
	)
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


## Split out so the orientation predicate is unit-testable without a window.
static func _is_orientation_settled(window_size: Vector2i, expect_portrait: bool) -> bool:
	var is_window_portrait := window_size.y >= window_size.x
	return is_window_portrait == expect_portrait
