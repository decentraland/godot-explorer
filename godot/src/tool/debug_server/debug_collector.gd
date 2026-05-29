class_name DebugCollector
extends RefCounted

## Collects scene / entity debug data for the developer-only debug WS endpoint.
## All work happens on the Godot main thread; the SceneManager Rust funcs may
## briefly lock the CRDT mutex.

const ROOT_ENTITY_ID: int = 0

## Candidate root paths for the Explorer's own UI, tried in order by
## `collect_app_ui` when no explicit `path` is given. The Explorer relocates
## its UI root between states (e.g. lobby vs. explorer-loaded), so we probe a
## small list rather than baking in a single absolute path.
const APP_UI_CANDIDATE_ROOTS: Array[String] = [
	"/root/explorer/UI",  # SDK scene loaded — UI is parented under the explorer scene
	"/root/Menu",  # Lobby / main menu state — UI sits at root directly
]
## Subtree under the explorer-state app UI root that hosts per-scene SDK UI —
## already reachable via `ui_scene` / `ui_entity`. Skipped by default in
## `collect_app_ui` so the app-UI command doesn't accidentally return
## scene-authored UI. Only applies when we're rooted at `/root/explorer/UI`.
const APP_UI_SDK_SUBTREE := "SceneUIContainer/scenes_ui"


static func collect_scenes_summary() -> Array:
	var out: Array = []
	if not is_instance_valid(Global.scene_runner):
		return out
	for child in Global.scene_runner.get_children():
		if child is DclSceneNode:
			var scene_id: int = child.get_scene_id()
			var entity_count: int = Global.scene_runner.debug_list_entities(scene_id).size()
			(
				out
				. append(
					{
						"scene_id": scene_id,
						"is_global": child.is_global(),
						"urn": str(Global.scene_runner.get_scene_entity_id(scene_id)),
						"title": str(Global.scene_runner.get_scene_title(scene_id)),
						"base_parcel":
						_vec2i_to_array(Global.scene_runner.get_scene_base_parcel(scene_id)),
						"entity_count": entity_count,
						"paused": Global.scene_runner.get_scene_is_paused(scene_id),
					}
				)
			)
	return out


static func collect_scene(scene_id: int, filters: Dictionary) -> Dictionary:
	if not _scene_loaded(scene_id):
		return {"error": "scene %d not loaded" % scene_id}

	# `include_parents` / `include_children` / `collect_nodes` are consumed
	# downstream by `_ensure_entity_in_map`. `limit` / `offset` cap the
	# matched set we expand into full objects (0 / missing = unlimited).
	var limit: int = int(filters.get("limit", 0))
	var offset: int = int(filters.get("offset", 0))

	var matched: Array = _find_matching_entities(scene_id, filters)
	var matched_total: int = matched.size()
	if offset > 0:
		matched = matched.slice(offset)
	if limit > 0 and matched.size() > limit:
		matched = matched.slice(0, limit)

	# Build entity objects for matches + expand parents/children. Use a Dictionary
	# keyed by entity_id so we don't emit duplicates.
	var by_id: Dictionary = {}
	for entity_id in matched:
		_ensure_entity_in_map(scene_id, entity_id, by_id, filters, true)

	var entities: Array = by_id.values()
	# Stable ordering by entity_id makes diffing snapshots easier.
	entities.sort_custom(func(a, b): return int(a["entity_id"]) < int(b["entity_id"]))

	return {
		"scene_id": scene_id,
		"matched_count": matched_total,
		"returned_count": matched.size(),
		"offset": offset,
		"limit": limit,
		"entity_count": entities.size(),
		"entities": entities,
	}


static func collect_entity(scene_id: int, entity_id: int, filters: Dictionary) -> Dictionary:
	if not _scene_loaded(scene_id):
		return {"error": "scene %d not loaded" % scene_id}

	var want_parents: bool = filters.get("include_parents", true)
	var want_children: bool = filters.get("include_children", true)

	var by_id: Dictionary = {}
	_ensure_entity_in_map(scene_id, entity_id, by_id, filters, true)
	if not by_id.has(entity_id):
		return {"error": "entity %d not present in scene %d" % [entity_id, scene_id]}

	var entry: Dictionary = by_id[entity_id]
	# Inline parents/children as nested arrays for the single-entity response.
	if want_parents:
		var parents: Array = []
		var pid = Global.scene_runner.debug_get_entity_parent(scene_id, entity_id)
		var guard: int = 0
		while pid != -1 and pid != ROOT_ENTITY_ID and guard < 128:
			parents.append(_build_entity_object(scene_id, int(pid), false, filters))
			pid = Global.scene_runner.debug_get_entity_parent(scene_id, int(pid))
			guard += 1
		entry["parents"] = parents
	if want_children:
		entry["children"] = _collect_direct_children(scene_id, entity_id, filters)
	return entry


