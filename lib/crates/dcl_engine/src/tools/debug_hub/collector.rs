//! Collects scene, entity, UI and avatar data for the debug hub. Everything runs on the main
//! thread; the scene runner's debug functions may briefly lock the CRDT mutex.

use std::collections::BTreeMap;

use godot::classes::{CanvasItem, Control, Node, Node3D, VisualInstance3D};
use godot::prelude::*;
use serde_json::{json, Map, Value};

use super::json::{aabb, property, transform, variant_to_json, vec2};
use crate::fetcher::globals;
use crate::godot_classes::dcl_node_entity_3d::DclNodeEntity3d;
use crate::godot_classes::dcl_scene_node::DclSceneNode;

const ROOT_ENTITY_ID: i64 = 0;
/// Bound on a parent chain walk: a cycle in a broken scene must not hang the client.
const PARENT_GUARD: u32 = 128;

/// Candidate roots of the app's own UI, tried in order when no path is given: the explorer moves
/// its UI root between states.
const APP_UI_ROOTS: [&str; 2] = [
    "/root/explorer/UI", // a world is loaded: the UI sits under the explorer scene
    "/root/Menu",        // the lobby / main menu state
];
/// The subtree under the explorer UI that hosts per-scene SDK UI, reachable through `ui_scene`.
const APP_UI_SDK_SUBTREE: &str = "SceneUIContainer/scenes_ui";

fn error(message: String) -> Value {
    json!({"error": message})
}

fn int(value: Option<&Value>, fallback: i64) -> i64 {
    value.and_then(Value::as_i64).unwrap_or(fallback)
}

fn flag(value: Option<&Value>, fallback: bool) -> bool {
    value.and_then(Value::as_bool).unwrap_or(fallback)
}

