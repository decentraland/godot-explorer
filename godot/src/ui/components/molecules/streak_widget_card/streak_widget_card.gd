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
##                        self-manages its own CLAIMING.../CLAIMED lifecycle via claim_action,
##                        assigned fresh below each time the claimable day changes)

var _claimable_day: int = 0

@onready var streak_day_row: StreakDayRow = %StreakDayRow
@onready var button_claim: ClaimButton = %Button_Claim


func _ready() -> void:
	visibility_changed.connect(_on_visibility_changed)


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

	_claimable_day = Global.streak_manager.get_claimable_day()
	button_claim.visible = _claimable_day != 0
	if _claimable_day == 0:
		return
	var day := _claimable_day
	var is_guest := StreakManager.is_unlinked_guest()
	button_claim.claim_action = func() -> bool:
		Global.metrics.track_click_button("CLAIM", "LOGIN_STREAK_WIDGET_SHOW", "")
		return await Global.streak_modal_controller.async_claim_reward(day, is_guest)
