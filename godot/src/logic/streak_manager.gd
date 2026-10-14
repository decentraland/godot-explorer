class_name StreakManager
extends Node

## 7-day login streak business logic (issue #3018).
##
## A day is credited on a successful "jump into an experience" (Global.loading_finished) — never
## on app-open or browsing Discover. Day 3 is a checkpoint: losing the streak after reaching it
## resets progress back to day 3 (not 0), and the day-3 reward stays claimable regardless. Day 7
## ends the streak for the account forever once claimed. State is local-only (ConfigData) for
## this first pass — no backend, no multi-device sync.
##
## Modal display is decoupled from crediting: credit_jump_in() runs in-world and only queues a
## pending modal (streak_pending_modal); Discover is the only place a modal is shown ("Streak
## modals on Discover only — never in-world" per the issue), via take_pending_modal().

const DAY_SECONDS := 86400
const CHECKPOINT_DAY := 3
const FINAL_DAY := 7

const PENDING_WELCOME := "welcome"
const PENDING_PROGRESS := "progress"
const PENDING_LOST := "lost"
const PENDING_D3 := "d3"
const PENDING_D7 := "d7"


func _ready() -> void:
	Global.loading_finished.connect(_on_loading_finished)


func _on_loading_finished() -> void:
	if Global.config.streak_d7_claimed:
		return
	credit_jump_in()


## Implements the gap/floor/credit algorithm (see class doc + plan). Local-calendar-day
## granularity: two jump-ins on the same local day only ever credit once (gap == 0 is a no-op),
## which also makes this safe against loading_finished firing more than once for the same
## real-world jump-in.
func credit_jump_in() -> void:
	var config: ConfigData = Global.config
	var today := _local_midnight(int(Time.get_unix_time_from_system()))

	if config.streak_last_credit_day_unix == 0:
		# First-ever credit.
		config.streak_progress_day = 1
		config.streak_last_credit_day_unix = today
		config.streak_pending_modal = PENDING_WELCOME
		_finish_credit(config, 1)
		return

	var gap_days := (today - config.streak_last_credit_day_unix) / DAY_SECONDS
	if gap_days <= 0:
		# Already credited today (or a clock anomaly) — no-op.
		return

	config.streak_last_credit_day_unix = today

	if gap_days == 1:
		config.streak_progress_day += 1
		config.streak_pending_modal = _pending_for_progress()
	else:
		# Missed at least one day: floor is the checkpoint if it was ever reached, else 0, and
		# this jump-in itself is immediately credited as the day after the floor.
		var floor_day := CHECKPOINT_DAY if config.streak_progress_day >= CHECKPOINT_DAY else 0
		config.streak_progress_day = floor_day + 1
		config.streak_pending_modal = PENDING_LOST

	_finish_credit(config, config.streak_progress_day)


func _finish_credit(config: ConfigData, day: int) -> void:
	config.save_to_settings_file()
	Global.metrics.track_login_streak_day_counted(day, is_unlinked_guest())


## True for a thirdweb guest that hasn't linked an email yet — the account must upgrade before
## claiming a reward. NOT PlayerIdentity.is_guest, which is reserved for disposable LocalWallet
## sessions and stays false for a thirdweb guest (see is_thirdweb_guest's own doc comment in
## dcl_player_identity.rs) — checking that instead silently skipped the upgrade flow entirely.
static func is_unlinked_guest() -> bool:
	var identity = Global.player_identity
	return (
		identity != null
		and identity.is_thirdweb_guest()
		and not identity.is_thirdweb_guest_upgraded()
	)


## Defers entirely to get_claimable_day() rather than comparing the day to CHECKPOINT_DAY/
## FINAL_DAY directly — an exact-match comparison would stop offering the D3 claim the moment
## progress moves past day 3 without it being claimed (days 4-6 would silently fall through to
## the generic "GOT IT" progress state instead of re-prompting the still-pending reward).
func _pending_for_progress() -> String:
	match get_claimable_day():
		FINAL_DAY:
			return PENDING_D7
		CHECKPOINT_DAY:
			return PENDING_D3
	return PENDING_PROGRESS


