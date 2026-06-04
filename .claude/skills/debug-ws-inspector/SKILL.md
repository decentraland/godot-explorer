---
name: debug-ws-inspector
description: Use when inspecting the running Decentraland Godot Explorer client over the localhost debug WebSocket (port 9230), writing or interpreting `websocat` queries against `ws://127.0.0.1:9230`, or extending the debug server itself. Covers the JSON protocol, the five trees (`scene`/`entity`, `ui_scene`/`ui_entity`, `avatars`/`avatar`, `app_ui`, `ping`/`scenes`), the `focus` keyboard-focus tracker, the shared `filters` dict, and the wiring across `godot/src/tool/debug_server/` and the Rust `SceneManager::debug_*` / `AvatarScene::debug_*` hooks. Trigger when the user mentions the debug WS server, port 9230, `DebugWs`, `debug_collector`, websocat against the client, or asks how to query live scene/UI/avatar/focus state.
---

# Debug WebSocket inspector

A localhost-only WS server exposes a JSON inspection protocol for the live
client state. Off by default; turn on in **Settings → Developer → "Debug WS
Server"**. Binds to `127.0.0.1:9230`. Hidden in production builds.

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
- Read-only. No cmd mutates client state. Loopback bind only — never exposed
  beyond the local machine.
- Per-peer outbound buffer is 8 MiB. If a reply exceeds it the server returns a
  short `{"ok":false,"error":"reply dropped (err=..., payload=..., buffer=...)"}`
  frame carrying the original `id`, so the client gets an actionable error
  instead of a silent hang — narrow `filters` (add `component` / `property_is`,
  drop `include_children`/`include_parents`, use `limit`).
- `scene` defaults `include_children` and `include_parents` to `false`. Pass
  them as `true` explicitly when you want the tree expanded; `entity` still
  inlines parents/direct children by default.