# ---- app UI tree ----


## Dump the Explorer's own UI subtree. When `filters.path` is omitted, probes
## `APP_UI_CANDIDATE_ROOTS` in order and uses whichever exists in the current
## tree state (lobby vs. explorer-loaded). The per-scene SDK UI subtree is
## pruned unless `filters.include_scene_ui = true` — keeps the response
## cleanly distinct from `ui_scene`.
##
## Filters:
##   path            (str)  override the root node path; skips auto-detect
##   depth           (int)  walk depth, default 3
##   collect_nodes   (dict) per-node-name extra property dump (same as elsewhere)
##   class_filter    (str)  only emit nodes whose `get_class()` equals this
##   name_contains   (str)  only emit nodes whose `name.to_lower()` contains this
##   include_scene_ui (bool) lift the default `SceneUIContainer/scenes_ui` skip
static func collect_app_ui(filters: Dictionary) -> Dictionary:
	var depth: int = int(filters.get("depth", 3))
	var collect_nodes: Dictionary = filters.get("collect_nodes", {})
	var class_filter: String = str(filters.get("class_filter", ""))
	var name_contains: String = str(filters.get("name_contains", "")).to_lower()
	var include_scene_ui: bool = bool(filters.get("include_scene_ui", false))

	var tree := Engine.get_main_loop() as SceneTree
	if tree == null or not is_instance_valid(tree.root):
		return {"error": "scene tree unavailable"}

	# Resolve the root: explicit `path` wins, otherwise probe candidates in
	# order. We record which candidates were tried so the error response is
	# debuggable if every probe misses.
	var root: Node = null
	var resolved_from := ""
	if filters.has("path"):
		var explicit: String = str(filters["path"])
		root = tree.root.get_node_or_null(explicit)
		if root == null:
			return {"error": "node not found at path: %s" % explicit}
		resolved_from = "explicit"
	else:
		for candidate in APP_UI_CANDIDATE_ROOTS:
			var node := tree.root.get_node_or_null(candidate)
			if node != null:
				root = node
				resolved_from = candidate
				break
		if root == null:
			return {"error": "no app UI root found; tried %s" % str(APP_UI_CANDIDATE_ROOTS)}

	# The SDK-UI skip only makes sense relative to the explorer-loaded root;
	# the lobby root (/root/Menu) has no scene UI under it. We anchor on the
	# resolved root rather than the constant so explicit `path` overrides
	# still get the skip applied when they target the right subtree.
	var sdk_skip_path := ""
	if not include_scene_ui:
		sdk_skip_path = "%s/%s" % [str(root.get_path()), APP_UI_SDK_SUBTREE]

	return {
		"tree": "app_ui",
		"root_path": str(root.get_path()),
		"resolved_from": resolved_from,
		"node":
		_walk_app_ui(root, depth, collect_nodes, class_filter, name_contains, sdk_skip_path),
	}


