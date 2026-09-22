class_name NodeGuard
extends RefCounted

## Validity check plus telemetry for a node reference whose lifetime this code does not own.
##
## Read REVIEW.md §5 "Freed-node access after `await`" first. On the release template a
## method call on a freed node is a SIGSEGV, and `await` is where references go stale. The
## engine protects `self` - a coroutine whose instance was freed is never resumed - and
## nothing else, so the rule is structural, not a check: async work that needs a node is a
## method of that node; a longer-lived owner re-resolves the node after the await instead
## of carrying it across; work is cancelled when its owner frees the node. None of that
## needs this class.
##
## `is_alive()` is for the one case left: an object whose lifetime is genuinely not ours -
## a remote player that can leave at any moment, a modal the user can close under a
## request. It is `is_instance_valid()` plus one report per site per session to Sentry and
## Segment, so the races we knowingly tolerate stay measured. Two facts it relies on:
##
## 1. A freed instance is not null: `if node:` passes and the next line crashes. Only
##    `is_instance_valid()` resolves the object id instead of the pointer.
## 2. A check before an `await` says nothing about the state on resume.
## 3. Valid is not the same as in the tree, and in the tree is not the same as ready.
##    `change_scene_to_file()` detaches a page at once but frees it later, so
##    `is_instance_valid()` stays true for a container that has already left the tree - and a
##    node added under a detached parent never runs `_ready`, so its `@onready` members stay
##    null and crash on first use. When what you need is a node you are about to build into or
##    read children from, ask `is_inside_tree()`; this class does not answer that.

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
