@tool
extends Control

enum TimelineMode { FIT, EXPLORE, LIVE }

const PERIOD_VALUES = [15000, 30000, 60000, 120000]  # in milliseconds

var resource_statuses = {}

@onready var tree = %Tree
@onready var option_box_filter = %OptionBox_Filter
@onready var label_speed = %Label_Speed
@onready var label_info = %LabelInfo
@onready var waterfall_chart = %WaterfallChart
@onready var view_mode_button = %OptionButton_ViewMode
@onready var timeline_mode_button = %OptionButton_TimelineMode
@onready var period_button = %OptionButton_Period
@onready var label_period = %LabelPeriod


func _ready():
	# Initialize the Tree node
	tree.create_item()  # Create a root item
	tree.columns = 7
	tree.set_column_title(0, "Type")
	tree.set_column_title(1, "Hash ID")
	tree.set_column_title(2, "State")
	tree.set_column_title(3, "Progress")
	tree.set_column_title(4, "Size")
	tree.set_column_title(5, "Metadata")
	tree.set_column_title(6, "Time")

	# Cache for tree items
	resource_statuses = {}

	# Update the resource status regularly
	set_process(true)


func _process(_delta):
	update_resource_status()
	if waterfall_chart:
		waterfall_chart.queue_redraw()


func report_speed(speed: String):
	label_info.hide()
	label_speed.text = speed


func _record_state_transition(resource: Dictionary, new_state: int, current_tick: int) -> void:
	var history: Array = resource.get("state_history", [])

	# Close previous state if exists
	if history.size() > 0:
		var last_entry = history[history.size() - 1]
		if last_entry["end_tick"] == -1:
			last_entry["end_tick"] = current_tick

	# Check if this is a terminal state (resource is done loading)
	var is_terminal_state = (
		new_state == ResourceTrackerDebugger.ResourceTrackerState.FINISHED
		or new_state == ResourceTrackerDebugger.ResourceTrackerState.FAILED
		or new_state == ResourceTrackerDebugger.ResourceTrackerState.DELETED
		or new_state == ResourceTrackerDebugger.ResourceTrackerState.TIMEOUT
	)

	# Add new state entry - close immediately if terminal state
	var end_tick = current_tick if is_terminal_state else -1
	history.append({"state": new_state, "start_tick": current_tick, "end_tick": end_tick})

	resource["state_history"] = history


func report_resource(
	hash_id: String,
	state: ResourceTrackerDebugger.ResourceTrackerState,
	progress: String,
	size: String,
	metadata: String,
	resource_type: String = ""
):
	var current_tick = Time.get_ticks_msec()

	# Check if the resource already exists in the cache
	if resource_statuses.has(hash_id):
		# Update existing resource
		var resource = resource_statuses[hash_id]
		var old_state = resource["state"]

		# Check if current state is a terminal state (resource already done)
		var is_terminal_state = (
			old_state == ResourceTrackerDebugger.ResourceTrackerState.FINISHED
			or old_state == ResourceTrackerDebugger.ResourceTrackerState.FAILED
			or old_state == ResourceTrackerDebugger.ResourceTrackerState.DELETED
		)

		# Update state if:
		# - new state is greater than current, OR
		# - new state is DELETED (always accept), OR
		# - current state is TIMEOUT (recover from timeout on any update)
		# BUT: Never let TIMEOUT overwrite terminal states (race condition fix)
		var should_update = (
			old_state < state
			or state == ResourceTrackerDebugger.ResourceTrackerState.DELETED
			or old_state == ResourceTrackerDebugger.ResourceTrackerState.TIMEOUT
		)
		var is_timeout_after_terminal = (
			state == ResourceTrackerDebugger.ResourceTrackerState.TIMEOUT and is_terminal_state
		)

		if should_update and not is_timeout_after_terminal:
			# Record state transition for waterfall chart
			if old_state != state:
				_record_state_transition(resource, state, current_tick)
			resource["state"] = state

		if not progress.is_empty():
			resource["progress"] = progress

		if not size.is_empty():
			resource["size"] = size

		if not metadata.is_empty():
			resource["metadata"] = metadata

		if not resource_type.is_empty():
			resource["resource_type"] = resource_type

		var elapsed: float = float(current_tick - resource["start_tick"]) / 1000.0
		resource["elapsed"] = snappedf(elapsed, 0.01)

		resource_statuses[hash_id] = resource
	else:
		# Check if this is a terminal state
		var is_terminal = (
			state == ResourceTrackerDebugger.ResourceTrackerState.FINISHED
			or state == ResourceTrackerDebugger.ResourceTrackerState.FAILED
			or state == ResourceTrackerDebugger.ResourceTrackerState.DELETED
			or state == ResourceTrackerDebugger.ResourceTrackerState.TIMEOUT
		)
		var initial_end_tick = current_tick if is_terminal else -1

		# Add new resource with state history
		resource_statuses[hash_id] = {
			"hash_id": hash_id,
			"state": state,
			"progress": progress,
			"size": size,
			"metadata": metadata,
			"resource_type": resource_type,
			"start_tick": current_tick,
			"elapsed": 0.0,
			"item": null,  # Placeholder for the TreeItem
			"state_history":
			[{"state": state, "start_tick": current_tick, "end_tick": initial_end_tick}]
		}

	# Update the UI after modifying the resource status
	_update_ui()

	# Update waterfall chart
	if waterfall_chart:
		waterfall_chart.set_resources(resource_statuses)


