@tool
extends HSplitContainer

## Entity Tree panel for Decentraland scene debugging.
## Shows entities in a parent-child hierarchy with real-time CRDT updates.
## Selecting an entity shows its current components and a live change feed.

const MAX_CHANGES_PER_ENTITY := 200
const MAX_CHANGES_LIST := 500
const GOS_LIMIT := 100

# ── Entity state ──────────────────────────────────────────────────────
# eid (int) -> { "components": {name: value}, "parent": int, "tree_item": TreeItem or null }
var entities: Dictionary = {}
# eid (int) -> Array[Dictionary] of recent CRDT changes (capped)
var entity_changes: Dictionary = {}

var selected_entity: int = -1
var paused: bool = false
var pending_entries: Array = []
var tree_dirty: bool = false
var filter_text: String = ""
var total_crdt_count: int = 0

# ── Node references ──────────────────────────────────────────────────
@onready var entity_tree: Tree = %EntityTree
@onready var components_tree: Tree = %ComponentsTree
@onready var changes_list: ItemList = %ChangesList
@onready var entity_info_label: Label = %EntityInfoLabel
@onready var label_count: Label = %LabelCount
@onready var filter_edit: LineEdit = %FilterEdit
@onready var check_pause: CheckButton = %CheckPause


func _ready():
	entity_tree.set_column_title(0, "Entity")
	entity_tree.set_column_title(1, "Components")
	entity_tree.set_column_expand(0, false)
	entity_tree.set_column_custom_minimum_width(0, 100)
	entity_tree.set_column_expand(1, true)
	entity_tree.create_item()  # hidden root

	components_tree.set_column_title(0, "Component")
	components_tree.set_column_title(1, "Value")
	components_tree.set_column_expand(0, false)
	components_tree.set_column_custom_minimum_width(0, 160)
	components_tree.set_column_expand(1, true)
	components_tree.create_item()  # hidden root


# ── Public API (called from plugin _capture) ─────────────────────────


func add_entries(entries_array: Array) -> void:
	if paused:
		pending_entries.append_array(entries_array)
		return
	_process_entries(entries_array)


func clear_entries() -> void:
	entities.clear()
	entity_changes.clear()
	selected_entity = -1
	total_crdt_count = 0
	tree_dirty = false
	pending_entries.clear()
	_comp_tree_items.clear()
	_rendered_changes_count = 0
	_rebuild_tree()
	_rebuild_detail_panel()
	_update_count_label()


# ── Entry processing ─────────────────────────────────────────────────


func _process_entries(entries_array: Array) -> void:
	var changed_eids: Dictionary = {}  # set of entity IDs that changed

	for entry in entries_array:
		if not entry is Dictionary:
			continue
		var entry_type: String = entry.get("type", "")
		if entry_type == "crdt":
			total_crdt_count += 1
			var eid: int = entry.get("e", -1)
			if eid < 0:
				continue
			_apply_crdt_entry(eid, entry)
			changed_eids[eid] = true

			# Track change for this entity
			if not entity_changes.has(eid):
				entity_changes[eid] = []
			var changes: Array = entity_changes[eid]
			changes.push_back(entry)
			if changes.size() > MAX_CHANGES_PER_ENTITY:
				changes.pop_front()

	# Update tree
	if tree_dirty:
		_rebuild_tree()
		tree_dirty = false
	else:
		for eid in changed_eids:
			if entities.has(eid):
				_update_tree_item(eid)

	# Update detail panel if selected entity was affected
	if selected_entity >= 0 and changed_eids.has(selected_entity):
		_update_detail_panel()

	_update_count_label()


func _apply_crdt_entry(eid: int, entry: Dictionary) -> void:
	var op: String = entry.get("op", "")
	var comp: String = entry.get("c", "")

	# delete_entity
	if op == "de":
		if entities.has(eid):
			entities.erase(eid)
			entity_changes.erase(eid)
			tree_dirty = true
		return

	# Ensure entity exists
	if not entities.has(eid):
		entities[eid] = {"components": {}, "parent": 0, "tree_item": null}
		tree_dirty = true

	var ent: Dictionary = entities[eid]

	if op == "d":
		# delete component
		if ent.components.has(comp):
			ent.components.erase(comp)
	elif op == "a":
		# append (GOS)
		if not ent.components.has(comp) or not (ent.components[comp] is Array):
			ent.components[comp] = []
		var arr: Array = ent.components[comp]
		arr.push_back(entry.get("payload", {}))
		if arr.size() > GOS_LIMIT:
			arr.pop_front()
	else:
		# put (LWW)
		ent.components[comp] = entry.get("payload", true)

		# Extract parent from Transform
		if comp == "Transform":
			var payload = entry.get("payload")
			if payload is Dictionary and payload.has("parent"):
				var new_parent: int = payload.get("parent", 0)
				if ent.parent != new_parent:
					ent.parent = new_parent
					tree_dirty = true

		# Extract parent from UiTransform
		if comp == "UiTransform":
			var payload = entry.get("payload")
			if payload is Dictionary:
				var new_parent: int = 0
				if payload.has("parent"):
					new_parent = payload.get("parent", 0)
				elif payload.has("parent_entity"):
					new_parent = payload.get("parent_entity", 0)
				if ent.parent != new_parent:
					ent.parent = new_parent
					tree_dirty = true


