@tool
extends EditorPlugin


class ResourceTrackerDebuggerPlugin:
	extends EditorDebuggerPlugin

	var profiler_panel

	func _has_capture(prefix):
		# Return true if you wish to handle message with this prefix.
		return prefix == "resource_tracker"

	func _capture(message, data, _session_id):
		if message == "resource_tracker:report":
			var hash_id: String = data[0]
			var state: ResourceTrackerDebugger.ResourceTrackerState = data[1]
			var progress: String = data[2]
			var size: String = data[3]
			var metadata: String = data[4]

			profiler_panel.report_resource(hash_id, state, progress, size, metadata)
			return true
		if message == "resource_tracker:report_speed":
			profiler_panel.report_speed(data[0])
			return true
		return false

	func _setup_session(session_id):
		# Add a new tab in the debugger session UI containing a label.
		# Load and instantiate the profiler panel
		profiler_panel = (
			preload("res://addons/resource_tracker/resource_tracker_panel.tscn").instantiate()
		)
		profiler_panel.name = "Resource Tracker"

		var session = get_session(session_id)
		# Listens to the session started and stopped signals.
		session.started.connect(func(): print("Session started"))
		session.started.connect(func(): profiler_panel.clear_cache())
		session.stopped.connect(func(): print("Session stopped"))
		session.add_session_tab(profiler_panel)


var debugger = ResourceTrackerDebuggerPlugin.new()


func _enter_tree():
	add_debugger_plugin(debugger)


func _exit_tree():
	remove_debugger_plugin(debugger)
