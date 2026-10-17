class_name StreakWidgetCard
extends Control

## Compact Discover-feed widget for the login streak (issue #3018). Embedded as a sibling of
## %GuestUpgradeCard in discover.tscn's feed VBoxContainer. Figma: HIFI-WIP page, "Widget-Discover"
## family — the plain variant is just the 7-day pill row; "Widget-Discover-ClaimPendient" adds a
## full-width CLAIM button beneath it once a reward is ready, which %Button_Claim mirrors here.
##
## StreakDayRow IS the bordered card (Figma's "Streak panel" component is reused verbatim, not
## wrapped in another card):
##   StreakWidgetCard (MarginContainer, this script)
##   └─ VBoxContainer
##      ├─ StreakDayRow (instance, unique name) — also opens the modal on tap, same as Button_Claim
##      └─ Button_Claim (ClaimButton instance, unique name, hidden unless a reward is claimable —
##                        opens the full streak modal on tap (Figma "Widget-Discover" →
##                        "D3-Streak"/"D7-Streak"), it never claims in place: claim_action is left
##                        unassigned on purpose so pressed just fires, no CLAIMING/CLAIMED here)

@onready var streak_day_row: StreakDayRow = %StreakDayRow
@onready var button_claim: ClaimButton = %Button_Claim


func _ready() -> void:
	visibility_changed.connect(_on_visibility_changed)
	button_claim.pressed.connect(_on_button_claim_pressed)
	# Refreshes immediately when a claim resolves (e.g. inside the modal this widget opened),
	# rather than only the next time visibility/the modal's own close cycle re-renders this.
	Global.streak_manager.claim_state_changed.connect(refresh_visibility)
	_make_day_row_tappable()


## StreakDayRow's own pills/labels default to MOUSE_FILTER_STOP (plain Controls), which would
## swallow the tap before it ever reaches the row itself — set every descendant to IGNORE so the
## whole panel acts as one tap target. Runtime-only and scoped to this widget's own StreakDayRow
## instance; the shared scene (also used inside StreakModal, where pills aren't meant to be
## individually interactive either) is never touched.
func _make_day_row_tappable() -> void:
	var stack: Array[Node] = [streak_day_row]
	while not stack.is_empty():
		var node: Node = stack.pop_back()
		for child in node.get_children():
			if child is Control:
				child.mouse_filter = Control.MOUSE_FILTER_IGNORE
			stack.append(child)
	streak_day_row.gui_input.connect(_on_streak_day_row_gui_input)


## Fires on release, not press — CustomScrollRubberContainer (this widget sits inside
## ScrollContainer_Content, discover.tscn) only cancels a child's activation by eating the
## matching release once a drag crosses its own scroll_deadzone; the press itself is never
## intercepted. Firing on press would bypass that protection entirely and pop the modal the
## instant a vertical scroll starts with a finger on the row — the same reason Button_Claim
## (a real Button, same container) never fires early either.
## Also checks the release is still within the row's own bounds — Godot keeps routing touch
## events to whichever control received the press (gui.touch_focus) regardless of where the
## finger ends up, so a sub-deadzone drag that ends off the row would otherwise still open the
## modal; a real Button bails in that same case instead of firing.
func _on_streak_day_row_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch and not event.pressed:
		if Rect2(Vector2.ZERO, streak_day_row.size).has_point(event.position):
			_on_day_row_tapped()


## Same entry point as Button_Claim (see its own doc comment) — both components open the modal;
## tapping the row just reaches it even when nothing is claimable yet (Button_Claim is hidden
## then), landing on whatever state currently applies instead of always a claim screen.
func _on_day_row_tapped() -> void:
	Global.metrics.track_click_button("STREAK_DAY_ROW", "LOGIN_STREAK_WIDGET_SHOW", "")
	var pending_day := Global.streak_manager.get_pending_today_day()
	_async_show_streak_modal_and_wait(
		Global.streak_manager.compute_modal_state(pending_day),
		Global.streak_manager.get_effective_progress_day()
	)


## Mirrors GuestUpgradeCard's own _on_visibility_changed — visibility_changed also fires from an
## ancestor (e.g. Discover itself showing/hiding on tab switch), so guard with is_visible_in_tree().
func _on_visibility_changed() -> void:
	if is_visible_in_tree():
		Global.metrics.track_screen_viewed("LOGIN_STREAK_WIDGET_SHOW", "")


## Same method name/shape as GuestUpgradeCard's — discover.gd calls both the same way.
func refresh_visibility() -> void:
	visible = Global.streak_manager.is_widget_visible()
	if not visible:
		return
	var claimed_days: Array[int] = []
	if Global.streak_manager.is_d3_claimed():
		claimed_days.append(3)
	if Global.streak_manager.is_d7_claimed():
		claimed_days.append(7)
	streak_day_row.render(
		Global.streak_manager.get_effective_progress_day(),
		[3, 7],
		claimed_days,
		Global.streak_manager.get_pending_today_day()
	)

	var claimable_days := Global.streak_manager.get_claimable_days()
	button_claim.visible = not claimable_days.is_empty()
	if claimable_days.is_empty():
		return
	button_claim.text_normal = (
		"STREAK_CTA_CLAIM_REWARDS" if claimable_days.size() > 1 else "STREAK_CTA_CLAIM"
	)


## Opens the streak modal to the highest-priority claimable day's state (D7 over D3) rather than
## claiming here — the modal owns the actual claim UI/lifecycle (Figma "Widget-Discover" tap →
## "D3-Streak"/"D7-Streak").
func _on_button_claim_pressed() -> void:
	if Global.streak_manager.get_claimable_days().is_empty():
		return
	Global.metrics.track_click_button("CLAIM", "LOGIN_STREAK_WIDGET_SHOW", "")
	# pending_day is irrelevant here — get_claimable_days() is already non-empty, so
	# compute_modal_state() resolves from its first branch regardless of what's passed.
	_async_show_streak_modal_and_wait(
		Global.streak_manager.compute_modal_state(0), Global.streak_manager.get_progress_day()
	)


## Mirrors Discover._async_show_streak_modal_and_wait: suppresses the upgrade nudge while the
## modal is up and refreshes this widget once it closes, so claiming/viewing from here doesn't
## leave a stale pill row/CLAIM button up behind it.
func _async_show_streak_modal_and_wait(state: int, progress_day: int) -> void:
	Global.upgrade_nudge_coordinator.suppress_until_cleared("streak")
	var shown := await Global.streak_modal_controller.async_show_streak_modal(
		state, progress_day, StreakManager.is_unlinked_guest()
	)
	if shown and is_instance_valid(Global.streak_modal_controller.current_streak_modal):
		await Global.streak_modal_controller.current_streak_modal.tree_exited
	refresh_visibility()
	Global.upgrade_nudge_coordinator.resume("streak")