# ── Entity Tree ──────────────────────────────────────────────────────


func _rebuild_tree() -> void:
	# Save selection
	var prev_selected := selected_entity

	entity_tree.clear()
	var root := entity_tree.create_item()

	# Null out all tree_item refs
	for eid in entities:
		entities[eid].tree_item = null

	# Build children map: parent_eid -> [child_eids]
	var children_map: Dictionary = {}
	var all_eids: Array = entities.keys()
	all_eids.sort()

	for eid in all_eids:
		var parent_eid: int = entities[eid].parent
		# Skip self-referencing parents (entity 0 has parent=0)
		if parent_eid == eid:
			continue
		if not children_map.has(parent_eid):
			children_map[parent_eid] = []
		children_map[parent_eid].append(eid)

	# Find roots: self-referencing parent, or parent entity doesn't exist in our state.
	# Note: entities with parent=0 are children of entity 0 if entity 0 exists
	# (they are already in children_map[0]), NOT roots.
	var roots: Array = []
	for eid in all_eids:
		var parent_eid: int = entities[eid].parent
		if parent_eid == eid:
			# Self-referencing (e.g., entity 0 with parent=0)
			roots.append(eid)
		elif not entities.has(parent_eid):
			# Parent doesn't exist in our state — treat as root
			roots.append(eid)

	# Render recursively
	for eid in roots:
		_add_tree_node(eid, root, children_map)

	# Restore selection
	if prev_selected >= 0 and entities.has(prev_selected):
		var item: TreeItem = entities[prev_selected].tree_item
		if item:
			item.select(0)
			selected_entity = prev_selected


func _add_tree_node(eid: int, parent_item: TreeItem, children_map: Dictionary) -> void:
	if not entities.has(eid):
		return

	# Apply filter
	if not filter_text.is_empty():
		if not _entity_matches_filter(eid):
			return

	var ent: Dictionary = entities[eid]
	var item := entity_tree.create_item(parent_item)

	# Column 0: Entity ID
	item.set_text(0, str(eid))

	# Column 1: Component names
	var comp_names: Array = ent.components.keys()
	comp_names.sort()
	item.set_text(1, ", ".join(comp_names))

	item.set_meta("entity_id", eid)
	ent.tree_item = item

	# Recurse children
	if children_map.has(eid):
		for child_eid in children_map[eid]:
			_add_tree_node(child_eid, item, children_map)


func _update_tree_item(eid: int) -> void:
	if not entities.has(eid):
		return
	var ent: Dictionary = entities[eid]
	var item: TreeItem = ent.get("tree_item")
	if item == null:
		return
	# Update component list text
	var comp_names: Array = ent.components.keys()
	comp_names.sort()
	item.set_text(1, ", ".join(comp_names))


func _entity_matches_filter(eid: int) -> bool:
	if str(eid).find(filter_text) >= 0:
		return true
	var ent: Dictionary = entities[eid]
	for comp_name in ent.components:
		if comp_name.to_lower().find(filter_text.to_lower()) >= 0:
			return true
	return false


# ── Detail Panel ─────────────────────────────────────────────────────

# Cached TreeItem refs for components: comp_name -> TreeItem
var _comp_tree_items: Dictionary = {}
# How many live changes we've already rendered for this entity
var _rendered_changes_count: int = 0


func _update_detail_panel() -> void:
	if selected_entity < 0 or not entities.has(selected_entity):
		entity_info_label.text = "Select an entity"
		return

	var ent: Dictionary = entities[selected_entity]
	var comp_count: int = ent.components.size()
	entity_info_label.text = (
		"Entity %d  |  parent=%d  |  %d components" % [selected_entity, ent.parent, comp_count]
	)

	_update_components_tree(ent)
	_append_new_changes()


