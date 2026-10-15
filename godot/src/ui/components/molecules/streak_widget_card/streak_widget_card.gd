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
##      ├─ StreakDayRow (instance, unique name)
##      └─ Button_Claim (ClaimButton instance, unique name, hidden unless a reward is claimable —
##                        opens the full streak modal on tap (Figma "Widget-Discover" →
##                        "D3-Streak"/"D7-Streak"), it never claims in place: claim_action is left
##                        unassigned on purpose so pressed just fires, no CLAIMING/CLAIMED here)

@onready var streak_day_row: StreakDayRow = %StreakDayRow
@onready var button_claim: ClaimButton = %Button_Claim


func _ready() -> void:
	visibility_changed.connect(_on_visibility_changed)
	button_claim.pressed.connect(_on_button_claim_pressed)


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
## "D3-Streak"/"D7-Streak"). The widget itself just refreshes (claimable_days empties out, button
## hides) the next time refresh_visibility() runs after the modal closes.
func _on_button_claim_pressed() -> void:
	var day := Global.streak_manager.get_claimable_day()
	if day == 0:
		return
	Global.metrics.track_click_button("CLAIM", "LOGIN_STREAK_WIDGET_SHOW", "")
	var state := (
		StreakModal.State.D7_COMPLETE
		if day == StreakManager.FINAL_DAY
		else StreakModal.State.D3_COMPLETE
	)
	Global.streak_modal_controller.async_show_streak_modal(
		state, Global.streak_manager.get_progress_day(), StreakManager.is_unlinked_guest()
	)
