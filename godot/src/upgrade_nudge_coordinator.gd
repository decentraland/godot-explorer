class_name UpgradeNudgeCoordinator
extends Node

## Decides when to show the aspirational guest-upgrade modal (issue #2372).
##
## Cadence is TIME-BASED and RELATIVE to the previous appearance (shown at most MAX_SHOWS times,
## and never once the guest has upgraded):
##   Modal 1 — MODAL_1_DELAY_HOURS after INSTALL (the anchor, config.upgrade_modal_first_seen_unix)
##   Modal 2 — MODAL_2_DELAY_HOURS after Modal 1 actually appeared (upgrade_modal_last_shown_unix)
##   Modal 3 — MODAL_3_DELAY_HOURS after Modal 2 actually appeared (upgrade_modal_last_shown_unix)
## So Modals 2 and 3 count from when the PREVIOUS modal was really shown, not from install — if the
## player doesn't open a portrait context right when one is due, the next deadline shifts with it.
##
## The state lives in the persistent config (config_data.gd) so it survives across launches.
## Eligibility mirrors the guest_upgrade_card gating: portrait, thirdweb guest, not yet upgraded.
##
## The modal is a portrait design, but the in-world view is landscape (menus / discover
## are portrait). So rather than evaluate at world entry (loading_finished, landscape) we
## evaluate whenever the app is in a portrait context this session — i.e. the next time the
## player opens discover / a menu.

# Modal 1's delay is measured from install; Modals 2 and 3 from the previous modal's appearance.
# PRODUCTION (active): 18h after install, then +5 days, then +5 more days.
const MODAL_1_DELAY_HOURS := 18
const MODAL_2_DELAY_HOURS := 5 * 24
const MODAL_3_DELAY_HOURS := 5 * 24
# TESTING: 1h after install, then +2h, then +2h (≈ appears at ~1h / 3h / 5h from install). To
# test the nudge in real time, swap the three consts above for these:
#   const MODAL_1_DELAY_HOURS := 1
#   const MODAL_2_DELAY_HOURS := 2
#   const MODAL_3_DELAY_HOURS := 2
const HOUR_SECONDS := 3600
const MAX_SHOWS := 3

var _evaluated_this_session: bool = false
# Tag -> Array of the Nodes currently outranking this nudge under that tag (e.g. a device-support
# modal #2936/#2935, or a pending login-streak modal #3018) — appended in suppress_until_cleared,
# removed by identity in resume. An array of owners, not a bool or a count: two callers can share
# a tag (discover.gd and streak_widget_card.gd both suppress "streak"), and either a bool or a
# plain count lets one caller's resume() clear or outlive the other's still-active hold (#2867
# review). Tracking owners also self-heals a leaked hold — a caller that suppressed and was freed
# before its own resume (e.g. torn down mid-wait by Global.return_to_discover()/sign-out) would
# otherwise block the nudge for the rest of the session; _prune_stale_suppressors() drops any
# owner that is no longer is_instance_valid() before every check.
# _try_evaluate() checks this without consuming _evaluated_this_session, so the real evaluation
# still runs once the last suppressor calls resume() (or is pruned).
var _suppressors: Dictionary = {}


func _ready() -> void:
	var config: ConfigData = Global.get_config()
	# Stamp the install anchor on the very first session; it never changes afterwards. Only Modal 1
	# is measured from it (Modals 2 and 3 count from the previous modal's actual appearance).
	if config.upgrade_modal_first_seen_unix == 0:
		config.upgrade_modal_first_seen_unix = int(Time.get_unix_time_from_system())
		print("[UpgradeNudge] anchored install at ", config.upgrade_modal_first_seen_unix)
	config.save_to_settings_file()

	Global.orientation_changed.connect(_on_orientation_changed)
	Global.loading_finished.connect(_on_loading_finished)


func _on_orientation_changed(is_portrait: bool) -> void:
	if is_portrait:
		_try_evaluate()


func _on_loading_finished() -> void:
	_try_evaluate()


## Called synchronously by a caller right before it shows a modal that should outrank this nudge
## (e.g. discover.gd before a device-support or login-streak modal), so the suppression is in
## place before that same call stack's Global.set_orientation_portrait() can fire this
## coordinator's own orientation-triggered evaluation. `tag` identifies the caller so concurrent
## suppressors don't clear each other's hold early — including two concurrent callers sharing the
## same tag. `owner` is whoever is responsible for the matching resume() call (normally the
## caller's own `self`), so a caller freed before it gets there doesn't hold this open forever.
func suppress_until_cleared(tag: String, owner: Node) -> void:
	var owners: Array = _suppressors.get(tag, [])
	owners.append(owner)
	_suppressors[tag] = owners


## Called once the modal that outranked this nudge (identified by the same `tag` and `owner`
## passed to suppress_until_cleared) has been dismissed — re-attempts the evaluation once no
## suppressor remains. A no-op if `tag`/`owner` wasn't actually suppressing (e.g. this session's
## evaluation already happened some other way).
func resume(tag: String, owner: Node) -> void:
	var owners: Array = _suppressors.get(tag, [])
	owners.erase(owner)
	if owners.is_empty():
		_suppressors.erase(tag)
	else:
		_suppressors[tag] = owners
	_prune_stale_suppressors()
	if _suppressors.is_empty():
		_try_evaluate()


