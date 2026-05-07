class_name EngineLogTap
extends Logger

const MAX_LINES := 500 # Keep memory usage bounded.

var _mutex := Mutex.new()
var _lines: Array[String] = []
var _dropped_count: int = 0

func _log_message(message: String, error: bool) -> void:
	var prefix := "ERROR" if error else "LOG"
	_push_line("[%s] %s" % [prefix, message])


func _log_error(
	_function: String,
	file: String,
	line: int,
	code: String,
	rationale: String,
	_editor_notify: bool,
	error_type: int,
	script_backtraces: Array[ScriptBacktrace]
) -> void:
	var kind := "ERROR"
	match error_type:
		Logger.ERROR_TYPE_WARNING:
			kind = "WARNING"
		Logger.ERROR_TYPE_SCRIPT:
			kind = "SCRIPT"
		Logger.ERROR_TYPE_SHADER:
			kind = "SHADER"

	var msg := "[%s] %s:%d %s" % [kind, file, line, code]
	if not rationale.is_empty():
		msg += " | " + rationale

	for bt in script_backtraces:
		msg += "\n" + str(bt)

	_push_line(msg)


func _push_line(line: String) -> void:
	_mutex.lock()

	_lines.append(line)

	if _lines.size() > MAX_LINES:
		_lines.pop_front()
		_dropped_count += 1

	_mutex.unlock()


func get_lines() -> Array[String]:
	_mutex.lock()
	var copy := _lines.duplicate()
	_mutex.unlock()
	return copy


func clear() -> void:
	_mutex.lock()
	_lines.clear()
	_dropped_count = 0
	_mutex.unlock()


func get_dropped_count() -> int:
	_mutex.lock()
	var value := _dropped_count
	_mutex.unlock()
	return value
