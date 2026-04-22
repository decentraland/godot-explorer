extends RefCounted

# Append-to-file logger for the double-jump + glide investigation. Each call
# timestamps the entry to millisecond resolution and flushes immediately so a
# crash or hang still leaves a usable log on disk.
#
# Output path is printed to the Godot console on first use. You can grep the
# log from a terminal:
#
#   grep -iE 'glide|jump' ~/Library/Application\ Support/Godot/app_userdata/decentraland/debug_dj_glide_*.log | tail -80
#
# Categories currently in use:
#   PLAYER  — jump / glide FSM transitions, input edges, ground-distance reads
#   AVATAR  — AnimationTree condition edges, glider-prop visibility changes
#
# To keep the signal-to-noise ratio high, instrumentation only logs on state
# transitions and input edges — NOT every physics frame.

static var _file: FileAccess = null
static var _path: String = ""


static func log(category: String, msg: String) -> void:
	if _file == null:
		_init_file()
		if _file == null:
			return
	var t: float = Time.get_ticks_msec() / 1000.0
	_file.store_line("[%08.3f] [%s] %s" % [t, category, msg])
	_file.flush()


static func session_path() -> String:
	return _path


static func _init_file() -> void:
	var ts: String = Time.get_datetime_string_from_system(false, false).replace(":", "-").replace(
		"T", "_"
	)
	var path: String = "user://debug_dj_glide_%s.log" % ts
	_file = FileAccess.open(path, FileAccess.WRITE)
	if _file == null:
		push_error("DebugLog: could not open %s" % path)
		return
	_path = ProjectSettings.globalize_path(path)
	print("[DebugLog] writing session to: ", _path)
	_file.store_line("# Session started at %s" % Time.get_datetime_string_from_system())
	_file.flush()
