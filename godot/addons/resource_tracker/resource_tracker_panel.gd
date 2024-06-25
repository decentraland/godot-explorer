@tool
extends Control

var resource_statuses = {}

@onready var tree = %Tree
@onready var option_box_filter = %OptionBox_Filter
@onready var label_speed = %Label_Speed
@onready var label_info = %LabelInfo


func _ready():
	# Initialize the Tree node
	tree.create_item()  # Create a root item
	tree.columns = 6
	tree.set_column_title(0, "Hash ID")
	tree.set_column_title(1, "State")
	tree.set_column_title(2, "Progress")
	tree.set_column_title(3, "Size")
	tree.set_column_title(4, "Metadata")
	tree.set_column_title(5, "Time")

	# Cache for tree items
	resource_statuses = {}

	# Update the resource status regularly
	set_process(true)


func _process(_delta):
	update_resource_status()


func report_speed(speed: String):
	label_info.hide()
	label_speed.text = speed


func report_resource(
	hash_id: String,
	state: ResourceTrackerDebugger.ResourceTrackerState,
	progress: String,
	size: String,
	metadata: String
):
	# Check if the resource already exists in the cache
	if resource_statuses.has(hash_id):
		# Update existing resource
		var resource = resource_statuses[hash_id]
		if (
			resource["state"] < state
			or state == ResourceTrackerDebugger.ResourceTrackerState.DELETED
		):  # only update the state when is greater
			resource["state"] = state

		if not progress.is_empty():
			resource["progress"] = progress

		if not size.is_empty():
			resource["size"] = size

		if not metadata.is_empty():
			resource["metadata"] = metadata

		var elapsed: float = float(Time.get_ticks_msec() - resource["start_tick"]) / 1000.0
		resource["elapsed"] = snappedf(elapsed, 0.01)

		resource_statuses[hash_id] = resource
	else:
		# Add new resource
		resource_statuses[hash_id] = {
			"hash_id": hash_id,
			"state": state,
			"progress": progress,
			"size": size,
			"metadata": metadata,
			"start_tick": Time.get_ticks_msec(),
			"elapsed": 0.0,
			"item": null  # Placeholder for the TreeItem
		}
	# Update the UI after modifying the resource status
	_update_ui()


func update_resource_status():
	# This can be used to periodically update the status if needed
	pass


func clear_cache():
	resource_statuses.clear()
	clear()


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

		item.set_text(0, hash_id)
		item.set_text(1, ResourceTrackerDebugger.get_resource_state_string(resource["state"]))
		item.set_text(2, resource["progress"])
		item.set_text(3, resource["size"])
		item.set_text(4, resource["metadata"])
		item.set_text(5, str(resource["elapsed"]) + "segs")

		resource_statuses[hash_id] = resource


func _on_option_box_filter_item_selected(_index):
	clear()
	for hash_id in resource_statuses.keys():
		resource_statuses[hash_id]["item"] = null

	_update_ui()


func _on_tree_item_mouse_selected(_position, mouse_button_index):
	if mouse_button_index != MOUSE_BUTTON_RIGHT:
		return

	var selected: TreeItem = tree.get_selected()
	if selected:
		var text = selected.get_text(0)
		DisplayServer.clipboard_set(text)
