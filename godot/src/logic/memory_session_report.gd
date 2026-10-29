class_name MemorySessionReport
extends RefCounted
## The previous session's memory footprint as the Rust memory monitor kept it on
## disk (#3150). Read once at launch: attached to that session's exit report and
## sent as MEMORY_SESSION_REPORT, so healthy and killed sessions are both counted.

const RECORD_PATH := "user://memory_footprint.json"
const PREVIOUS_RECORD_PATH := "user://memory_footprint.json.prev"
const EXIT_FIELDS := [
	"footprint_mb",
	"peak_footprint_mb",
	"rss_anon_mb",
	"swap_mb",
	"graphics_mb",
	"budget_mb",
	"level",
]

var record: Dictionary = {}
# Passed to Rust as written: re-serialising would turn its ints into floats.
var raw_json := ""
var exit_reason := ""


## Must run before the memory monitor starts (SceneManager enters the tree
## deferred), while RECORD_PATH still holds the session that just ended.
static func consume() -> MemorySessionReport:
	var report := MemorySessionReport.new()
	if FileAccess.file_exists(RECORD_PATH):
		DirAccess.rename_absolute(RECORD_PATH, PREVIOUS_RECORD_PATH)
	if not FileAccess.file_exists(PREVIOUS_RECORD_PATH):
		return report
	report.raw_json = FileAccess.get_file_as_string(PREVIOUS_RECORD_PATH)
	var parsed = JSON.parse_string(report.raw_json)
	if parsed is Dictionary:
		report.record = parsed
	else:
		report.raw_json = ""
		DirAccess.remove_absolute(PREVIOUS_RECORD_PATH)
	return report


## Footprint fields for `exit_info`, or {} when the record belongs to another
## process (pid mismatch) or was written after that process died.
func footprint_for_exit(exit_info: Dictionary) -> Dictionary:
	var exit_ms := int(exit_info.get("timestamp", 0))
	var written_ms := int(record.get("written_ms", 0))
	if written_ms <= 0 or written_ms > exit_ms:
		return {}
	var exit_pid := int(exit_info.get("pid", 0))
	if exit_pid > 0 and int(record.get("pid", 0)) != exit_pid:
		return {}
	var footprint := {"age_s": (exit_ms - written_ms) / 1000}
	for key in EXIT_FIELDS:
		footprint[key] = int(record.get(key, -1))
	return footprint


func send() -> void:
	if raw_json.is_empty() or Global.metrics == null:
		return
	Global.metrics.track_memory_session_report(raw_json, exit_reason)
	DirAccess.remove_absolute(PREVIOUS_RECORD_PATH)