fn text(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn object(value: Option<&Value>) -> Map<String, Value> {
    value
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

// ---- scenes

fn scene_nodes() -> Vec<Gd<DclSceneNode>> {
    globals::scene_runner()
        .get_children()
        .iter_shared()
        .filter_map(|child| child.try_cast::<DclSceneNode>().ok())
        .collect()
}

fn scene_loaded(scene_id: i64) -> bool {
    let loaded = globals::scene_runner().bind().debug_get_loaded_scene_ids();
    loaded
        .as_slice()
        .iter()
        .any(|id| i64::from(*id) == scene_id)
}

pub fn scenes_summary() -> Value {
    let runner = globals::scene_runner();
    let mut out = Vec::new();
    for node in scene_nodes() {
        let (scene_id, is_global) = {
            let node = node.bind();
            (node.get_scene_id(), node.is_global())
        };
        let runner = runner.bind();
        let base = runner.get_scene_base_parcel(scene_id);
        let tick = runner.get_scene_tick_stats(scene_id);
        let int = |key: &str| {
            tick.get(key)
                .and_then(|v| v.try_to::<i64>().ok())
                .unwrap_or(0)
        };
        let ms = |key: &str| {
            tick.get(key)
                .and_then(|v| v.try_to::<f64>().ok())
                .unwrap_or(0.0)
        };
        out.push(json!({
            "scene_id": scene_id,
            "is_global": is_global,
            "urn": runner.get_scene_entity_id(scene_id).to_string(),
            "title": runner.get_scene_title(scene_id).to_string(),
            "base_parcel": [base.x, base.y],
            "entity_count": runner.debug_list_entities(scene_id).len(),
            "paused": runner.get_scene_is_paused(scene_id),
            "ticks": int("ticks"),
            "js_tick_p95_ms": ms("js_tick_p95_ms"),
            "round_trip_p95_ms": ms("round_trip_p95_ms"),
            "apply_p95_ms": ms("apply_p95_ms"),
            "missed_pct": ms("missed_pct"),
            "skipped_ticks": int("skipped_ticks"),
        }));
    }
    Value::Array(out)
}

pub fn scene(scene_id: i64, filters: &Map<String, Value>) -> Value {
    if !scene_loaded(scene_id) {
        return error(format!("scene {scene_id} not loaded"));
    }
    // `limit` and `offset` cap the matched set expanded into full objects (0 means no limit).
    let limit = int(filters.get("limit"), 0).max(0) as usize;
    let offset = int(filters.get("offset"), 0).max(0) as usize;
    let mut matched = matching_entities(scene_id, filters);
    let matched_total = matched.len();
    matched.drain(..offset.min(matched.len()));
    if limit > 0 {
        matched.truncate(limit);
    }
    // Keyed by id, so an entity reached twice is emitted once, in a stable order.
    let mut by_id: BTreeMap<i64, Value> = BTreeMap::new();
    for entity_id in &matched {
        ensure_entity(scene_id, *entity_id, &mut by_id, filters, true);
    }
    json!({
        "scene_id": scene_id,
        "matched_count": matched_total,
        "returned_count": matched.len(),
        "offset": offset,
        "limit": limit,
        "entity_count": by_id.len(),
        "entities": by_id.into_values().collect::<Vec<_>>(),
    })
}

pub fn entity(scene_id: i64, entity_id: i64, filters: &Map<String, Value>) -> Value {
    if !scene_loaded(scene_id) {
        return error(format!("scene {scene_id} not loaded"));
    }
    let want_parents = flag(filters.get("include_parents"), true);
    let want_children = flag(filters.get("include_children"), true);
    let mut by_id = BTreeMap::new();
    ensure_entity(scene_id, entity_id, &mut by_id, filters, true);
    let Some(mut entry) = by_id.remove(&entity_id) else {
        return error(format!(
            "entity {entity_id} not present in scene {scene_id}"
        ));
    };
    if want_parents {
        let mut parents = Vec::new();
        let mut parent = entity_parent(scene_id, entity_id);
        let mut guard = 0;
        while parent != -1 && parent != ROOT_ENTITY_ID && guard < PARENT_GUARD {
            parents.push(entity_object(scene_id, parent, false, filters));
            parent = entity_parent(scene_id, parent);
            guard += 1;
        }
        entry["parents"] = Value::Array(parents);
    }
    if want_children {
        let children: Vec<Value> = direct_children(scene_id, entity_id)
            .into_iter()
            .map(|child| entity_object(scene_id, child, false, filters))
            .collect();
        entry["children"] = Value::Array(children);
    }
    entry
}

fn entity_parent(scene_id: i64, entity_id: i64) -> i64 {
    globals::scene_runner()
        .bind()
        .debug_get_entity_parent(scene_id as i32, entity_id as i32)
}

fn entity_ids(scene_id: i64) -> Vec<i64> {
    globals::scene_runner()
        .bind()
        .debug_list_entities(scene_id as i32)
        .as_slice()
        .iter()
        .map(|id| i64::from(*id))
        .collect()
}

fn direct_children(scene_id: i64, parent_id: i64) -> Vec<i64> {
    entity_ids(scene_id)
        .into_iter()
        .filter(|id| *id != parent_id && entity_parent(scene_id, *id) == parent_id)
        .collect()
}

/// The entities a `scene` query matches: by id, by component name, and by a substring of a
/// top-level field of a component (`property_is`).
fn matching_entities(scene_id: i64, filters: &Map<String, Value>) -> Vec<i64> {
    let ids: Vec<i64> = filters
        .get("entity")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_i64).collect())
        .unwrap_or_default();
    let components: Vec<String> = filters
        .get("component")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let property = object(filters.get("property_is"));
    let (prop_component, prop_field) =
        (text(property.get("component")), text(property.get("field")));
    let contains = text(property.get("contains")).to_lowercase();
    let has_property_filter =
        !prop_component.is_empty() && !prop_field.is_empty() && !contains.is_empty();

    let mut result = Vec::new();
    for id in entity_ids(scene_id) {
        if !ids.is_empty() && !ids.contains(&id) {
            continue;
        }
        if components.is_empty() && !has_property_filter {
            result.push(id);
            continue;
        }
        // The cheap names listing first: no protobuf decode.
        if !components.is_empty() {
            let names = globals::scene_runner()
                .bind()
                .debug_get_entity_component_names(scene_id as i32, id as i32);
            let present =
                |wanted: &String| names.as_slice().iter().any(|n| n.to_string() == *wanted);
            if !components.iter().any(present) {
                continue;
            }
        }
        if has_property_filter {
            let payload = fetch_components(scene_id, id);
            let value = payload["lww"][&prop_component][&prop_field]
                .as_str()
                .map(str::to_lowercase)
                .or_else(|| {
                    let raw = &payload["lww"][&prop_component][&prop_field];
                    (!raw.is_null()).then(|| raw.to_string().to_lowercase())
                })
                .unwrap_or_default();
            if !value.contains(&contains) {
                continue;
            }
        }
        result.push(id);
    }
    result
}

