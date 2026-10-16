class_name StreakDayPill
extends Control

## Single day indicator for the 7-day login streak (issue #3018). Shared by StreakDayRow, used by
## both the streak modal and the Discover widget. Figma: "DaysStreak" component (ATLAS component
## sheet "Slots 1-2-4-5-6" and "Slots 3 y 7 - Reward").
##
## Each state is a single baked PNG (box + icon + glow all in one export) — no StyleBoxFlat/
## PanelContainer, %Icon just swaps texture. Non-reward days use empty/current/complete; reward
## days (D3/D7) additionally distinguish "today is this reward day" from "already past it, still
## unclaimed" (REWARD_TODAY vs REWARD_PASSED — Figma ships these as two distinct exports, not one
## shared look). Reward slots read visually bigger than day slots by design: %Icon's own
## custom_minimum_size changes per state (56x68 for day states, 68x68 for reward states, both at
## the row's shared 68 height) rather than relying on each baked PNG's internal padding — the
## control's own box size is what drives the size difference.

enum State { FUTURE, TODAY, DONE, REWARD_LOCKED, REWARD_TODAY, REWARD_PASSED, REWARD_DONE }

const COLOR_DEFAULT := Color(0.9882353, 0.9882353, 0.9882353)  # DCL/Snow
const COLOR_REWARD_LOCKED := Color(1, 0.7882353, 0.3568627)  # #FFC95B
const COLOR_REWARD_DONE := Color(0.4509804, 1, 0.6862745)  # #73FFAF
const COLOR_CLAIMABLE := Color(0.8745098, 0.6117647, 1)  # #DF9CFF
const ICON_SIZE_DAY := Vector2(56, 68)
const ICON_SIZE_REWARD := Vector2(68, 68)
const REWARD_STATES := [
	State.REWARD_LOCKED, State.REWARD_TODAY, State.REWARD_PASSED, State.REWARD_DONE
]
# A reward day's entitlement is never lost once reached (see StreakManager.get_claimable_day) —
# REWARD_PASSED ("already past it, still unclaimed") is just as claimable as REWARD_TODAY, only
# without the "this is the one" highlight. REWARD_LOCKED (not reached yet) and REWARD_DONE
# (already claimed) are the only two reward states that are NOT currently claimable.
const CLAIMABLE_STATES := [State.REWARD_TODAY, State.REWARD_PASSED]

@export var texture_future: Texture2D
@export var texture_today: Texture2D
@export var texture_done: Texture2D
@export var texture_reward_locked: Texture2D
@export var texture_reward_today: Texture2D
@export var texture_reward_passed: Texture2D
@export var texture_reward_done: Texture2D
## Shown on every pill, day or reward, once the whole streak is claimed — replaces whatever
## texture this state's own `match` branch below would otherwise set.
@export var texture_all_complete: Texture2D

@onready var icon: TextureRect = %Icon
@onready var label_day: RichTextLabel = %Label_Day
@onready var claimable_background: TextureRect = %Claimable_Background


## @param all_claimed: true once both D3 and D7 are claimed — every day's icon and label (not
## just the reward days') switch to the finished-streak look (`texture_all_complete`, the same
## REWARD_DONE green) regardless of its own per-day state.
func set_state(day_number: int, state: State, all_claimed: bool = false) -> void:
	label_day.text = "D[b]%d[/b]" % day_number
	icon.custom_minimum_size = ICON_SIZE_REWARD if state in REWARD_STATES else ICON_SIZE_DAY
	claimable_background.visible = state in CLAIMABLE_STATES
	match state:
		State.FUTURE:
			icon.texture = texture_future
			label_day.modulate = COLOR_DEFAULT
		State.TODAY:
			icon.texture = texture_today
			label_day.modulate = COLOR_DEFAULT
		State.DONE:
			icon.texture = texture_done
			label_day.modulate = COLOR_DEFAULT
		State.REWARD_LOCKED:
			icon.texture = texture_reward_locked
			label_day.modulate = COLOR_REWARD_LOCKED
		State.REWARD_TODAY:
			icon.texture = texture_reward_today
			label_day.modulate = COLOR_CLAIMABLE
		State.REWARD_PASSED:
			icon.texture = texture_reward_passed
			label_day.modulate = COLOR_CLAIMABLE
		State.REWARD_DONE:
			icon.texture = texture_reward_done
			label_day.modulate = COLOR_REWARD_DONE
	if all_claimed:
		icon.texture = texture_all_complete
		label_day.modulate = COLOR_REWARD_DONE