func update_resource_status():
	# This can be used to periodically update the status if needed
	pass


func clear_cache():
	resource_statuses.clear()
	clear()
	if waterfall_chart:
		waterfall_chart.clear_resources()


func clear():
	tree.clear()
	tree.create_item()  # Create a root item


func _update_ui():
	var root = tree.get_root()
	for hash_id in resource_statuses.keys():
		var resource = resource_statuses[hash_id]
		var item = null

		if option_box_filter.get_selected_id() != 0:
			if resource["state"] != (option_box_filter.get_selected_id() - 1):
				if resource["item"] != null:
					item = resource["item"]
					item.free()
					resource_statuses[hash_id]["item"] = null
				continue

		if resource["item"] != null:
			item = resource["item"]
		else:
			item = tree.create_item(root, 0)
			resource["item"] = item

		item.set_text(0, resource.get("resource_type", ""))
		item.set_text(1, hash_id)
		item.set_text(2, ResourceTrackerDebugger.get_resource_state_string(resource["state"]))
		item.set_text(3, resource["progress"])
		item.set_text(4, resource["size"])
		item.set_text(5, resource["metadata"])
		item.set_text(6, str(resource["elapsed"]) + "s")

		resource_statuses[hash_id] = resource


func _on_option_box_filter_item_selected(_index):
	clear()
	for hash_id in resource_statuses.keys():
		resource_statuses[hash_id]["item"] = null

	_update_ui()

	# Update waterfall with filter
	if waterfall_chart:
		var filter_state = (
			option_box_filter.get_selected_id() - 1
			if option_box_filter.get_selected_id() != 0
			else -1
		)
		waterfall_chart.set_filter_state(filter_state)


func _on_tree_item_mouse_selected(_position, mouse_button_index):
	if mouse_button_index != MOUSE_BUTTON_RIGHT:
		return

	var selected: TreeItem = tree.get_selected()
	if selected:
		var text = selected.get_text(0)
		DisplayServer.clipboard_set(text)


func _on_view_mode_button_item_selected(index: int):
	if index == 0:
		tree.show()
		if waterfall_chart:
			waterfall_chart.hide()
	else:
		tree.hide()
		if waterfall_chart:
			waterfall_chart.show()


func _on_timeline_mode_selected(index: int):
	var mode = index as TimelineMode
	var period_enabled = mode != TimelineMode.FIT

	period_button.disabled = not period_enabled
	label_period.modulate.a = 1.0 if period_enabled else 0.5

	if waterfall_chart:
		var period_ms = PERIOD_VALUES[period_button.get_selected_id()]
		waterfall_chart.set_timeline_mode(mode, period_ms)


func _on_period_selected(index: int):
	if waterfall_chart:
		var mode = timeline_mode_button.get_selected_id() as TimelineMode
		var period_ms = PERIOD_VALUES[index]
		waterfall_chart.set_timeline_mode(mode, period_ms)


func _on_clear_pressed():
	clear_cache()


func _on_export_report_pressed():
	var report = _generate_markdown_report()
	DisplayServer.clipboard_set(report)
	print("Resource Tracker report copied to clipboard!")