static func _walk_app_ui(
	node: Node,
	depth_remaining: int,
	collect_nodes: Dictionary,
	class_filter: String,
	name_contains: String,
	sdk_skip_path: String
) -> Dictionary:
	var data: Dictionary = {
		"name": String(node.name),
		"class": node.get_class(),
		"node_path": str(node.get_path()),
	}
	if node is CanvasItem:
		data["visible"] = node.visible
	if node is Control:
		var ctrl: Control = node
		data["position"] = [ctrl.position.x, ctrl.position.y]
		data["size"] = [ctrl.size.x, ctrl.size.y]
		data["z_index"] = ctrl.z_index

	# `collect_nodes` here is keyed by node name (same convention as the other
	# cmds), so the client can ask for extra properties on specific nodes.
	if collect_nodes.has(String(node.name)):
		var requested: Array = collect_nodes[String(node.name)]
		var dump: Dictionary = {}
		for raw_prop in requested:
			var prop: String = String(raw_prop)
			dump[prop] = _variant_to_json(node.get(prop))
		data["properties"] = dump

	if depth_remaining > 0:
		var children: Array = []
		for child in node.get_children():
			# Skip the per-scene SDK UI subtree unless explicitly included.
			if not sdk_skip_path.is_empty() and str(child.get_path()) == sdk_skip_path:
				continue
			var child_data := _walk_app_ui(
				child,
				depth_remaining - 1,
				collect_nodes,
				class_filter,
				name_contains,
				sdk_skip_path
			)
			# Skip nodes that don't match filters (they're effectively pruned
			# from the tree). Children are still recursively walked above so
			# filter matches deeper down survive.
			var keep := true
			if not class_filter.is_empty() and child.get_class() != class_filter:
				# Hide this node UNLESS it has matching descendants.
				keep = child_data.get("children", []).size() > 0
			if keep and not name_contains.is_empty():
				if String(child.name).to_lower().find(name_contains) == -1:
					keep = child_data.get("children", []).size() > 0
			if keep:
				children.append(child_data)
		if not children.is_empty():
			data["children"] = children
	return data


# ---- avatar tree ----


## List every tracked avatar with identity + transform. `Global.avatars` is the
## global `AvatarScene` that lives on tree root (not under any DCL scene), so
## this command takes no `scene_id` argument.
static func collect_avatars() -> Array:
	if not is_instance_valid(Global.avatars):
		return []
	var raw: Array = Global.avatars.debug_list_avatars()
	var out: Array = []
	for entry in raw:
		var normalized: Dictionary = {}
		for k in entry:
			normalized[String(k)] = _variant_to_json(entry[k])
		out.append(normalized)
	out.sort_custom(func(a, b): return int(a.get("entity_id", 0)) < int(b.get("entity_id", 0)))
	return out


## Inspect one avatar identified by `by` ∈ {"address", "alias", "entity"}.
## Returns the identity block (matching the `collect_avatars` row) plus a
## `godot` block with the rendered `DclAvatar` transform / properties. Also
## supports the generic `collect_nodes` filter for child-node inspection.
static func collect_avatar(by: String, value: Variant, filters: Dictionary) -> Dictionary:
	if not is_instance_valid(Global.avatars):
		return {"error": "AvatarScene not available"}
	var iid: int = -1
	match by:
		"address":
			iid = int(Global.avatars.debug_get_avatar_instance_id_by_address(str(value)))
		"alias":
			iid = int(Global.avatars.debug_get_avatar_instance_id_by_alias(int(value)))
		"entity":
			iid = int(Global.avatars.debug_get_avatar_instance_id_by_entity(int(value)))
		"local":
			# Local player has no comms identity; `value` is ignored.
			iid = int(Global.avatars.debug_get_local_player_instance_id())
		_:
			return {"error": "unknown 'by' (expected address|alias|entity|local): %s" % by}
	if iid < 0:
		return {"error": "avatar not found"}
	var node = instance_from_id(iid)
	if node == null:
		return {"error": "avatar instance no longer valid"}

	# Locate the matching identity row from `debug_list_avatars`. We pay one
	# list traversal here; the avatar count is tiny relative to scene entities.
	var identity: Dictionary = {}
	for d in collect_avatars():
		if int(d.get("instance_id", 0)) == iid:
			identity = d
			break

	return {
		"identity": identity,
		"godot": _collect_avatar_godot_side(node, filters.get("collect_nodes", {})),
	}


static func _collect_avatar_godot_side(node: Node, collect_nodes: Dictionary) -> Dictionary:
	var data: Dictionary = {
		"present": true,
		"tree": "avatar",
		"node_path": str(node.get_path()),
		"node_class": node.get_class(),
		"name": node.name,
	}
	if node is Node3D:
		var n3d: Node3D = node
		data["visible"] = n3d.visible
		data["local_transform"] = _transform_to_dict(n3d.transform)
		data["global_transform"] = _transform_to_dict(n3d.global_transform)

	# Reuse the same `collect_nodes` mechanism the 3D / UI cmds use. Avatars
	# have a deep child hierarchy (mesh, AnimationPlayer, AvatarRenderer, …)
	# so this is the most useful knob.
	for raw_child_name in collect_nodes:
		var child_name: String = String(raw_child_name)
		var child := node.get_node_or_null(child_name)
		if child == null:
			continue
		var requested: Array = collect_nodes[raw_child_name]
		var dump: Dictionary = {"node_class": child.get_class()}
		for raw_prop in requested:
			var prop: String = String(raw_prop)
			dump[prop] = _variant_to_json(child.get(prop))
		data[child_name] = dump
	return data


