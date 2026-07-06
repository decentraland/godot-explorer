extends LoginFlowScenario

## Scenario 4: the FULL fresh-guest onboarding flow. Forces a brand-new guest so the standard
## Play-as-Guest flow runs the whole onboarding — ACCOUNT_HOME -> AVATAR_CREATE ->
## AVATAR_NAMING -> DISCOVER_FTUE -> world — instead of the COMEBACK shortcut (scenario 3).
##
## Run with `--clear` so no persisted session short-circuits it. The avatar and name are
## RANDOM every run, so AVATAR_CREATE / AVATAR_NAMING carry dynamic content: golden-compare
## them with a MASK (`<name>_mask.png`) that blacks out the avatar/name regions — the static
## UI (buttons, layout, labels) is still regression-checked, the random bits are skipped.


func _init() -> void:
	capture_prefix = "scenario4"


func async_run() -> GameLoopResult:
	# A random anchor -> a never-before-seen thirdweb wallet -> no server profile -> the fresh
	# onboarding path. Set BEFORE the flow presses Play-as-Guest (when get_device_anchor_id()
	# is read to mint the guest). randomize() reseeds the RNG so it differs every run.
	randomize()
	Global.forced_guest_seed_override = randi()
	_log("scenario=4 fresh guest, random seed override=%d" % Global.forced_guest_seed_override)
	return await async_drive_login_flow()
