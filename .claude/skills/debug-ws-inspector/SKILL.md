---
name: debug-ws-inspector
description: Use when inspecting the running Decentraland Godot Explorer client over the localhost debug WebSocket (port 9230), writing or interpreting `websocat` queries against `ws://127.0.0.1:9230`, or extending the debug server itself. Covers the JSON protocol, the five trees (`scene`/`entity`, `ui_scene`/`ui_entity`, `avatars`/`avatar`, `app_ui`, `ping`/`scenes`), the `focus` keyboard-focus tracker, the shared `filters` dict, and the wiring across `godot/src/tool/debug_server/` and the Rust `SceneManager::debug_*` / `AvatarScene::debug_*` hooks. Also covers the `eval` command for running arbitrary GDScript against the live client (non-production only). Trigger when the user mentions the debug WS server, port 9230, `DebugWs`, `debug_collector`, websocat against the client, running/evaluating GDScript against the running client, or asks how to query live scene/UI/avatar/focus state.
---

# Debug WebSocket inspector

A localhost-only WS server exposes a JSON inspection protocol for the live
client state, plus an `eval` command that runs arbitrary GDScript. Auto-starts
in debug builds (editor / debug exports); in release builds it stays off until
toggled in **Settings → Developer → "Debug WS Server"**. Binds to
`127.0.0.1:9230`. Hidden — and `eval` hard-disabled — in production builds.

## Wiring

- Autoload: `DebugWs` → `godot/src/tool/debug_server/debug_ws_server.gd`
- Data assembly: `godot/src/tool/debug_server/debug_collector.gd`
- Rust `#[func]` hooks for state only Rust can reach:
  - `SceneManager::debug_*` (`lib/src/scene_runner/scene_manager.rs`) —
    CRDT enumeration, deserialization, UI control lookup.
  - `AvatarScene::debug_*` (`lib/src/avatars/avatar_scene.rs`) — avatar
    listing, address/alias/entity/local lookup.

## Protocol

Each frame is a JSON object with `id` (echoed in the reply) and `cmd`. Reply
shape: `{"id":..., "ok":true, "data":{...}}` or
`{"id":..., "ok":false, "error":"..."}`.

## Five trees, one protocol

| cmd | Tree | Identified by |
|---|---|---|
| `scene` / `entity` | 3D entity tree (`DclSceneNode` → `DclNodeEntity3d`) | `(scene_id, entity_id)` |
| `ui_scene` / `ui_entity` | per-scene SDK UI (`UiNode.base_control`) | `(scene_id, entity_id)` |
| `avatars` / `avatar` | global `AvatarScene` | `by` ∈ {`address`,`alias`,`entity`,`local`} |
| `app_ui` | Explorer's own UI | auto-detected (`/root/explorer/UI` or `/root/Menu`) |
| `ping` / `scenes` / `focus` | — | — |

All four data cmds (`scene`, `ui_scene`, `avatar`, `app_ui`) share a `filters` dict:
- `component: [...]` — OR-match SDK component names (cheap, no proto decode)
- `property_is: {component, field, contains}` — generic substring filter on
  any (SDK component, field) pair
- `collect_nodes: {<child_name>: [<property>, ...]}` — per-child-node
  property dump via `Object.get()`; values pass through `_variant_to_json`
- `include_parents`, `include_children`, `limit`, `offset`, `depth`,
  `class_filter`, `name_contains` — traversal/pagination knobs

## Quick examples

Use the bundled wrapper `scripts/debug-ws.sh` (relative to this SKILL.md) — it
sends a single JSON frame to `ws://127.0.0.1:9230` and bakes in `-B 16777216`
so websocat doesn't split large replies. Requires `websocat` on `$PATH`
(`cargo install websocat` or your package manager).

Why the buffer flag matters: websocat's default inbound buffer is 64 KiB.
`ping`/`scenes`/`avatars` fit easily, but `scene`/`ui_scene`/`app_ui`/`avatar`
replies routinely exceed that and websocat will split the frame
("Incoming message too long ... splitting it to parts"), which breaks JSON
parsing. The wrapper's `-B 16777216` (16 MiB) matches the server's 8 MiB
outbound cap with headroom.