fn ensure_entity(
    scene_id: i64,
    entity_id: i64,
    by_id: &mut BTreeMap<i64, Value>,
    filters: &Map<String, Value>,
    is_match: bool,
) {
    if let Some(known) = by_id.get_mut(&entity_id) {
        // Reaching a primary match again keeps its mark.
        if is_match {
            known["is_match"] = json!(true);
        }
        return;
    }
    by_id.insert(
        entity_id,
        entity_object(scene_id, entity_id, is_match, filters),
    );
    // Parents and children are walked from the top-level call only; they are off by default
    // because the expansion bypasses `limit` and can balloon a reply past the socket's buffer.
    let inherited = {
        let mut inherited = Map::new();
        inherited.insert("include_parents".into(), json!(false));
        inherited.insert("include_children".into(), json!(false));
        inherited.insert(
            "collect_nodes".into(),
            filters.get("collect_nodes").cloned().unwrap_or(json!({})),
        );
        inherited.insert(
            "tree".into(),
            filters.get("tree").cloned().unwrap_or(json!("3d")),
        );
        inherited
    };
    if flag(filters.get("include_parents"), false) {
        let mut parent = entity_parent(scene_id, entity_id);
        let mut guard = 0;
        while parent != -1 && parent != ROOT_ENTITY_ID && guard < PARENT_GUARD {
            ensure_entity(scene_id, parent, by_id, &inherited, false);
            parent = entity_parent(scene_id, parent);
            guard += 1;
        }
    }
    if flag(filters.get("include_children"), false) {
        for child in direct_children(scene_id, entity_id) {
            ensure_entity(scene_id, child, by_id, &inherited, false);
        }
    }
}

fn entity_object(
    scene_id: i64,
    entity_id: i64,
    is_match: bool,
    filters: &Map<String, Value>,
) -> Value {
    let components = fetch_components(scene_id, entity_id);
    let collect_nodes = object(filters.get("collect_nodes"));
    let tree = text(filters.get("tree"));
    let tree = if tree.is_empty() {
        "3d".to_string()
    } else {
        tree
    };
    json!({
        "scene_id": scene_id,
        "entity_id": entity_id,
        "is_match": is_match,
        "godot": godot_side(scene_id, entity_id, &collect_nodes, &tree),
        "sdk": {"components": components["lww"]},
        "sdk_gos": components["gos"],
    })
}

fn fetch_components(scene_id: i64, entity_id: i64) -> Value {
    let raw = globals::scene_runner()
        .bind()
        .debug_get_entity_components_json(scene_id as i32, entity_id as i32)
        .to_string();
    match serde_json::from_str::<Value>(&raw) {
        Ok(parsed @ Value::Object(_)) => parsed,
        _ => json!({"lww": {}, "gos": {}}),
    }
}

/// The renderer node of an entity: the entity's 3D node under its scene, or the UI control the
/// scene runner reports.
fn find_entity_node(scene_id: i64, entity_id: i64, tree: &str) -> Option<Gd<Node>> {
    if tree == "ui" {
        let instance = globals::scene_runner()
            .bind()
            .debug_get_entity_ui_control_id(scene_id as i32, entity_id as i32);
        if instance < 0 {
            return None;
        }
        return Gd::<Node>::try_from_instance_id(InstanceId::from_i64(instance)).ok();
    }
    let root = scene_nodes()
        .into_iter()
        .find(|n| i64::from(n.bind().get_scene_id()) == scene_id)?;
    root.get_children()
        .iter_shared()
        .filter_map(|child| child.try_cast::<DclNodeEntity3d>().ok())
        .find(|child| i64::from(child.bind().e_id()) == entity_id)
        .map(|child| child.upcast())
}