func _generate_markdown_report() -> String:
	var lines: Array[String] = []
	var timestamp = Time.get_datetime_string_from_system()

	lines.append("# Resource Tracker Report")
	lines.append("")
	lines.append("Generated: " + timestamp)
	lines.append("")

	# Summary statistics
	lines.append("## Summary")
	lines.append("")

	var total_resources = resource_statuses.size()
	var state_counts: Dictionary = {}
	var type_counts: Dictionary = {}

	for hash_id in resource_statuses.keys():
		var resource = resource_statuses[hash_id]
		var state = resource["state"]
		var res_type = resource.get("resource_type", "unknown")

		state_counts[state] = state_counts.get(state, 0) + 1
		type_counts[res_type] = type_counts.get(res_type, 0) + 1

	lines.append("- **Total Resources:** " + str(total_resources))
	lines.append("")

	# State breakdown
	lines.append("### By State")
	lines.append("")
	lines.append("| State | Count |")
	lines.append("|-------|-------|")
	for state in state_counts.keys():
		var state_name = ResourceTrackerDebugger.get_resource_state_string(state)
		lines.append("| " + state_name + " | " + str(state_counts[state]) + " |")
	lines.append("")

	# Type breakdown
	lines.append("### By Type")
	lines.append("")
	lines.append("| Type | Count |")
	lines.append("|------|-------|")
	var sorted_types = type_counts.keys()
	sorted_types.sort()
	for res_type in sorted_types:
		var display_type = res_type if not res_type.is_empty() else "(none)"
		lines.append("| " + display_type + " | " + str(type_counts[res_type]) + " |")
	lines.append("")

	# Detailed resource list
	lines.append("## Resources")
	lines.append("")
	lines.append("| Type | Hash | State | Size | Time | Metadata |")
	lines.append("|------|------|-------|------|------|----------|")

	# Sort resources by start_tick
	var sorted_resources: Array = []
	for hash_id in resource_statuses.keys():
		var resource = resource_statuses[hash_id]
		sorted_resources.append({"hash_id": hash_id, "resource": resource})

	sorted_resources.sort_custom(
		func(a, b): return a["resource"]["start_tick"] < b["resource"]["start_tick"]
	)

	for item in sorted_resources:
		var hash_id = item["hash_id"]
		var resource = item["resource"]
		var res_type = resource.get("resource_type", "")
		var state_name = ResourceTrackerDebugger.get_resource_state_string(resource["state"])
		var size = resource.get("size", "")
		var elapsed = str(resource.get("elapsed", 0.0)) + "s"
		var metadata = resource.get("metadata", "")

		# Truncate hash for readability
		var short_hash = hash_id.left(12) + "..." if hash_id.length() > 15 else hash_id

		lines.append(
			(
				"| "
				+ res_type
				+ " | `"
				+ short_hash
				+ "` | "
				+ state_name
				+ " | "
				+ size
				+ " | "
				+ elapsed
				+ " | "
				+ metadata
				+ " |"
			)
		)

	lines.append("")

	# State history for failed/timeout resources
	var problem_resources: Array = []
	for hash_id in resource_statuses.keys():
		var resource = resource_statuses[hash_id]
		var state = resource["state"]
		if (
			state == ResourceTrackerDebugger.ResourceTrackerState.FAILED
			or state == ResourceTrackerDebugger.ResourceTrackerState.TIMEOUT
		):
			problem_resources.append({"hash_id": hash_id, "resource": resource})

	if problem_resources.size() > 0:
		lines.append("## Problem Resources (Failed/Timeout)")
		lines.append("")

		for item in problem_resources:
			var hash_id = item["hash_id"]
			var resource = item["resource"]

			lines.append("### `" + hash_id + "`")
			lines.append("")
			lines.append("- **Type:** " + resource.get("resource_type", "unknown"))
			lines.append(
				(
					"- **State:** "
					+ ResourceTrackerDebugger.get_resource_state_string(resource["state"])
				)
			)
			lines.append("- **Metadata:** " + resource.get("metadata", ""))
			lines.append("")

			var history: Array = resource.get("state_history", [])
			if history.size() > 0:
				lines.append("**State History:**")
				lines.append("")
				lines.append("| State | Start (ms) | Duration |")
				lines.append("|-------|------------|----------|")

				var base_tick = resource["start_tick"]
				for entry in history:
					var state_name = ResourceTrackerDebugger.get_resource_state_string(
						entry["state"]
					)
					var rel_start = entry["start_tick"] - base_tick
					var duration_str = ""
					if entry["end_tick"] != -1:
						var duration_ms = entry["end_tick"] - entry["start_tick"]
						duration_str = str(duration_ms) + "ms"
					else:
						duration_str = "(ongoing)"
					lines.append(
						"| " + state_name + " | " + str(rel_start) + " | " + duration_str + " |"
					)

				lines.append("")

	return "\n".join(lines)