## Drops any suppressor whose owner is no longer alive, so a caller freed mid-wait (teardown from
## Global.return_to_discover() or sign-out) cannot block the nudge for the rest of the session.
func _prune_stale_suppressors() -> void:
	for tag in _suppressors.keys():
		var owners: Array = _suppressors[tag]
		owners = owners.filter(func(owner): return is_instance_valid(owner))
		if owners.is_empty():
			_suppressors.erase(tag)
		else:
			_suppressors[tag] = owners


func _try_evaluate() -> void:
	# Evaluate at most once per launch, and only in a portrait context (the modal design).
	# In-world is landscape, so if we're not portrait yet we wait for the next portrait
	# moment (discover / a menu opening).
	if _evaluated_this_session:
		return
	if not Global.is_orientation_portrait():
		return
	_prune_stale_suppressors()
	# A higher-priority modal (device-support #2936/#2935, login-streak #3018) is showing — don't
	# consume this session's evaluation slot; the suppressor calls resume() once it's dismissed.
	if not _suppressors.is_empty():
		return
	_evaluated_this_session = true
	_async_evaluate()


func _async_evaluate() -> void:
	var config: ConfigData = Global.get_config()
	if not await _async_is_upgradeable_guest():
		print("[UpgradeNudge] skip: not an upgradeable thirdweb guest")
		return
	if not _is_due():
		var now := int(Time.get_unix_time_from_system())
		var due_at := _due_at_unix()
		var remaining_h := float(due_at - now) / HOUR_SECONDS
		print(
			(
				"[UpgradeNudge] skip: not due (shown=%d, due in %.2fh)"
				% [config.upgrade_modal_shown_count, remaining_h]
			)
		)
		return

	print("[UpgradeNudge] showing upgrade modal (shown=%d)" % config.upgrade_modal_shown_count)
	# Only advance the cadence if the modal was actually shown — otherwise a failed reveal
	# would burn a nudge slot and shift the next deadline. Also un-consume this session's
	# evaluation slot: a stale trigger (the guest upgraded, a teardown started) shouldn't cost
	# the only other chance this session has to show, which is the next portrait moment.
	if not await Global.modal_manager.async_show_upgrade_modal():
		print("[UpgradeNudge] modal failed to show; cadence not advanced")
		_evaluated_this_session = false
		return

	config.upgrade_modal_shown_count += 1
	config.upgrade_modal_last_shown_unix = int(Time.get_unix_time_from_system())
	config.save_to_settings_file()


# The nudge only applies to a thirdweb guest that hasn't upgraded yet. The "already
# upgraded" bit can't be known locally on a recovered session, so ask thirdweb once.
func _async_is_upgradeable_guest() -> bool:
	if Global.player_identity == null or not Global.player_identity.is_thirdweb_guest():
		return false
	var anchor: String = Global.get_device_anchor_id()
	var promise: Promise = Global.player_identity.async_refresh_thirdweb_upgrade_state(anchor)
	var result = await PromiseUtils.async_awaiter(promise)
	var is_upgraded: bool
	if result is PromiseError:
		# Couldn't confirm — fall back to the last-known cached flag.
		is_upgraded = Global.player_identity.is_thirdweb_guest_upgraded()
	else:
		is_upgraded = bool(result)
		# This can correct a stale cached flag on a recovered session; keep every badge/nudge
		# gated on it (Settings navbar badge, guest upgrade card, ...) in sync with the correction.
		Global.guest_upgrade_state_refreshed.emit(is_upgraded)
	return not is_upgraded


# Absolute unix time at which the modal for the current shown_count becomes due, or 0 when it
# can't be determined yet. Modal 1 counts from install (the anchor); Modals 2 and 3 count from
# the previous modal's actual appearance (last_shown), so each deadline is relative to the last.
func _due_at_unix() -> int:
	var config: ConfigData = Global.get_config()
	match config.upgrade_modal_shown_count:
		0:
			var anchor := config.upgrade_modal_first_seen_unix
			if anchor == 0:
				# Anchor not stamped yet (shouldn't happen after _ready).
				return 0
			return anchor + MODAL_1_DELAY_HOURS * HOUR_SECONDS
		1:
			return config.upgrade_modal_last_shown_unix + MODAL_2_DELAY_HOURS * HOUR_SECONDS
		2:
			return config.upgrade_modal_last_shown_unix + MODAL_3_DELAY_HOURS * HOUR_SECONDS
	return 0


# Cadence rule: the current modal is due once the wall clock passes its deadline (see class doc).
func _is_due() -> bool:
	var config: ConfigData = Global.get_config()
	if config.upgrade_modal_shown_count >= MAX_SHOWS:
		return false
	var due_at := _due_at_unix()
	if due_at == 0:
		return false
	return int(Time.get_unix_time_from_system()) >= due_at