```bash
# Confirm connection
./scripts/debug-ws.sh '{"id":1,"cmd":"ping"}'

# All loaded scenes
./scripts/debug-ws.sh '{"id":2,"cmd":"scenes"}'

# All TextShape entities in scene 0 with their Label3D properties
./scripts/debug-ws.sh '{"id":3,"cmd":"scene","scene_id":0,"filters":{
  "component":["TextShape"],
  "collect_nodes":{"TextShape":["text","font_size","pixel_size","outline_size","modulate"]}
}}'

# Your own avatar — what it's wearing + what's playing
./scripts/debug-ws.sh '{"id":4,"cmd":"avatar","by":"local","filters":{
  "collect_nodes":{"AnimationPlayer":["current_animation","autoplay"],"AnimationTree":["active"]}
}}'

# Explorer's own UI hierarchy (lobby in this state, scene UI when loaded)
./scripts/debug-ws.sh '{"id":5,"cmd":"app_ui","filters":{"depth":2}}'

# Keyboard-focus tracker — current owner + ui_root + change history
./scripts/debug-ws.sh '{"id":6,"cmd":"focus"}'
```

## Eval — running GDScript

`{"id":N,"cmd":"eval","code":"<gdscript>"}` compiles and runs `code` against the
live client and returns the serialized result — the agent-facing equivalent of
the old devtools console. **Non-production only**: in a production build the cmd
replies `{"ok":false,"error":"eval disabled in production builds"}`.

`code` is a GDScript function body. Use `return X` to send a value back. Three
locals are in scope:

| local | what |
|---|---|
| `tree` | the `SceneTree` (`tree.root`, `tree.get_node(...)`) |
| `global` | the `Global` autoload |
| `server` | this `DebugWsServer` node |

Autoloads (`Global`, `DebugWs`, …) and engine singletons (`OS`, `Engine`,
`Time`, …) are reachable directly too. A bare single-line expression is
auto-wrapped in `return`, so `"code":"1 + 1"` works without the keyword.

The result passes through the same `_variant_to_json` used elsewhere
(primitives, `Vector*`, `Color`, `AABB`, `Array`, `Dictionary`; everything else
— `Object`/`Node`/`Callable` — falls back to `str()`).

```bash
# Bare expression (auto-wrapped)
./scripts/debug-ws.sh '{"id":10,"cmd":"eval","code":"Engine.get_frames_per_second()"}'

# Reach into the tree
./scripts/debug-ws.sh '{"id":11,"cmd":"eval","code":"return tree.get_root().get_child_count()"}'

# Multi-line statement body (\n between lines, \t for indentation)
./scripts/debug-ws.sh '{"id":12,"cmd":"eval","code":"var names = []\nfor c in tree.get_root().get_children():\n\tnames.append(c.name)\nreturn names"}'
```

Limitations: **synchronous only** — `await` is not supported (it would return a
coroutine signal, not the awaited value). GDScript **runtime errors** (e.g. a
null access) are logged to the client console and the eval returns `null` with
`ok:true`; only *compile* errors come back as `ok:false`.

## `focus` — keyboard-focus tracker

`focus` takes no args and returns the viewport's current keyboard-focus owner
plus a timestamped change history (no `filters`). Reply `data`:
`{"current": "<path> [<class>]", "ui_root_path": "/root/explorer/UI",
"history": [{"t_ms", "frame", "from", "to"}, ...]}` (last `FOCUS_HISTORY_MAX`
= 64 changes; `"<none>"` means focus was released to null).

The server polls `get_viewport().gui_get_focus_owner()` every `_process` frame
(only while running) so the history captures transient changes — including
release-to-null, which the engine's `gui_focus_changed` signal misses.

