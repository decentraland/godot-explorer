class_name NodeGuard
extends RefCounted

## Validity guard for node references that outlive an `await` (issue #2714).
##
## Every `await` hands control back to the engine, so between suspending and resuming a
## function another code path is free to destroy the node that function is about to touch:
## the user closes the modal, a realm change tears the UI down, sign-out reaps the tree.
## Reading a member of a freed instance afterwards is a use-after-free. Under the debug
## export template GDScript validates each object access and turns it into an "Attempted to
## access a freed object" error; the release template we ship to the stores compiles that
## validation out and reads the freed memory instead, which is a SIGSEGV on the phone.
##
## Two rules follow, and this class exists to make both cheap:
##
## 1. Never test a node reference for truthiness (`if node:`). A freed instance is not
##    null, so the test passes and the *next* line is the one that crashes. Ask
##    `is_instance_valid()` instead — it resolves the object id rather than the pointer.
## 2. Re-validate after every `await`, not only before it. A check that ran before
##    suspending says nothing about the state on resume.
##
## `is_alive()` is rule 2 plus telemetry. Swallowing the stale reference silently would fix
## the crash and hide how often the race actually fires, so every guarded site reports the
## first hit of the session to Sentry and Segment — enough to size the problem per build and
## per device, capped so a device that reopens a modal in a loop cannot flood either.

## Hits reported per site per session. The counter in `_hits` keeps rising past this;
## only the outbound event is capped.
const REPORTS_PER_SITE := 1

## site name -> times that site caught a freed node this session.
static var _hits: Dictionary = {}


## True when `node` is still a live instance. When it is not, records the site, reports it
## once per session, and returns false so the caller can bail out instead of crashing.
## `site` identifies the call site in the telemetry, e.g. "ModalManager.async_show_world_modal".
static func is_alive(node: Object, site: String) -> bool:
	if is_instance_valid(node):
		return true
	_report_stale(site)
	return false


## Times each guarded site has caught a freed node this session, keyed by site name.
## Empty on a healthy session; read by tests and the debug hub.
static func hit_counts() -> Dictionary:
	return _hits.duplicate()


static func _report_stale(site: String) -> void:
	var hits: int = int(_hits.get(site, 0)) + 1
	_hits[site] = hits
	if hits > REPORTS_PER_SITE:
		return

	var message := "NodeGuard: %s resumed on a freed node — skipped" % site
	push_warning(message)

	# Telemetry is off in asset-server / CI builds, where SentrySDK was never initialized.
	if not DclGlobal.is_telemetry_disabled():
		SentrySDK.capture_message(message, SentrySDK.LEVEL_WARNING)

	# Segment carries the same hit so the rate can be tracked per release alongside the
	# store crash rate, which is the number this guard is meant to move.
	if Global.metrics != null:
		Global.metrics.track_screen_viewed(
			"STALE_NODE_GUARD", JSON.stringify({"site": site, "platform": OS.get_name()})
		)
