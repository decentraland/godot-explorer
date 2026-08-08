class_name DebugWsServer
extends Node

## Developer-only command backend for live scene / entity / UI / avatar
## inspection and GDScript `eval`. Exposes `run_command(cmd, args)`, driven by the
## scene-inspector unified channel (`scene_inspector_bridge.gd`) so the same
## surface is reachable on desktop and on-device via the debug-hub.
##
## Formerly this node also served a loopback `{id,cmd}` WebSocket on 9230; that
## transport + its protocol were removed — everything now goes over the unified
## scene-inspector CMD protocol. This node is kept as the shared command backend
## (and the keyboard-focus tracker for the `focus` cmd).
##
## Supported commands:
##   ping
##   scenes
##   scene     {scene_id, filters}                — 3D entity tree
##   entity    {scene_id, entity_id, filters}     — 3D entity tree
##   ui_scene  {scene_id, filters}                — 2D UI entity tree
##   ui_entity {scene_id, entity_id, filters}     — 2D UI entity tree
##   avatars                                       — list every tracked avatar
##   avatar    {by, value, filters}                — one avatar's detail
##   app_ui    {filters}                            — Explorer's own UI tree
##                                                    (skips per-scene SDK UI
##                                                    subtree by default).
##   eval      {code}                                — run GDScript, return result
##                                                    (non-production only).
##   focus                                          — keyboard-focus owner + history
##
## `ui_scene` / `ui_entity` are identical to their 3D counterparts except the
## `godot` block reports the rendered `Control` (rect, anchors, modulate, …)
## instead of the `Node3D`. The SDK payload is unchanged — UI and 3D entities
## share the same per-scene entity-id space.
##
## `avatars` / `avatar` query the global `AvatarScene` (not per-scene CRDT).
## Identify a single avatar via `by` ∈ {"address", "alias", "entity"} +
## `value`. The avatar tree uses its own SceneEntityId space; the entity_id
## here is not addressable through the `entity` command.

## Max focus-change entries retained for the `focus` diagnostic cmd.
const FOCUS_HISTORY_MAX: int = 64
## Keywords that mark an `eval` snippet as a statement body, not a bare expression.
const EVAL_STATEMENT_PREFIXES: Array[String] = [
	"return", "var", "const", "if", "for", "while", "match", "pass", "print", "assert"
]
const Collector := preload("res://src/tool/debug_server/debug_collector.gd")

## Focus tracking: poll the viewport's keyboard-focus owner each frame and log
## every change (including release-to-null, which `gui_focus_changed` misses).
## Exposed via the `focus` cmd. Diagnostic aid for "input stops working" bugs
## where movement is gated by `ui_root.has_focus()`.
var _focus_history: Array = []
var _last_focus_desc: String = "<unset>"


func _ready() -> void:
	# Poll keyboard focus each frame so the `focus` cmd has history. Same gate the
	# loopback server used to auto-start under — debug builds (editor / debug
	# exports), never in production.
	set_process(OS.is_debug_build() and not Global.is_production())


func _process(_dt: float) -> void:
	_poll_focus()


# --------------------------------------------------------------------
# Dispatch


## Shared command backend. Returns `{ok:true, data:...}` or `{ok:false, error:...}`.
## Driven by the scene-inspector unified channel (scene_inspector_bridge.gd) so
## the inspection/eval surface is transport-agnostic. `p` is the parameter dict
## (the scene-inspector CMD `args` object).
func run_command(cmd: String, p: Dictionary) -> Dictionary:
	match cmd:
		"ping":
			return {"ok": true, "data": _build_ping_data()}
		"focus":
			return {"ok": true, "data": _build_focus_data()}
		"scenes":
			return {"ok": true, "data": Collector.collect_scenes_summary()}
		"avatars":
			return {"ok": true, "data": Collector.collect_avatars()}
		"eval":
			return _run_eval(p)
		_:
			return _run_tree_query(cmd, p)


## Tree / entity / avatar inspection verbs. Split out of `run_command` to stay
## under the per-function return-count limit. All read-only; an invalid id falls
## through to the Collector, which returns a structured `{error}`.
func _run_tree_query(cmd: String, p: Dictionary) -> Dictionary:
	match cmd:
		"scene":
			return _wrap(Collector.collect_scene(int(p.get("scene_id", -1)), p.get("filters", {})))
		"entity":
			# `entity` and `scene` share the `filters` dict. Backwards-compat: also
			# accept `include_parents` / `include_children` at the top level.
			var filters_e: Dictionary = (p.get("filters", {}) as Dictionary).duplicate()
			if p.has("include_parents") and not filters_e.has("include_parents"):
				filters_e["include_parents"] = p["include_parents"]
			if p.has("include_children") and not filters_e.has("include_children"):
				filters_e["include_children"] = p["include_children"]
			return _wrap(
				Collector.collect_entity(
					int(p.get("scene_id", -1)), int(p.get("entity_id", -1)), filters_e
				)
			)
		"ui_scene":
			var ui_filters: Dictionary = (p.get("filters", {}) as Dictionary).duplicate()
			ui_filters["tree"] = "ui"
			return _wrap(Collector.collect_scene(int(p.get("scene_id", -1)), ui_filters))
		"ui_entity":
			var ui_ef: Dictionary = (p.get("filters", {}) as Dictionary).duplicate()
			ui_ef["tree"] = "ui"
			return _wrap(
				Collector.collect_entity(
					int(p.get("scene_id", -1)), int(p.get("entity_id", -1)), ui_ef
				)
			)
		"app_ui":
			return _wrap(Collector.collect_app_ui((p.get("filters", {}) as Dictionary).duplicate()))
		"avatar":
			var by: String = str(p.get("by", ""))
			if by.is_empty():
				return {"ok": false, "error": "missing 'by' (expected address|alias|entity|local)"}
			# `local` is keyless — all other modes require `value`.
			if by != "local" and not p.has("value"):
				return {"ok": false, "error": "missing 'value'"}
			return _wrap(
				Collector.collect_avatar(
					by, p.get("value", null), (p.get("filters", {}) as Dictionary).duplicate()
				)
			)
		_:
			return {"ok": false, "error": "unknown command: %s" % cmd}


