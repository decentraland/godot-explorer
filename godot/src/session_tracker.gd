extends Node

## Emits the `App Opened` Segment event and carries the session chain across process deaths.
##
## The OS kills the app without running any of our code, so the end of a session is never
## observed directly — only the last moment we know it was alive, which this node keeps writing
## to disk. The rest of the design follows from that one fact.

const STATE_PATH := "user://session_state.cfg"
const STATE_PATH_TMP := "user://session_state.cfg.tmp"

## Bounds how stale the liveness mark can be when the OS kills us mid-session: worst case we
## under-report a gap by this much.
const HEARTBEAT_SECONDS := 30.0

## Warm returns quicker than this are the app coming back from its own dialogs — permission
## prompt, browser auth hop, share sheet — rather than the user. NOT the session cutoff: that
## one lives in SQL over `seconds_since_last_seen`, so it can change without a client release.
const WARM_OPEN_FLOOR_SECONDS := 30

## Android delivers the launch intent asynchronously, after _ready, so an open waits this long
## for a deep link before concluding nothing brought the user in.
const DEEP_LINK_SETTLE_SECONDS := 1.5

const START_COLD := "cold"
const START_WARM := "warm"

const TRIGGER_DEEP_LINK := "deeplink"
const TRIGGER_ICON := "icon"
const TRIGGER_PUSH := "push"

var _enabled := false

# Where the app was last seen alive. Advanced by every _write_state, so a warm open measures
# against this process's own last mark and a cold one against the previous process's.
var _prev_session_id := ""
var _prev_last_seen := 0

var _heartbeat_left := 0.0

# An open held back until its trigger is known. Captured at detection time, not at emit time:
# the settle window is long enough for the liveness mark to move underneath it.
var _pending_start_kind := ""
var _pending_prev_session_id := ""
var _pending_seconds_since := -1
var _pending_settle_left := 0.0


func _ready() -> void:
	set_process(false)
	# Same telemetry gate Global applies to Metrics: with nobody to emit to, the disk writes
	# would buy nothing. Global itself is checked because the script validator instantiates
	# autoloads without one.
	if Global == null or Global.testing_scene_mode or Global.metrics == null:
		return
	_enabled = true
	set_process(true)
	_load_previous_state()
	_begin_open(START_COLD)
	_write_state()


func _process(delta: float) -> void:
	if not _enabled:
		return
	if _pending_start_kind != "":
		_pending_settle_left -= delta
		if _pending_settle_left <= 0.0:
			_emit_open(_trigger_from_launch_deep_link())
	_heartbeat_left -= delta
	if _heartbeat_left <= 0.0:
		_write_state()


func _notification(what: int) -> void:
	if not _enabled:
		return
	if what == NOTIFICATION_APPLICATION_FOCUS_OUT:
		# Pins the mark to the moment we actually left rather than up to a heartbeat earlier.
		# On mobile _process stops here, so this is the last write until we come back.
		_write_state()
	elif what == NOTIFICATION_APPLICATION_FOCUS_IN:
		_begin_open(START_WARM)


## Resolve a pending open's trigger. Called by DeepLinkRouter for every deep link on both
## platforms and in both start kinds; a link arriving mid-session finds no open and is ignored.
func notify_deep_link(params: Dictionary) -> void:
	if not _enabled or _pending_start_kind == "":
		return
	var is_push: bool = not str(params.get("push_campaign_id", "")).strip_edges().is_empty()
	_emit_open(TRIGGER_PUSH if is_push else TRIGGER_DEEP_LINK)


## What launched a cold open, read straight off Global rather than waiting to be told.
##
## The autoloads that route the deep link are ready before this one, so on a push tap the link
## is already routed by the time this node starts listening — notify_deep_link finds nobody
## home. Only cold opens may read it: the object lives for the whole process, so on a warm open
## it could still hold a link from an hour ago.
func _trigger_from_launch_deep_link() -> String:
	if _pending_start_kind != START_COLD:
		return TRIGGER_ICON
	var params: Dictionary = Global.deep_link_obj.params
	if params.is_empty():
		return TRIGGER_ICON
	if not str(params.get("push_campaign_id", "")).strip_edges().is_empty():
		return TRIGGER_PUSH
	return TRIGGER_DEEP_LINK


## Start an open, held until a deep link resolves it or the settle window runs out.
func _begin_open(start_kind: String) -> void:
	if _pending_start_kind != "":
		return
	var seconds_since := _seconds_since_last_seen()
	# A negative (unknown) gap is below the floor too, so a clock that moved backwards never
	# manufactures a warm open.
	if start_kind == START_WARM and seconds_since < WARM_OPEN_FLOOR_SECONDS:
		return
	_pending_start_kind = start_kind
	_pending_prev_session_id = _prev_session_id
	_pending_seconds_since = seconds_since
	_pending_settle_left = DEEP_LINK_SETTLE_SECONDS


func _emit_open(trigger: String) -> void:
	var start_kind := _pending_start_kind
	_pending_start_kind = ""
	if Global.metrics == null:
		return
	print(
		(
			"[SESSION] opened start=%s trigger=%s prev=%s since=%ds"
			% [start_kind, trigger, _pending_prev_session_id, _pending_seconds_since]
		)
	)
	Global.metrics.track_app_opened(
		start_kind, trigger, _pending_prev_session_id, _pending_seconds_since
	)


## Wall clock, because it is the only one that survives the process — which also means it
## inherits the user's ability to move it. A backwards jump reads as unknown, not as zero.
func _seconds_since_last_seen() -> int:
	if _prev_last_seen <= 0:
		return -1
	var delta := int(Time.get_unix_time_from_system()) - _prev_last_seen
	return delta if delta >= 0 else -1


func _load_previous_state() -> void:
	var file := ConfigFile.new()
	# Missing is the first launch ever; unparseable is a torn write we simply don't trust.
	if file.load(STATE_PATH) != OK:
		return
	_prev_session_id = str(file.get_value("session", "id", ""))
	_prev_last_seen = int(file.get_value("session", "last_seen", 0))


func _write_state() -> void:
	_heartbeat_left = HEARTBEAT_SECONDS
	var now := int(Time.get_unix_time_from_system())
	_prev_session_id = Global.session_id
	_prev_last_seen = now

	var file := ConfigFile.new()
	file.set_value("session", "id", Global.session_id)
	file.set_value("session", "last_seen", now)
	# Written aside and renamed in: a kill between truncating and writing would otherwise cost
	# the previous session's mark, which is the one thing this file exists to hold.
	if file.save(STATE_PATH_TMP) != OK:
		return
	DirAccess.rename_absolute(STATE_PATH_TMP, STATE_PATH)