# ---- internals ----


static func _scene_loaded(scene_id: int) -> bool:
	# Guards every `collect_scene` / `collect_entity` request — both bail out
	# with `{"error": "scene N not loaded"}` if this returns false. Keeping the
	# `is_instance_valid` check here means downstream helpers (which call
	# `Global.scene_runner.*` unguarded) never run with a freed SceneManager.
	if not is_instance_valid(Global.scene_runner):
		return false
	var loaded: PackedInt32Array = Global.scene_runner.debug_get_loaded_scene_ids()
	for sid in loaded:
		if int(sid) == scene_id:
			return true
	return false


static func _find_matching_entities(scene_id: int, filters: Dictionary) -> Array:
	var entity_filter: Array = filters.get("entity", [])
	var component_filter: Array = filters.get("component", [])

	# `property_is` is the generic per-field filter that replaced `text_contains`.
	# Schema: {"component": "<SDK component name>", "field": "<top-level proto field>",
	#          "contains": "<case-insensitive substring>"}.
	# Adding `equals` / `prefix` operators later is a one-line check below.
	var prop_is: Dictionary = filters.get("property_is", {})
	var prop_component: String = str(prop_is.get("component", ""))
	var prop_field: String = str(prop_is.get("field", ""))
	var prop_contains: String = str(prop_is.get("contains", "")).to_lower()
	var has_property_filter := (
		not prop_component.is_empty() and not prop_field.is_empty() and not prop_contains.is_empty()
	)

	var all_entities: PackedInt32Array = Global.scene_runner.debug_list_entities(scene_id)

	var result: Array = []
	for raw_id in all_entities:
		var eid: int = int(raw_id)

		if not entity_filter.is_empty() and not entity_filter.has(eid):
			continue

		# No component / property filter → keep the entity without touching payloads.
		if component_filter.is_empty() and not has_property_filter:
			result.append(eid)
			continue

		# Component-name filter uses the cheap names listing (no proto decode).
		if not component_filter.is_empty():
			var names: PackedStringArray = Global.scene_runner.debug_get_entity_component_names(
				scene_id, eid
			)
			var ok := false
			for needle in component_filter:
				if names.has(needle):
					ok = true
					break
			if not ok:
				continue

		# `property_is` requires the SDK payload — only deserialize for entities
		# that survived the cheaper component-name filter above.
		if has_property_filter:
			var comps := _fetch_components(scene_id, eid)
			var lww: Dictionary = comps.get("lww", {})
			var comp: Dictionary = lww.get(prop_component, {})
			var val: String = str(comp.get(prop_field, "")).to_lower()
			if val.find(prop_contains) == -1:
				continue

		result.append(eid)
	return result


static func _ensure_entity_in_map(
	scene_id: int, entity_id: int, by_id: Dictionary, filters: Dictionary, is_match: bool
) -> void:
	if by_id.has(entity_id):
		# Preserve is_match=true if this entity was reached as a primary match.
		if is_match:
			(by_id[entity_id] as Dictionary)["is_match"] = true
		return

	var entry := _build_entity_object(scene_id, entity_id, is_match, filters)
	by_id[entity_id] = entry

	# Parents / children are walked relative to the top-level call only — we
	# pass an empty filters dict downstream so we don't compound traversal.
	# Default off: expansion bypasses `limit` and can balloon a `scene` reply
	# past the WS outbound buffer. Callers that want the tree opt in explicitly.
	var want_parents: bool = filters.get("include_parents", false)
	var want_children: bool = filters.get("include_children", false)
	var inherited_filters: Dictionary = {
		"include_parents": false,
		"include_children": false,
		"collect_nodes": filters.get("collect_nodes", {}),
		"tree": filters.get("tree", "3d"),
	}

	if want_parents:
		var pid = Global.scene_runner.debug_get_entity_parent(scene_id, entity_id)
		var guard: int = 0
		while pid != -1 and pid != ROOT_ENTITY_ID and guard < 128:
			_ensure_entity_in_map(scene_id, int(pid), by_id, inherited_filters, false)
			pid = Global.scene_runner.debug_get_entity_parent(scene_id, int(pid))
			guard += 1

	if want_children:
		for child_id in _direct_child_ids(scene_id, entity_id):
			_ensure_entity_in_map(scene_id, child_id, by_id, inherited_filters, false)


