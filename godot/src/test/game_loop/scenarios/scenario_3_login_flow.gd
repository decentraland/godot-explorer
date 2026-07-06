extends LoginFlowScenario

## Scenario 3: guest login via the standard Play-as-Guest flow. Run with a fixed
## `--guest-seed` and `--clear`: the seed-derived anchor recovers the SAME thirdweb wallet
## every run, so (once its profile exists) the flow takes the COMEBACK path. The stable
## ACCOUNT_HOME + COMEBACK screens are the golden targets; the transient ACCOUNT_GUEST_CREATE
## and the live 3D world are captured but excluded from goldens (non-deterministic).
##
## For the FRESH onboarding screens (AVATAR_CREATE / AVATAR_NAMING / DISCOVER_FTUE) see
## scenario 4, which forces a brand-new guest.


func _init() -> void:
	capture_prefix = "scenario3"


func async_run() -> GameLoopResult:
	return await async_drive_login_flow()
