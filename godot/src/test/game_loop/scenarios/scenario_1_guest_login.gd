extends GameLoopScenario

## Scenario 1: create a guest account and assert a wallet address materializes. Exercises
## the identity path end to end (device anchor -> thirdweb guest wallet), no UI.

## Max time to wait for the guest wallet/address to materialize (network + thirdweb).
const GUEST_LOGIN_TIMEOUT_SEC := 60.0


func async_run() -> GameLoopResult:
	if Global.player_identity == null:
		return GameLoopResult.fail("player_identity unavailable")

	# Already authenticated (e.g. persisted session) — count as pass.
	if not Global.player_identity.get_address_str().is_empty():
		return GameLoopResult.ok("already had address")

	var anchor: String = Global.get_device_anchor_id()
	_log("scenario=1 creating guest account (anchor len=%d)" % anchor.length())
	var promise: Promise = Global.player_identity.async_create_guest_account(anchor)
	var result = await PromiseUtils.async_awaiter(promise)
	if result is PromiseError:
		return GameLoopResult.fail("create_guest_account error: %s" % result.get_error())

	# The address may land a beat after the promise resolves (profile chain) — poll.
	var address := await async_wait_for(
		func() -> bool: return not Global.player_identity.get_address_str().is_empty(),
		GUEST_LOGIN_TIMEOUT_SEC
	)
	if not address:
		return GameLoopResult.fail("no address after %ss" % GUEST_LOGIN_TIMEOUT_SEC)
	return GameLoopResult.ok("address=%s" % Global.player_identity.get_address_str())