static func _build_entity_object(
	scene_id: int, entity_id: int, is_match: bool, filters: Dictionary
) -> Dictionary:
	var comps := _fetch_components(scene_id, entity_id)
	var collect_nodes: Dictionary = filters.get("collect_nodes", {})
	var tree: String = str(filters.get("tree", "3d"))
	return {
		"scene_id": scene_id,
		"entity_id": entity_id,
		"is_match": is_match,
		"godot": _collect_godot_side(scene_id, entity_id, collect_nodes, tree),
		"sdk": {"components": comps.get("lww", {})},
		"sdk_gos": comps.get("gos", {}),
	}


static func _fetch_components(scene_id: int, entity_id: int) -> Dictionary:
	var raw: String = str(Global.scene_runner.debug_get_entity_components_json(scene_id, entity_id))
	if raw.is_empty():
		return {"lww": {}, "gos": {}}
	var parsed = JSON.parse_string(raw)
	if parsed is Dictionary:
		return parsed
	return {"lww": {}, "gos": {}}


static func _collect_godot_side(
	scene_id: int, entity_id: int, collect_nodes: Dictionary, tree: String
) -> Dictionary:
	var node := _find_entity_node(scene_id, entity_id, tree)
	if node == null:
		return {"present": false}

	var data: Dictionary = {
		"present": true,
		"tree": tree,
		"node_path": str(node.get_path()),
		"node_class": node.get_class(),
		"name": node.name,
	}
	if node is Node3D:
		var n3d: Node3D = node
		data["visible"] = n3d.visible
		data["local_transform"] = _transform_to_dict(n3d.transform)
		data["global_transform"] = _transform_to_dict(n3d.global_transform)
		var aabb_variant: Variant = _compute_world_aabb(n3d)
		if aabb_variant != null:
			var aabb: AABB = aabb_variant
			data["aabb"] = {
				"position": _vec3_to_array(aabb.position),
				"size": _vec3_to_array(aabb.size),
			}
	elif node is Control:
		var ctrl: Control = node
		data["visible"] = ctrl.visible
		data["position"] = [ctrl.position.x, ctrl.position.y]
		data["size"] = [ctrl.size.x, ctrl.size.y]
		data["global_position"] = [ctrl.global_position.x, ctrl.global_position.y]
		data["modulate"] = _variant_to_json(ctrl.modulate)
		data["self_modulate"] = _variant_to_json(ctrl.self_modulate)
		data["z_index"] = ctrl.z_index
		data["mouse_filter"] = int(ctrl.mouse_filter)
		data["anchors"] = {
			"left": ctrl.anchor_left,
			"top": ctrl.anchor_top,
			"right": ctrl.anchor_right,
			"bottom": ctrl.anchor_bottom,
		}

	# Generic per-child-node inspection. `collect_nodes` is a Dictionary of
	# {<child node name>: [<property name>, ...]}; the client decides which
	# children to look at and which properties to read. Use `Object.get()` so
	# any property exposed on the node is fair game — no component-type
	# special-casing here.
	for raw_child_name in collect_nodes:
		var child_name: String = String(raw_child_name)
		var child := node.get_node_or_null(child_name)
		if child == null:
			continue
		var requested: Array = collect_nodes[raw_child_name]
		var dump: Dictionary = {"node_class": child.get_class()}
		for raw_prop in requested:
			var prop: String = String(raw_prop)
			dump[prop] = _variant_to_json(child.get(prop))
		data[child_name] = dump
	return data


static func _collect_direct_children(scene_id: int, entity_id: int, filters: Dictionary) -> Array:
	var out: Array = []
	for child_id in _direct_child_ids(scene_id, entity_id):
		out.append(_build_entity_object(scene_id, child_id, false, filters))
	return out


static func _direct_child_ids(scene_id: int, parent_id: int) -> Array:
	var out: Array = []
	for raw_id in Global.scene_runner.debug_list_entities(scene_id):
		var eid: int = int(raw_id)
		if eid == parent_id:
			continue
		var pid := int(Global.scene_runner.debug_get_entity_parent(scene_id, eid))
		if pid == parent_id:
			out.append(eid)
	return out