## Wrap a Collector result (`{...}` or `{error:...}`) into the `{ok, data|error}` shape.
func _wrap(d: Dictionary) -> Dictionary:
	if d.has("error"):
		return {"ok": false, "error": str(d["error"])}
	return {"ok": true, "data": d}


## Run arbitrary GDScript. Hard-gated out of production builds (it can mutate state).
func _run_eval(p: Dictionary) -> Dictionary:
	if Global.is_production():
		return {"ok": false, "error": "eval disabled in production builds"}
	var code: String = str(p.get("code", ""))
	if code.is_empty():
		return {"ok": false, "error": "missing 'code'"}
	var res: Dictionary = _eval_gdscript(code)
	if res.get("ok", false):
		return {"ok": true, "data": res.get("data")}
	return {"ok": false, "error": str(res.get("error", "eval failed"))}


func _poll_focus() -> void:
	var tree := get_tree()
	if tree == null:
		return
	var vp := tree.root
	if vp == null:
		return
	var owner := vp.gui_get_focus_owner()
	var desc := _describe_focus(owner)
	if desc == _last_focus_desc:
		return
	(
		_focus_history
		. append(
			{
				"t_ms": Time.get_ticks_msec(),
				"frame": Engine.get_process_frames(),
				"from": _last_focus_desc,
				"to": desc,
			}
		)
	)
	if _focus_history.size() > FOCUS_HISTORY_MAX:
		_focus_history = _focus_history.slice(_focus_history.size() - FOCUS_HISTORY_MAX)
	_last_focus_desc = desc


func _describe_focus(node: Control) -> String:
	if node == null:
		return "<none>"
	return "%s [%s]" % [str(node.get_path()), node.get_class()]


func _build_focus_data() -> Dictionary:
	# `explorer_has_focus` (and thus mobile walk/jump) is true iff this matches
	# the explorer's `ui_root` (%UI). Compare `current` against it.
	var ui_root_path := "<no explorer>"
	var explorer := get_node_or_null("/root/explorer")
	if explorer != null and explorer.get("ui_root") != null:
		ui_root_path = str(explorer.ui_root.get_path())
	return {
		"current": _last_focus_desc,
		"ui_root_path": ui_root_path,
		"history": _focus_history,
	}


func _build_ping_data() -> Dictionary:
	var version: String = str(ProjectSettings.get_setting("application/config/version", "unknown"))
	var loaded: PackedInt32Array
	if is_instance_valid(Global.scene_runner):
		loaded = Global.scene_runner.debug_get_loaded_scene_ids()
	return {
		"version": version,
		"engine": Engine.get_version_info().get("string", ""),
		"scenes_loaded": loaded.size(),
	}


# --------------------------------------------------------------------
# Eval


## Compile and run a GDScript snippet, returning {ok, data} or {ok:false, error}.
## `code` is treated as a function body with three locals available:
## `tree` (SceneTree), `global` (the Global autoload) and `server` (this node).
## Use `return X` to send a value back. A bare single-line expression is also
## accepted and auto-wrapped in `return`. Synchronous only — `await` is not
## supported, and GDScript runtime errors are logged to the client console while
## the eval returns null.
func _eval_gdscript(code: String) -> Dictionary:
	# Pick the more likely shape first so the common case compiles cleanly; fall
	# back to the other shape on a compile failure (a misclassified snippet then
	# self-heals at the cost of one parse error in the client log).
	var expr_first := _looks_like_expression(code)
	var first := _compile_and_run(code, expr_first)
	if first.get("compiled", false):
		return first
	var second := _compile_and_run(code, not expr_first)
	if second.get("compiled", false):
		return second
	return {"ok": false, "error": second.get("error", "compile failed")}


func _looks_like_expression(code: String) -> bool:
	var trimmed := code.strip_edges()
	if trimmed.is_empty() or trimmed.contains("\n"):
		return false
	for prefix in EVAL_STATEMENT_PREFIXES:
		if (
			trimmed == prefix
			or trimmed.begins_with(prefix + " ")
			or trimmed.begins_with(prefix + "(")
		):
			return false
	return true


func _compile_and_run(code: String, as_expression: bool) -> Dictionary:
	var body := ""
	if as_expression:
		body = "\treturn (%s)\n" % code
	else:
		for line in code.split("\n"):
			body += "\t" + line + "\n"
	var script := GDScript.new()
	script.source_code = "extends RefCounted\n\n\nfunc _run(tree, global, server):\n" + body
	var err := script.reload()
	if err != OK:
		return {"compiled": false, "ok": false, "error": "compile failed (err=%d)" % err}
	var instance: Object = script.new()
	if instance == null or not instance.has_method("_run"):
		return {"compiled": true, "ok": false, "error": "internal: eval runner missing _run()"}
	var result: Variant = instance.call("_run", get_tree(), Global, self)
	return {"compiled": true, "ok": true, "data": Collector._variant_to_json(result)}
