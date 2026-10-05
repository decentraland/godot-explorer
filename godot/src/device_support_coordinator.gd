class_name DeviceSupportCoordinator
extends RefCounted

## Decides whether this Android device is end-of-support (#2936, Play Store excluded) or below
## minimum spec (#2935, kept in the store but under the Galaxy A53 target), by RAM or by chipset
## (SoC). iOS has its own separate check — see DclIosPlugin.is_low_spec_iphone().
##
## The RAM rule is instant (local device info). The chipset rule asks the mobile-bff
## `/device-support?soc=` endpoint for this one SoC's decision, instead of shipping the client a
## full exclude/below-minspec list to check locally — see mobile-bff PR #93.

enum Status { OK, BELOW_MINSPEC, END_OF_SUPPORT }

const _MIN_RAM_MB := 4096
const _TIMEOUT_SECONDS := 5.0

## End-of-support modal re-show schedule (#2936), in days since first detected: first session,
## then day 5, then day 10, then day 30, then every _END_OF_SUPPORT_REPEAT_DAYS after that while
## the device remains excluded. Below-minspec (#2935) has no schedule — it shows once, ever.
const _END_OF_SUPPORT_SCHEDULE_DAYS: Array[int] = [0, 5, 10, 30]
const _END_OF_SUPPORT_REPEAT_DAYS := 30
const _DAY_SECONDS := 86400

# -1 = the /device-support lookup hasn't resolved yet this run. Only ever set once: device
# hardware and its server decision don't change mid-session.
static var _soc_status: int = -1


## Synchronous, best-effort answer: the instant RAM rule, or the cached server decision once
## async_check() has resolved it. Returns OK for the SoC part until then (fail-open) — callers
## that need the authoritative answer, not just a snapshot, should await async_check() instead.
static func check() -> Status:
	if not DclAndroidPlugin.is_available():
		return Status.OK
	if _is_ram_excluded():
		return Status.END_OF_SUPPORT
	return (Status.OK if _soc_status == -1 else _soc_status) as Status


## Authoritative answer: the instant RAM rule short-circuits without a network call; otherwise
## awaits the per-SoC server lookup (once per run — later calls return the cached result).
## Emits Global.device_support_status_resolved the first time the SoC lookup settles, so a UI
## element that read check() before this resolved (e.g. settings_warning.gd, instantiated lazily
## whenever Settings happens to be opened) can re-check instead of staying frozen on the
## fail-open default.
static func async_check() -> Status:
	if not DclAndroidPlugin.is_available():
		return Status.OK
	if _is_ram_excluded():
		return Status.END_OF_SUPPORT
	if _soc_status == -1:
		_soc_status = await _async_fetch_soc_status()
		Global.device_support_status_resolved.emit()
	return _soc_status as Status


## True if the end-of-support modal is due again, per the day-based schedule above.
## `first_detected_unix` is the once-ever anchor stamped in lobby.gd the first time this device
## was found excluded; <= 0 means it hasn't been stamped yet, which shouldn't happen by the time
## this is checked (lobby.gd runs before discover.gd can). Fails closed to "not due" rather than
## "show" — same direction as every other fail-open in this feature (the SoC fetch itself fails
## to OK, not to a flagged status): with no real anchor to measure days_elapsed against, the safe
## default is to stay quiet, not to re-show on every single call.
static func is_end_of_support_modal_due(shown_count: int, first_detected_unix: int) -> bool:
	if first_detected_unix <= 0:
		return false
	return _days_elapsed(first_detected_unix) >= _end_of_support_next_due_day(shown_count)


## The shown_count to persist right after showing the modal today: jumps past every schedule
## tier the real calendar gap since first_detected_unix already satisfies, rather than advancing
## one tier at a time. Without this, a device that returns after a long absence (say 100 days)
## would need one relaunch per skipped tier (day 5, then 10, then 30, then 60...) before the
## schedule catches up to the present — this makes it catch up in the single show that happens
## today.
static func end_of_support_catch_up_shown_count(first_detected_unix: int) -> int:
	var days_elapsed := _days_elapsed(first_detected_unix)
	var count := 0
	while _end_of_support_next_due_day(count) <= days_elapsed:
		count += 1
	return count


static func _days_elapsed(first_detected_unix: int) -> int:
	return (int(Time.get_unix_time_from_system()) - first_detected_unix) / _DAY_SECONDS


static func _end_of_support_next_due_day(shown_count: int) -> int:
	if shown_count < _END_OF_SUPPORT_SCHEDULE_DAYS.size():
		return _END_OF_SUPPORT_SCHEDULE_DAYS[shown_count]
	var extra_shows := shown_count - (_END_OF_SUPPORT_SCHEDULE_DAYS.size() - 1)
	return _END_OF_SUPPORT_SCHEDULE_DAYS[-1] + _END_OF_SUPPORT_REPEAT_DAYS * extra_shows


static func _is_ram_excluded() -> bool:
	var ram_mb := DclAndroidPlugin.get_total_ram_mb()
	return ram_mb >= 0 and ram_mb < _MIN_RAM_MB


static func _async_fetch_soc_status() -> int:
	var soc := _normalized_soc()
	if soc.is_empty():
		return Status.OK

	var url := "%s?soc=%s" % [String(DclUrls.device_support()), soc.uri_encode()]
	var http_fn := func() -> Promise:
		return Global.http_requester.request_json(url, HTTPClient.METHOD_GET, "", {})
	var timeout_fn := func() -> Promise:
		var p := Promise.new()
		var tree := Engine.get_main_loop() as SceneTree
		tree.create_timer(_TIMEOUT_SECONDS).timeout.connect(
			func(): p.reject("device_support: timeout")
		)
		return p

	var result = await PromiseUtils.async_race([http_fn, timeout_fn])
	if result is PromiseError:
		# Fail-open, like FeatureFlags: offline/slow cold starts are expected, not error-level.
		push_warning(
			"[DeviceSupportCoordinator] fetch failed (fail-open): " + str(result.get_error())
		)
		return Status.OK

	var json = result.get_string_response_as_json()
	return _parse_decision(json)


## Extracts the decision from the mobile-bff response:
## `{"ok": true, "data": {"soc": "...", "decision": "exclude"|"below-minspec"|"keep"}}`.
## Any shape mismatch or unrecognized decision fails open to OK.
static func _parse_decision(json) -> int:
	if typeof(json) != TYPE_DICTIONARY or not json.get("ok", false):
		return Status.OK
	var data = json.get("data", {})
	if typeof(data) != TYPE_DICTIONARY:
		return Status.OK
	match data.get("decision", "keep"):
		"exclude":
			return Status.END_OF_SUPPORT
		"below-minspec":
			return Status.BELOW_MINSPEC
		_:
			return Status.OK


## Build.SOC_MODEL (API 31+) is the most reliable id when present; ro.board.platform and
## Build.HARDWARE are fallbacks for older devices where it's always empty.
static func _normalized_soc() -> String:
	var candidates := [
		DclAndroidPlugin.get_soc_model(),
		DclAndroidPlugin.get_board_platform(),
		DclAndroidPlugin.get_hardware(),
	]
	for raw in candidates:
		var id := _normalize(raw)
		if not id.is_empty():
			return id
	return ""


static func _normalize(raw: String) -> String:
	var s := raw.strip_edges().to_upper()
	for prefix in ["QUALCOMM ", "MEDIATEK ", "QTI "]:
		s = s.trim_prefix(prefix)
	return s