## Rebuild detail panel from scratch (on entity selection change)
func _rebuild_detail_panel() -> void:
	components_tree.clear()
	components_tree.create_item()  # hidden root
	changes_list.clear()
	_comp_tree_items.clear()
	_rendered_changes_count = 0

	if selected_entity < 0 or not entities.has(selected_entity):
		entity_info_label.text = "Select an entity"
		return

	_update_detail_panel()

	# Render all existing changes
	if entity_changes.has(selected_entity):
		var changes: Array = entity_changes[selected_entity]
		var start := maxi(0, changes.size() - MAX_CHANGES_LIST)
		for i in range(start, changes.size()):
			_add_change_item(changes[i])
		_rendered_changes_count = changes.size()


func _update_components_tree(ent: Dictionary) -> void:
	var comp_root := components_tree.get_root()
	var comp_names: Array = ent.components.keys()
	comp_names.sort()

	# Remove components that no longer exist
	for old_name in _comp_tree_items.keys():
		if not ent.components.has(old_name):
			var old_item: TreeItem = _comp_tree_items[old_name]
			if old_item:
				old_item.free()
			_comp_tree_items.erase(old_name)

	# Add or update each component
	for comp_name in comp_names:
		var value = ent.components[comp_name]
		var item: TreeItem = _comp_tree_items.get(comp_name)

		if item == null:
			item = components_tree.create_item(comp_root)
			item.set_text(0, comp_name)
			_comp_tree_items[comp_name] = item

		# Clear existing children to rebuild value sub-tree
		var child := item.get_first_child()
		while child:
			var next := child.get_next()
			child.free()
			child = next

		if value is Array:
			item.set_text(1, "[GOS: %d items]" % value.size())
			for i in range(value.size()):
				var sub := components_tree.create_item(item)
				sub.set_text(0, "[%d]" % i)
				sub.set_text(1, _truncate_json(value[i], 200))
		elif value is Dictionary:
			item.set_text(1, _truncate_json(value, 200))
			var keys: Array = value.keys()
			keys.sort()
			for key in keys:
				var sub := components_tree.create_item(item)
				sub.set_text(0, str(key))
				sub.set_text(1, _truncate_json(value[key], 150))
		else:
			item.set_text(1, str(value))


func _append_new_changes() -> void:
	if not entity_changes.has(selected_entity):
		return
	var changes: Array = entity_changes[selected_entity]
	# Only render entries we haven't rendered yet
	while _rendered_changes_count < changes.size():
		_add_change_item(changes[_rendered_changes_count])
		_rendered_changes_count += 1
	# Cap the list UI
	while changes_list.item_count > MAX_CHANGES_LIST:
		changes_list.remove_item(0)


func _add_change_item(change: Dictionary) -> void:
	var tick = change.get("tk", "?")
	var op = change.get("op", "?")
	var comp = change.get("c", "?")
	var direction = change.get("d", "?")
	changes_list.add_item("[tk:%s] %s %s %s" % [tick, _op_label(op), comp, direction])
	# Auto-scroll to bottom
	if changes_list.item_count > 0:
		changes_list.ensure_current_is_visible()


func _truncate_json(value, max_len: int) -> String:
	var s := str(value)
	if s.length() > max_len:
		return s.left(max_len) + "..."
	return s


func _op_label(op: String) -> String:
	match op:
		"p":
			return "PUT"
		"d":
			return "DEL"
		"de":
			return "DEL_ENT"
		"a":
			return "APPEND"
		_:
			return op.to_upper()


# ── Signals ──────────────────────────────────────────────────────────


func _on_tree_item_selected() -> void:
	var item := entity_tree.get_selected()
	if item and item.has_meta("entity_id"):
		selected_entity = item.get_meta("entity_id")
		_rebuild_detail_panel()


func _on_filter_changed(new_text: String) -> void:
	filter_text = new_text.strip_edges()
	_rebuild_tree()


func _on_pause_toggled(pressed: bool) -> void:
	paused = pressed
	if not paused and not pending_entries.is_empty():
		var entries := pending_entries.duplicate()
		pending_entries.clear()
		_process_entries(entries)


func _on_button_clear_pressed() -> void:
	clear_entries()


func _update_count_label() -> void:
	if label_count:
		label_count.text = "%d entities | %d msgs" % [entities.size(), total_crdt_count]