fn godot_side(
    scene_id: i64,
    entity_id: i64,
    collect_nodes: &Map<String, Value>,
    tree: &str,
) -> Value {
    let Some(node) = find_entity_node(scene_id, entity_id, tree) else {
        return json!({"present": false});
    };
    let mut data = node_header(&node, tree);
    if let Ok(node_3d) = node.clone().try_cast::<Node3D>() {
        data["visible"] = json!(node_3d.is_visible());
        data["local_transform"] = transform(node_3d.get_transform());
        data["global_transform"] = transform(node_3d.get_global_transform());
        if let Some(bounds) = world_aabb(&node_3d) {
            data["aabb"] = aabb(bounds);
        }
    } else if let Ok(control) = node.clone().try_cast::<Control>() {
        data["visible"] = json!(control.is_visible());
        data["position"] = vec2(control.get_position());
        data["size"] = vec2(control.get_size());
        data["global_position"] = vec2(control.get_global_position());
        data["modulate"] = variant_to_json(&control.get_modulate().to_variant());
        data["self_modulate"] = variant_to_json(&control.get_self_modulate().to_variant());
        data["z_index"] = json!(control.get_z_index());
        data["mouse_filter"] = json!(control.get_mouse_filter().ord());
        data["anchors"] = json!({
            "left": control.get_anchor(godot::builtin::Side::LEFT),
            "top": control.get_anchor(godot::builtin::Side::TOP),
            "right": control.get_anchor(godot::builtin::Side::RIGHT),
            "bottom": control.get_anchor(godot::builtin::Side::BOTTOM),
        });
    }
    dump_children(&node, collect_nodes, &mut data);
    data
}

fn node_header(node: &Gd<Node>, tree: &str) -> Value {
    json!({
        "present": true,
        "tree": tree,
        "node_path": node.get_path().to_string(),
        "node_class": node.get_class().to_string(),
        "name": node.get_name().to_string(),
    })
}

/// `collect_nodes` is `{child node name: [property, ...]}`: the client decides which children to
/// look at and which properties to read, with no component-specific cases here.
fn dump_children(node: &Gd<Node>, collect_nodes: &Map<String, Value>, data: &mut Value) {
    for (child_name, requested) in collect_nodes {
        let Some(child) = node.get_node_or_null(child_name.as_str()) else {
            continue;
        };
        let mut dump = json!({"node_class": child.get_class().to_string()});
        for property_name in requested
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            dump[property_name] = property(&child, property_name);
        }
        data[child_name] = dump;
    }
}

fn world_aabb(node: &Gd<Node3D>) -> Option<godot::builtin::Aabb> {
    if let Ok(visual) = node.clone().try_cast::<VisualInstance3D>() {
        return Some(visual.get_aabb());
    }
    node.get_children()
        .iter_shared()
        .filter_map(|child| child.try_cast::<VisualInstance3D>().ok())
        .map(|visual| visual.get_aabb())
        .reduce(|merged, next| merged.merge(next))
}

// ---- the app's own UI

pub fn app_ui(filters: &Map<String, Value>) -> Value {
    let depth = int(filters.get("depth"), 3);
    let collect_nodes = object(filters.get("collect_nodes"));
    let class_filter = text(filters.get("class_filter"));
    let name_contains = text(filters.get("name_contains")).to_lowercase();
    let include_scene_ui = flag(filters.get("include_scene_ui"), false);

    let Some(tree) = godot::classes::Engine::singleton()
        .get_main_loop()
        .and_then(|main_loop| main_loop.try_cast::<godot::classes::SceneTree>().ok())
    else {
        return error("scene tree unavailable".to_string());
    };
    let Some(window) = tree.get_root() else {
        return error("scene tree unavailable".to_string());
    };
    let (root, resolved_from) = match filters.get("path").and_then(Value::as_str) {
        Some(explicit) => match window.get_node_or_null(explicit) {
            Some(node) => (node, "explicit".to_string()),
            None => return error(format!("node not found at path: {explicit}")),
        },
        None => match APP_UI_ROOTS.iter().find_map(|candidate| {
            Some((window.get_node_or_null(*candidate)?, candidate.to_string()))
        }) {
            Some(found) => found,
            None => return error(format!("no app UI root found; tried {APP_UI_ROOTS:?}")),
        },
    };
    // The SDK UI is skipped relative to the resolved root, so an explicit path still gets it.
    let skip_path =
        (!include_scene_ui).then(|| format!("{}/{APP_UI_SDK_SUBTREE}", root.get_path()));
    json!({
        "tree": "app_ui",
        "root_path": root.get_path().to_string(),
        "resolved_from": resolved_from,
        "node": walk_app_ui(&root, depth, &collect_nodes, &class_filter, &name_contains, skip_path.as_deref()),
    })
}