## DEBUG ONLY — advances the REFERENCE CLOCK by one day, not the streak itself: it only rewinds
## streak_last_credit_day_unix, so get_pending_today_day() reports a new pending day exactly like
## a real calendar day turning over would. It deliberately does NOT credit anything — a real
## jump-in (Global.loading_finished → credit_jump_in()) is still required to actually complete
## that day, same as in production. Pressing this twice in a row with no jump-in in between
## simulates missing a day: the next real jump-in sees gap_days > 1 and resets the streak, exactly
## like it would if the player had actually been away two real days.
func debug_advance_day() -> void:
	var config: ConfigData = Global.config
	if config.streak_last_credit_day_unix <= 0:
		return
	config.streak_last_credit_day_unix -= DAY_SECONDS
	config.save_to_settings_file()


## DEBUG ONLY — the inverse of debug_advance_day(): steps progress back one day (floor 0) and
## un-claims D3/D7 once progress drops below their checkpoint, so the claim flow can be re-tested
## by advancing again. Also steps streak_last_credit_day_unix back a day to keep it consistent
## with the lowered progress, same persistence tradeoffs as debug_advance_day — except at the
## progress_day == 0 floor, where streak_last_credit_day_unix resets to 0 instead of one more day
## back. credit_jump_in() only ever takes its "first-ever credit" (WELCOME) branch when that's
## exactly 0; a real jump-in never leaves progress_day at 0 with a non-zero timestamp (every
## production write path lands on 1 or CHECKPOINT_DAY + 1), so without this reset the next jump-in
## here would see a multi-day gap instead and wrongly show "lost the streak" over a fresh WELCOME.
func debug_rewind_day() -> void:
	var config: ConfigData = Global.config
	if config.streak_progress_day <= 0:
		return
	config.streak_progress_day -= 1
	if config.streak_progress_day == 0:
		config.streak_last_credit_day_unix = 0
	elif config.streak_last_credit_day_unix > 0:
		config.streak_last_credit_day_unix -= DAY_SECONDS
	if config.streak_progress_day < CHECKPOINT_DAY:
		config.streak_d3_claimed = false
	if config.streak_progress_day < FINAL_DAY:
		config.streak_d7_claimed = false
	config.save_to_settings_file()


## Returns the queued modal state ("", "welcome", "progress", "lost", "d3", "d7") and clears it,
## so a modal is shown at most once per credit event. Call exactly once per Discover landing.
func take_pending_modal() -> String:
	var config: ConfigData = Global.config
	var pending := config.streak_pending_modal
	if pending != "":
		config.streak_pending_modal = ""
		config.save_to_settings_file()
	return pending


func get_progress_day() -> int:
	return Global.config.streak_progress_day


## What progress_day the player should see RIGHT NOW — can be lower than the persisted
## streak_progress_day when 2+ calendar days have passed with no jump-in. The actual reset still
## only commits on the next real jump-in (credit_jump_in() is the sole writer of
## streak_progress_day, same as always) — this just stops the UI from showing the stale
## pre-break days as checked in the meantime, so a dead streak reads as dead immediately instead
## of only once the player happens to jump in again. Mirrors credit_jump_in()'s own floor logic
## exactly, so what's shown here is exactly what a real jump-in right now would commit.
func get_effective_progress_day() -> int:
	var config: ConfigData = Global.config
	if config.streak_progress_day <= 0 or config.streak_last_credit_day_unix <= 0:
		return config.streak_progress_day
	var today := _local_midnight(int(Time.get_unix_time_from_system()))
	var gap_days := (today - config.streak_last_credit_day_unix) / DAY_SECONDS
	if gap_days <= 1:
		return config.streak_progress_day
	return CHECKPOINT_DAY if config.streak_progress_day >= CHECKPOINT_DAY else 0


