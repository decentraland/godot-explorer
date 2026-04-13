@tool
extends EditorPlugin


class SceneLogDebugger:
	extends EditorDebuggerPlugin
	var panel: Control

	func _has_capture(prefix: String) -> bool:
		return prefix == "dcl_scene_log"

	func _capture(message: String, data: Array, _session_id: int) -> bool:
		if message == "dcl_scene_log:batch":
			if panel and data.size() > 0:
				var entries = JSON.parse_string(data[0])
				if entries is Array:
					panel.add_entries(entries)
			return true
		return false

	func _setup_session(session_id: int) -> void:
		panel = preload("res://addons/dcl_scene_logger/scene_log_panel.tscn").instantiate()
		panel.name = "Scene Logger"
		var session = get_session(session_id)
		session.started.connect(func(): panel.clear_entries())
		session.add_session_tab(panel)


var debugger := SceneLogDebugger.new()


func _enter_tree():
	add_debugger_plugin(debugger)


func _exit_tree():
	remove_debugger_plugin(debugger)
