class_name StreakDayRow
extends MarginContainer

## Renders the 7-day pill row for the login streak (issue #3018). Figma: "Streak panel"
## (file SKoahICmYk21D8GKErKxYK, node 21:5538 / 11:1305) — the bordered gradient card IS this
## component, reused verbatim by the streak modal and the Discover widget (neither wraps it in
## another card). Root is a MarginContainer with negative margins (lets the %Background
## NinePatchRect bleed past this node's own layout footprint) wrapping an inner HBoxContainer
## that lays out the 7 StreakDayPill children, instanced at runtime by render().

const DAY_COUNT := 7

@export var streak_day_pill_scene: PackedScene

var _pills: Array[StreakDayPill] = []

@onready var row: HBoxContainer = %Row


func _ensure_pills() -> void:
	if not _pills.is_empty():
		return
	for i in range(DAY_COUNT):
		if i > 0:
			# Grows/shrinks to absorb all extra row width so the pills themselves stay at their
			# own fixed size and the MarginContainer's side margins never stretch.
			var spacer := Control.new()
			spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
			row.add_child(spacer)
		var pill := streak_day_pill_scene.instantiate() as StreakDayPill
		row.add_child(pill)
		_pills.append(pill)


## @param progress_day: furthest day actually credited (0..7) — always DONE/REWARD_PASSED, never a
## "today" highlight.
## @param reward_days: 1-indexed days that grant a reward (D3 checkpoint + D7 final by default).
## @param claimed_days: 1-indexed reward days already claimed (REWARD_DONE's distinct green look).
## @param today_day: day to render as "current, not yet done" (TODAY/REWARD_TODAY); defaults to
## progress_day. The Discover widget passes StreakManager.get_pending_today_day() instead, since
## it's visible even before today's jump-in.
func render(
	progress_day: int,
	reward_days: Array[int] = [3, 7],
	claimed_days: Array[int] = [],
	today_day: int = -1
) -> void:
	if today_day < 0:
		today_day = progress_day
	_ensure_pills()
	# Every reward day claimed: the whole row's day labels — not just the reward days' — go green.
	var all_claimed := (
		not reward_days.is_empty()
		and reward_days.all(func(day: int) -> bool: return day in claimed_days)
	)
	for i in range(DAY_COUNT):
		var day_number := i + 1
		var is_reward_day := day_number in reward_days
		var is_today := day_number == today_day and today_day > 0 and today_day <= DAY_COUNT
		var state: StreakDayPill.State
		if day_number in claimed_days:
			state = StreakDayPill.State.REWARD_DONE
		elif day_number <= progress_day:
			if is_reward_day:
				# Distinct looks: "today is this reward day" vs "already past it, still
				# unclaimed" — Figma ships these as two separate exports, not one shared state.
				state = (
					StreakDayPill.State.REWARD_TODAY
					if is_today
					else StreakDayPill.State.REWARD_PASSED
				)
			else:
				# Always DONE, never TODAY — TODAY is only for the still-pending day below.
				state = StreakDayPill.State.DONE
		elif is_today:
			state = (
				StreakDayPill.State.REWARD_TODAY if is_reward_day else StreakDayPill.State.TODAY
			)
		elif is_reward_day:
			state = StreakDayPill.State.REWARD_LOCKED
		else:
			state = StreakDayPill.State.FUTURE
		_pills[i].set_state(day_number, state, all_claimed)
