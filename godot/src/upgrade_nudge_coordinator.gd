class_name UpgradeNudgeCoordinator
extends Node

## Decides when to show the aspirational guest-upgrade modal (issue #2372).
##
## Cadence (shown at most MAX_SHOWS times, and never once the guest has upgraded):
##   Modal 1 — second session
##   Modal 2 — 5 days after Modal 1
##   Modal 3 — 10 days after Modal 2
##
## The state (session count + shown count/date) lives in the persistent config
## (config_data.gd) so it survives across launches. Eligibility mirrors the
## guest_upgrade_card gating: portrait, thirdweb guest, not yet upgraded.
##
## The modal is a portrait design, but the in-world view is landscape (menus / discover
## are portrait). So rather than evaluate at world entry (loading_finished, landscape) we
## evaluate whenever the app is in a portrait context this session — i.e. the next time the
## player opens discover / a menu.

const DAY_SECONDS := 86400
const MODAL_2_DELAY_SECONDS := 5 * DAY_SECONDS
const MODAL_3_DELAY_SECONDS := 10 * DAY_SECONDS
const MAX_SHOWS := 3

var _evaluated_this_session: bool = false


func _ready() -> void:
	# Count this app launch once, before any cadence evaluation.
	var config: ConfigData = Global.get_config()
	config.session_count += 1
	config.save_to_settings_file()

	Global.orientation_changed.connect(_on_orientation_changed)
	Global.loading_finished.connect(_on_loading_finished)


func _on_orientation_changed(is_portrait: bool) -> void:
	if is_portrait:
		_try_evaluate()


func _on_loading_finished() -> void:
	_try_evaluate()


func _try_evaluate() -> void:
	# Evaluate at most once per launch, and only in a portrait context (the modal design).
	# In-world is landscape, so if we're not portrait yet we wait for the next portrait
	# moment (discover / a menu opening).
	if _evaluated_this_session:
		return
	if not Global.is_orientation_portrait():
		return
	_evaluated_this_session = true
	_async_evaluate()


func _async_evaluate() -> void:
	var config: ConfigData = Global.get_config()
	if not await _async_is_upgradeable_guest():
		print("[UpgradeNudge] skip: not an upgradeable thirdweb guest")
		return
	if not _is_due():
		print(
			(
				"[UpgradeNudge] skip: not due (session_count=%d shown=%d)"
				% [config.session_count, config.upgrade_modal_shown_count]
			)
		)
		return

	print("[UpgradeNudge] showing upgrade modal (session_count=%d)" % config.session_count)
	await Global.modal_manager.async_show_upgrade_modal()

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
	return not is_upgraded


# Cadence rule for the current shown_count (see class doc).
func _is_due() -> bool:
	var config: ConfigData = Global.get_config()
	var shown := config.upgrade_modal_shown_count
	if shown >= MAX_SHOWS:
		return false
	var now := int(Time.get_unix_time_from_system())
	match shown:
		0:
			return config.session_count >= 2
		1:
			return now >= config.upgrade_modal_last_shown_unix + MODAL_2_DELAY_SECONDS
		2:
			return now >= config.upgrade_modal_last_shown_unix + MODAL_3_DELAY_SECONDS
	return false