Use it for "input stops working" bugs: mobile walk/jump are gated by
`player.gd` → `explorer_has_focus()` (`== ui_root.has_focus()`), so movement
silently dies whenever `current` ≠ `ui_root_path`. The history shows which
control stole focus and on which frame. (This is how the navbar-toggle
focus-steal bug was found: the gate read true→false when a press landed focus
on the navbar's full-rect `Button`.)

## Important notes

- The local-player avatar appears in `avatars` with `is_local: true` and lives
  on `SceneManager.player_avatar_node`, separate from the
  `AvatarScene.avatar_godot_scene` HashMap that tracks remote players.
- The `entity` cmd returns `godot.present: false` for CRDT entities the
  renderer hasn't instantiated yet — that's an instantiation throttle in
  large scenes, not a bug in the tool.
- `app_ui` skips `<root>/SceneUIContainer/scenes_ui` by default to avoid
  shadowing `ui_scene`; pass `include_scene_ui: true` to lift the skip.
- The inspection cmds are read-only; only `eval` can mutate client state.
  Loopback bind only — never exposed beyond the local machine.
- Per-peer outbound buffer is 8 MiB. If a reply exceeds it the server returns a
  short `{"ok":false,"error":"reply dropped (err=..., payload=..., buffer=...)"}`
  frame carrying the original `id`, so the client gets an actionable error
  instead of a silent hang — narrow `filters` (add `component` / `property_is`,
  drop `include_children`/`include_parents`, use `limit`).
- `scene` defaults `include_children` and `include_parents` to `false`. Pass
  them as `true` explicitly when you want the tree expanded; `entity` still
  inlines parents/direct children by default.

## Unified channel (debug-hub) — same surface, reachable on any device

The DebugWs server above LISTENS on loopback, so it's only reachable on the
machine running the client. For **device builds (esp. iOS, which can't be dialed
into)** use the **unified scene-inspector channel** instead: the device dials OUT
to a desktop **debug-hub**, and local tools (AI / websocat) connect to the hub.

Same command surface (`ping`/`scenes`/`scene`/`entity`/`ui_scene`/`ui_entity`/
`avatars`/`avatar`/`app_ui`/`focus`/`eval`), but spoken over the scene-inspector
CMD protocol — the source-of-truth contract an external inspector app already
parses, so additions stay backward-compatible.

Wire format (vs the loopback `{id,cmd}` form):
- request: `{"type":"SCENE_INSPECTOR_CMD","cmd":"<verb>","args":{...},"id":"<id>"}`
- reply:   `{"type":"SCENE_INSPECTOR_CMD_ACK","id":"<id>","ok":<bool>,"data":...}`
- streams (push): `{"type":"SCENE_INSPECTOR","payload":{"sessionId":...,"entries":[{type:...}]}}`
  where `entries[].type` ∈ crdt | op_call_start | op_call_end | scene_lifecycle |
  perf | **log** | **network** | session_start | session_end.

Bring it up:
```bash
cargo run -- debug-hub                       # device port 9231, consumer port 9230
# launch the client pointed at the hub's device port (LAN IP shown in the banner):
cargo run -- run -- --scene-inspector=ws://<this-mac>:9231          # desktop
cargo run -- run --target ios -- --scene-inspector=ws://<this-mac>:9231   # device
```

Drive it (helpers in `scripts/`):
```bash
scripts/unified.sh ping
scripts/unified.sh scene  '{"scene_id":0,"filters":{"component":["Transform"]}}'
scripts/unified.sh avatar '{"by":"local"}'
scripts/unified.sh eval   'Engine.get_frames_per_second()'
scripts/unified-tail.sh log,network          # subscribe + tail (opt-in streams)
```

**Capture is connection-gated + opt-in.** With no consumer connected, the device
captures NOTHING (no buffering) — safe to leave the tool enabled in prod. Classic
streams (crdt/perf) flow once a consumer connects; `log`/`network` are opt-in via
`subscribe`. `eval` is hard-gated out of production builds.

**MCP / AI loop:** the hub's consumer port (`ws://127.0.0.1:9230`) is a stable
local endpoint an MCP server (or the helpers above, called from Bash) can use to
read all logs and issue `eval`/queries — the same contract the external app uses.

## Recipes (use cases)

Setup once (`export H=.claude/skills/debug-ws-inspector/scripts`):
```bash
cargo run -- debug-hub                                    # the hub (terminal 1)
cargo run -- run -- --scene-inspector=ws://127.0.0.1:9231 # desktop client -> hub
# iOS device: cargo run -- run --target ios  (export plugin auto-bakes the hub
#   address); then log in + enter a world + accept the local-network prompt.
```
On desktop you can skip the hub for query/eval and hit the client's loopback
DebugWs directly with `debug-ws.sh` (the `{id,cmd}` form); logs still need the hub.

### 1. Inspect a scene
```bash
$H/unified.sh scenes                                           # loaded scenes (id/title/urn/count)
$H/unified.sh scene  '{"scene_id":0,"filters":{"limit":5}}'    # entities + their components
$H/unified.sh entity '{"scene_id":0,"entity_id":600}'          # one entity + parents/children
$H/unified.sh scene  '{"scene_id":0,"filters":{"component":["MeshRenderer"]}}'  # by component
$H/unified.sh avatars                                          # avatars present
$H/unified.sh avatar '{"by":"local"}'                          # your avatar (position, animations)
$H/unified.sh app_ui '{"filters":{"depth":2}}'                 # the explorer's own UI tree
```
Component histogram across the scene (via eval):
`$H/unified.sh eval` with a snippet that loops `debug_list_entities` ×
`debug_get_entity_component_names` and tallies — see "Tower of Madness" example
in the session notes (Transform/TextShape/GltfContainer counts).

### 2. See logs
```bash
$H/unified-tail.sh log | jq -r 'select(.type=="SCENE_INSPECTOR").payload.entries[]?
                               | select(.type=="log") | "[\(.source)] \(.msg)"'
$H/unified-tail.sh log,network                                 # + HTTP
```
Logs are opt-in (the helper subscribes for you) and connection-gated — nothing
flows without it. `source` ∈ rust | godot | native (Swift/ObjC on iOS).

### 3. Follow the app lifecycle via logs
Start tailing BEFORE the action, then trigger it (realm change / jump / re-enter world):
```bash
$H/unified-tail.sh log | jq -rc 'select(.type=="SCENE_INSPECTOR").payload.entries[]?
  | select(.type=="log" or .type=="scene_lifecycle")
  | if .type=="log" then "[\(.source)] \(.msg)"
    else "LIFECYCLE \(.event) scene=\(.scene_id)" end'
```
`scene_lifecycle` events: scene_init, main_crdt_loaded, script_loaded, on_start,
on_update(_end), scene_shutdown. Silence the per-tick on_update firehose with the
`set_lifecycle_verbose` command (`args:{"enabled":false}`) so boot/load events stay readable.

### 4. Instrument a feature you're building (add logs + verify)
The dev loop while working ON this branch:
1. Add a log at debug level for your feature:
   - Rust: `tracing::debug!("[myfeat] x={:?}", x);`  — use `debug!`, NOT `info!` (info ships to mobile/Sentry-adjacent paths).
   - GDScript: `print("[myfeat] ...")`  — captured as source `"godot"`.
2. Run with debug logging on for your module:
   ```bash
   cargo run -- run -- --scene-inspector=ws://127.0.0.1:9231 --rust-log='dclgodot::yourmod=debug,warn'
   ```
   (device: bake `--rust-log=...` into `DCL_IOS_GODOT_CMDLINE` next to `--scene-inspector=`.)
3. Tail just your tag while you exercise the feature:
   ```bash
   $H/unified-tail.sh log | jq -rc 'select(.type=="SCENE_INSPECTOR").payload.entries[]?
     | select(.type=="log" and (.msg|test("myfeat"))) | "[\(.source)] \(.msg)"'
   ```
4. Poke it live without redeploying — `eval` to read state or call your code:
   ```bash
   $H/unified.sh eval 'return Global.your_singleton.your_state'
   $H/unified.sh eval 'Global.your_singleton.trigger(); return "ok"'
   ```
   (`eval` mutates — non-prod only. Ideal for "does my new function actually do X?" with no rebuild.)

Caveats: GDScript `print` is captured but `push_warning`/`push_error` are NOT (only
Rust warn/error, via the tracing layer). Mobile's default filter is `info`, so
`debug!` lines need `--rust-log=...=debug`. Everything is connection-gated +
opt-in, so a prod build with the tool present captures nothing until you connect.