fn walk_app_ui(
    node: &Gd<Node>,
    depth: i64,
    collect_nodes: &Map<String, Value>,
    class_filter: &str,
    name_contains: &str,
    skip_path: Option<&str>,
) -> Value {
    let mut data = json!({
        "name": node.get_name().to_string(),
        "class": node.get_class().to_string(),
        "node_path": node.get_path().to_string(),
    });
    if let Ok(item) = node.clone().try_cast::<CanvasItem>() {
        data["visible"] = json!(item.is_visible());
    }
    if let Ok(control) = node.clone().try_cast::<Control>() {
        data["position"] = vec2(control.get_position());
        data["size"] = vec2(control.get_size());
        data["z_index"] = json!(control.get_z_index());
    }
    // Keyed by node name, so the client can ask for extra properties of specific nodes.
    if let Some(requested) = collect_nodes.get(&node.get_name().to_string()) {
        let mut properties = Map::new();
        for name in requested
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            properties.insert(name.to_string(), property(node, name));
        }
        data["properties"] = Value::Object(properties);
    }
    if depth <= 0 {
        return data;
    }
    let mut children = Vec::new();
    for child in node.get_children().iter_shared() {
        if skip_path.is_some_and(|skip| child.get_path().to_string() == skip) {
            continue;
        }
        let entry = walk_app_ui(
            &child,
            depth - 1,
            collect_nodes,
            class_filter,
            name_contains,
            skip_path,
        );
        // A node that does not match is pruned unless something deeper does.
        let has_matches = entry["children"].as_array().is_some_and(|c| !c.is_empty());
        let mut keep = true;
        if !class_filter.is_empty() && child.get_class() != class_filter {
            keep = has_matches;
        }
        if keep
            && !name_contains.is_empty()
            && !child
                .get_name()
                .to_string()
                .to_lowercase()
                .contains(name_contains)
        {
            keep = has_matches;
        }
        if keep {
            children.push(entry);
        }
    }
    if !children.is_empty() {
        data["children"] = Value::Array(children);
    }
    data
}

// ---- avatars

pub fn avatars() -> Value {
    let rows = globals::avatars_scene().bind().debug_list_avatars();
    let mut out: Vec<Value> = rows
        .iter_shared()
        .map(|row| {
            Value::Object(
                row.iter_shared()
                    .map(|(key, value)| (key.stringify().to_string(), variant_to_json(&value)))
                    .collect(),
            )
        })
        .collect();
    out.sort_by_key(|row| row["entity_id"].as_i64().unwrap_or(0));
    Value::Array(out)
}

pub fn avatar(by: &str, value: &Value, filters: &Map<String, Value>) -> Value {
    let scene = globals::avatars_scene();
    let instance = {
        let scene = scene.bind();
        match by {
            "address" => scene
                .debug_get_avatar_instance_id_by_address(value.as_str().unwrap_or_default().into()),
            "alias" => scene.debug_get_avatar_instance_id_by_alias(value.as_i64().unwrap_or(-1)),
            "entity" => {
                scene.debug_get_avatar_instance_id_by_entity(value.as_i64().unwrap_or(-1) as i32)
            }
            // The local player has no comms identity: the value is ignored.
            "local" => scene.debug_get_local_player_instance_id(),
            _ => {
                return error(format!(
                    "unknown 'by' (expected address|alias|entity|local): {by}"
                ))
            }
        }
    };
    if instance < 0 {
        return error("avatar not found".to_string());
    }
    let Ok(node) = Gd::<Node>::try_from_instance_id(InstanceId::from_i64(instance)) else {
        return error("avatar instance no longer valid".to_string());
    };
    let identity = avatars()
        .as_array()
        .and_then(|rows| {
            rows.iter()
                .find(|row| row["instance_id"].as_i64() == Some(instance))
        })
        .cloned()
        .unwrap_or_else(|| json!({}));
    let mut godot = node_header(&node, "avatar");
    godot["present"] = json!(true);
    if let Ok(node_3d) = node.clone().try_cast::<Node3D>() {
        godot["visible"] = json!(node_3d.is_visible());
        godot["local_transform"] = transform(node_3d.get_transform());
        godot["global_transform"] = transform(node_3d.get_global_transform());
    }
    // Avatars have a deep child hierarchy (meshes, the animation player, ...): `collect_nodes` is
    // the most useful knob here.
    dump_children(&node, &object(filters.get("collect_nodes")), &mut godot);
    json!({"identity": identity, "godot": godot})
}