## The day number (1..7) the player still needs to jump in for today to keep the streak going, or
## 0 if today's day is already credited (nothing pending) — e.g. the streak already ended, or the
## player simply hasn't opened a fresh calendar day since their last credit. Drives the Discover
## widget's "pending today" pill (Figma "Widget-Discover" vs "Widget-Discover-Complete": the
## current day reads as the plain TODAY pill — not yet checked — until an actual jump-in credits
## it, at which point it becomes DONE like any other completed day).
func get_pending_today_day() -> int:
	var config: ConfigData = Global.config
	if config.streak_d7_claimed:
		return 0
	if config.streak_progress_day <= 0:
		return 1
	var today := _local_midnight(int(Time.get_unix_time_from_system()))
	if config.streak_last_credit_day_unix == today:
		return 0
	var gap_days := (today - config.streak_last_credit_day_unix) / DAY_SECONDS
	if gap_days == 1:
		return config.streak_progress_day + 1
	# Missed at least one day — mirrors credit_jump_in()'s own floor logic, so the pending day
	# shown here is exactly the day a real jump-in right now would actually credit.
	var floor_day := CHECKPOINT_DAY if config.streak_progress_day >= CHECKPOINT_DAY else 0
	return floor_day + 1


## One-shot like take_pending_modal(): returns the pending day to show a "come finish today's
## mission" reminder for (Figma "D2-Streak" — same title/copy as the post-credit modal, but the
## row renders that day as still-pending, not checked), or 0 if there's no pending day right now
## or its reminder was already shown. Call at most once per Discover landing, and only if
## take_pending_modal() didn't already return something — a fresh credit's own celebration modal
## always wins over a reminder for the day after it.
func take_current_reminder() -> int:
	var pending := get_pending_today_day()
	if pending == 0:
		return 0
	var config: ConfigData = Global.config
	if config.streak_current_reminder_shown_day == pending:
		return 0
	config.streak_current_reminder_shown_day = pending
	config.save_to_settings_file()
	return pending


## Visible once the player has credited at least one day; after the D7 reward is claimed it
## stays visible for the remainder of that local day, then disappears for good.
func is_widget_visible() -> bool:
	var config: ConfigData = Global.config
	if config.streak_progress_day <= 0:
		return false
	if not config.streak_d7_claimed:
		return true
	return (
		_local_midnight(int(Time.get_unix_time_from_system())) == config.streak_last_credit_day_unix
	)


func is_d3_claimed() -> bool:
	return Global.config.streak_d3_claimed


func is_d7_claimed() -> bool:
	return Global.config.streak_d7_claimed


## The reward day ready to claim right now (3 or 7), or 0 if none is pending — drives the
## Discover widget's optional CLAIM button (Figma "Widget-Discover-ClaimPendient").
func get_claimable_day() -> int:
	var config: ConfigData = Global.config
	if config.streak_progress_day >= FINAL_DAY and not config.streak_d7_claimed:
		return FINAL_DAY
	if config.streak_progress_day >= CHECKPOINT_DAY and not config.streak_d3_claimed:
		return CHECKPOINT_DAY
	return 0


func mark_claimed(day: int) -> void:
	var config: ConfigData = Global.config
	if day == CHECKPOINT_DAY:
		config.streak_d3_claimed = true
	elif day == FINAL_DAY:
		config.streak_d7_claimed = true
	else:
		push_error("StreakManager.mark_claimed: unexpected day %d" % day)
		return
	config.save_to_settings_file()


## Local-vs-UTC wall-clock offset right now, in seconds (negative west of UTC). Recomputed per
## call rather than cached, since `Time.get_datetime_dict_from_unix_time` has no "utc" parameter
## to convert an arbitrary past timestamp directly — this is the offset Godot's "now" is at.
## Approximation: a stored timestamp from a different DST state than "now" can be off by up to an
## hour; acceptable for a local-only streak clock.
func _local_utc_offset_seconds() -> int:
	var utc_now := Time.get_unix_time_from_datetime_dict(Time.get_datetime_dict_from_system(true))
	var local_now := Time.get_unix_time_from_datetime_dict(
		Time.get_datetime_dict_from_system(false)
	)
	return local_now - utc_now


## Truncates a unix timestamp to local midnight: shift into local wall-clock, zero out the
## time-of-day fields, shift back.
func _local_midnight(unix_time: int) -> int:
	var offset := _local_utc_offset_seconds()
	var parts := Time.get_datetime_dict_from_unix_time(unix_time + offset)
	parts["hour"] = 0
	parts["minute"] = 0
	parts["second"] = 0
	return Time.get_unix_time_from_datetime_dict(parts) - offset