## Resolve the renderer node attached to `entity_id` in `scene_id`, in the
## chosen `tree`:
##   "3d" — the `DclNodeEntity3d` child of the scene's `DclSceneNode`.
##   "ui" — the `Control` returned by `SceneManager.debug_get_entity_ui_control_id`.
## Returns null if the entity has no node in that tree (typical for entities
## whose CRDT components were authored but the renderer hasn't instantiated yet).
static func _find_entity_node(scene_id: int, entity_id: int, tree: String) -> Node:
	if not is_instance_valid(Global.scene_runner):
		return null
	if tree == "ui":
		var iid: int = int(Global.scene_runner.debug_get_entity_ui_control_id(scene_id, entity_id))
		if iid < 0:
			return null
		var node = instance_from_id(iid)
		return node if node is Node else null
	# "3d" fallback — walk the per-scene `DclSceneNode` children.
	for scene_child in Global.scene_runner.get_children():
		if scene_child is DclSceneNode and scene_child.get_scene_id() == scene_id:
			for entity_child in scene_child.get_children():
				if entity_child is DclNodeEntity3d and entity_child.e_id() == entity_id:
					return entity_child
			break
	return null


static func _compute_world_aabb(node: Node3D) -> Variant:
	if node is VisualInstance3D:
		return (node as VisualInstance3D).get_aabb()
	var merged: AABB = AABB()
	var has: bool = false
	for child in node.get_children():
		if child is VisualInstance3D:
			var a: AABB = (child as VisualInstance3D).get_aabb()
			if not has:
				merged = a
				has = true
			else:
				merged = merged.merge(a)
	return merged if has else null


static func _transform_to_dict(t: Transform3D) -> Dictionary:
	return {
		"position": _vec3_to_array(t.origin),
		"rotation_euler": _vec3_to_array(t.basis.get_euler()),
		"scale": _vec3_to_array(t.basis.get_scale()),
	}


static func _vec3_to_array(v: Vector3) -> Array:
	return [v.x, v.y, v.z]


static func _vec2i_to_array(v: Vector2i) -> Array:
	return [v.x, v.y]


## Convert a Variant to a JSON-safe value. Used for generic node-property
## inspection via `collect_nodes` — the client can request *any* property and
## we'll do our best to serialize it. Falls back to `str(v)` for types we
## don't have a structured representation for (e.g. Object refs, NodePaths).
static func _variant_to_json(v: Variant) -> Variant:
	if v == null:
		return null
	var t: int = typeof(v)
	if t == TYPE_BOOL or t == TYPE_INT or t == TYPE_FLOAT or t == TYPE_STRING:
		return v
	if t == TYPE_STRING_NAME:
		return String(v)
	if t == TYPE_VECTOR2 or t == TYPE_VECTOR2I:
		return [v.x, v.y]
	if t == TYPE_VECTOR3 or t == TYPE_VECTOR3I:
		return [v.x, v.y, v.z]
	if t == TYPE_VECTOR4 or t == TYPE_VECTOR4I or t == TYPE_QUATERNION:
		return [v.x, v.y, v.z, v.w]
	if t == TYPE_COLOR:
		return [v.r, v.g, v.b, v.a]
	if t == TYPE_AABB:
		return {
			"position": [v.position.x, v.position.y, v.position.z],
			"size": [v.size.x, v.size.y, v.size.z],
		}
	if t == TYPE_RECT2 or t == TYPE_RECT2I:
		return {"position": [v.position.x, v.position.y], "size": [v.size.x, v.size.y]}
	if t == TYPE_TRANSFORM3D:
		return _transform_to_dict(v)
	if t == TYPE_ARRAY or t == TYPE_PACKED_INT32_ARRAY or t == TYPE_PACKED_INT64_ARRAY:
		var out_arr: Array = []
		for item in v:
			out_arr.append(_variant_to_json(item))
		return out_arr
	if t == TYPE_DICTIONARY:
		var out_dict: Dictionary = {}
		for k in v:
			out_dict[String(k)] = _variant_to_json(v[k])
		return out_dict
	# Fallback — covers Object refs, NodePaths, Callable, RIDs, etc.
	return str(v)
