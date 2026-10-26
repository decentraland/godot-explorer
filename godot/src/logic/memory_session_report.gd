class_name MemorySessionReport
extends RefCounted
## The previous session's memory footprint as the Rust memory monitor kept it on
## disk (#3150). Read once at launch: attached to that session's exit report and
## sent as MEMORY_SESSION_REPORT, so healthy and killed sessions are both counted.

# memory_monitor::start() moves the record here before this session's first write.
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


static func consume() -> MemorySessionReport:
	var report := MemorySessionReport.new()
	if not FileAccess.file_exists(PREVIOUS_RECORD_PATH):
		return report
	report.raw_json = FileAccess.get_file_as_string(PREVIOUS_RECORD_PATH)
	DirAccess.remove_absolute(PREVIOUS_RECORD_PATH)
	var parsed = JSON.parse_string(report.raw_json)
	if parsed is Dictionary:
		report.record = parsed
	else:
		report.raw_json = ""
	return report


## Footprint fields for the exit at `exit_timestamp_ms`, or {} when the record
## cannot belong to it (written after that process died, so by a later run).
func footprint_for_exit(exit_timestamp_ms: int) -> Dictionary:
	var written_ms := int(record.get("written_ms", 0))
	if written_ms <= 0 or written_ms > exit_timestamp_ms:
		return {}
	var footprint := {"age_s": (exit_timestamp_ms - written_ms) / 1000}
	for key in EXIT_FIELDS:
		footprint[key] = int(record.get(key, -1))
	return footprint


func send() -> void:
	if raw_json.is_empty() or Global.metrics == null:
		return
	Global.metrics.track_memory_session_report(raw_json, exit_reason)
