---
name: mobile-dev-debug-tool
description: Use when inspecting or reporting the state of the running Decentraland Godot Explorer client — on desktop or on a mobile device (iOS/Android) — over the unified scene-inspector debug-hub (`cargo run -- debug-hub`; device port 9231, consumer port 9230) that the client dials out to. Covers the SCENE_INSPECTOR_CMD JSON protocol, the five trees (`scene`/`entity`, `ui_scene`/`ui_entity`, `avatars`/`avatar`, `app_ui`, `ping`/`scenes`), the `focus` keyboard-focus tracker, the `log`/`network` streams, the shared `filters` dict, the `websocat` helper scripts (`scripts/unified.sh`, `unified-tail.sh`), and the wiring across `lib/crates/dcl_engine/src/tools/debug_hub/` and the Rust `SceneManager::debug_*` / `AvatarScene::debug_*` hooks. Also covers the typed commands `state`, `teleport`, `screenshot` and `log_filter` that replaced the old GDScript `eval`. Trigger when the user asks what state the running app/client is in (what scenes/realm are loaded, where the avatar is, what the UI is showing — desktop or on-device/mobile/iOS/Android), asks to connect to or host the debug-hub, or mentions the scene-inspector channel, debug-hub, port 9230/9231, websocat against the client, or teleporting/screenshotting/dumping the state of the running client.
---

# Mobile dev debug tool — scene-inspector debug-hub

A single WebSocket channel exposes live client state (scenes / entities / UI /
avatars / focus), a `log`/`network`/lifecycle stream, and typed commands
(`state`, `teleport`, `screenshot`, `log_filter`) — reachable on **any platform, including iOS/Android
devices that can't be dialed into**. The client dials **out** to a desktop
**debug-hub**; local tools (AI / websocat) connect to the hub's consumer port.

There is **one transport**: the scene-inspector CMD protocol (the source-of-truth
contract an external inspector app already parses, so additions stay
backward-compatible). The mutating commands (`teleport`, `log_filter`) are disabled in production builds.

## Bring up the hub

```bash
cargo run -- debug-hub                       # device port 9231, consumer port 9230
# launch the client pointed at the hub's device port (LAN IP shown in the banner):
cargo run -- run -- --scene-inspector=ws://127.0.0.1:9231              # desktop
cargo run -- run --target ios -- --scene-inspector=ws://<this-mac>:9231   # device
```

On iOS pass the hub address with `--scene-inspector=ws://<this-mac>:9231`; accept the local-network prompt on first launch. The bridge activates at **boot**
(`SceneInspectorBridge`, started by `DclGlobal` at boot),
so the channel is live from the lobby, **before login** — no need to enter a world
first. A `?scene-inspector=ws://<lan-ip>:9231` deeplink is honoured only on
non-production builds and only when it points at the local network.

## Answering "what state is the app in?" — ONE step

Run the pre-armed connector **in the background**, read its output, then query:

```bash
scripts/hub-connect.sh          # Bash tool: run_in_background: true — read its output
```

`hub-connect.sh` does the whole cold-start dance in one shot: wires Android
`adb reverse`, ensures a hub (reuse or start), waits ≤35 s for the device to dial
in, and prints either `=== CONNECTED ===` + a `ping` snapshot or a `NO DEVICE`
relaunch hint. If it started the hub it keeps the task alive so the hub persists.
Then query with the id-filtered helpers:

```bash
scripts/unified.sh scenes
scripts/unified.sh scene '{"scene_id":0,"filters":{"limit":5}}'
scripts/unified.sh state
```

**Why it just works:** the client **dials out** and **retries forever** (backoff
1s→…→30 s cap, `debug_hub/socket.rs`), and debug builds **default the
target to `ws://127.0.0.1:9231`** with no arg, so a hub started
*after* the app is picked up within ≤30 s — no app restart needed. Per platform:

- **iOS** — pass `--scene-inspector=ws://<LAN-IP>:9231`; accept the Local Network prompt once.
- **Android** — NO plugin bakes the arg (the baking plugin is iOS-only, and Android
  editor-deploy CLI args don't reach the app). The **debug loopback default** +
  `adb reverse tcp:9231` (set by `hub-connect.sh`) carry it instead. A build made
  *before* that default won't connect — rebuild+redeploy, or use
  `cargo run -- run --target android`.
- **desktop** — `cargo run -- run -- --scene-inspector=ws://127.0.0.1:9231`, or an
  editor F5 also auto-dials loopback.

If `hub-connect.sh` reports `NO DEVICE`, the app simply isn't dialing — follow the
hint it prints (usually: relaunch/redeploy the app).

## Wiring

- Commands, collectors and the transport: `lib/crates/dcl_engine/src/tools/debug_hub/`
  (`commands.rs`, `collector.rs`, `socket.rs`, the `SceneInspectorBridge` node). The hub is the
  `debug-hub` xtask (`src/log_server.rs`).
- Rust `#[func]` hooks for state only Rust can reach:
  - `SceneManager::debug_*` (`lib/src/scene_runner/scene_manager.rs`) —
    CRDT enumeration, deserialization, UI control lookup.
  - `AvatarScene::debug_*` (`lib/src/avatars/avatar_scene.rs`) — avatar
    listing, address/alias/entity/local lookup.

## Protocol (scene-inspector CMD)

- request: `{"type":"SCENE_INSPECTOR_CMD","cmd":"<verb>","args":{...},"id":"<id>"}`
- reply:   `{"type":"SCENE_INSPECTOR_CMD_ACK","id":"<id>","ok":<bool>,"data":...}`
  (or `{"ok":false,"error":"..."}`)
- streams (push): `{"type":"SCENE_INSPECTOR","payload":{"sessionId":...,"entries":[{type:...}]}}`
  where `entries[].type` ∈ crdt | op_call_start | op_call_end | scene_lifecycle |
  perf | **log** | **network** | session_start | session_end.

The `id` is echoed in the ACK — always match replies by it (see the perf-vs-ACK
note below). The helpers (`scripts/unified.sh`) do this for you.

## Five trees, one command surface

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

## Querying — `unified.sh`

`scripts/unified.sh <cmd> [args-json]` sends one CMD frame to the hub consumer
port (`ws://127.0.0.1:9230`) and returns its matching ACK. It bakes in
`-B 16777216` so websocat doesn't split large replies, and keeps the socket open
until the ACK's `id` matches (needed for on-device round-trips). Requires
`websocat` on `$PATH` (`cargo install websocat` or your package manager).

```bash
# Confirm the round-trip to the connected client
scripts/unified.sh ping

# All loaded scenes
scripts/unified.sh scenes

# All TextShape entities in scene 0 with their Label3D properties
scripts/unified.sh scene '{"scene_id":0,"filters":{
  "component":["TextShape"],
  "collect_nodes":{"TextShape":["text","font_size","pixel_size","outline_size","modulate"]}
}}'

# Your own avatar — what it's wearing + what's playing
scripts/unified.sh avatar '{"by":"local","filters":{
  "collect_nodes":{"AnimationPlayer":["current_animation","autoplay"],"AnimationTree":["active"]}
}}'

# Explorer's own UI hierarchy (lobby in this state, scene UI when loaded)
scripts/unified.sh app_ui '{"filters":{"depth":2}}'

# Keyboard-focus tracker — current owner + ui_root + change history
scripts/unified.sh focus
```

## Typed commands

There is no `eval`: the GDScript interpreter is gone with the GDScript. What it was used for has
typed commands (the mutating ones are non-production only):

```bash
scripts/unified.sh state                              # realm, parcel, scenes loaded, fps, memory
scripts/unified.sh teleport '{"x":77,"y":1}'          # move the player to a parcel
scripts/unified.sh screenshot '{"inline":true}'       # PNG of the viewport (inline = base64)
scripts/unified.sh log_filter '{"filter":"dclgodot::comms=debug,warn"}'
```

`eval` itself replies `ok:false` with a pointer to these. For a query nothing covers, add a typed
command in `debug_hub/commands.rs`.

## `focus` — keyboard-focus tracker

`focus` takes no args and returns the viewport's current keyboard-focus owner
plus a timestamped change history (no `filters`). Reply `data`:
`{"current": "<path> [<class>]", "ui_root_path": "/root/explorer/UI",
"history": [{"t_ms", "frame", "from", "to"}, ...]}` (last `FOCUS_HISTORY_MAX`
= 64 changes; `"<none>"` means focus was released to null).

The bridge polls the viewport's focus owner every frame (in debug builds) so the history captures transient changes — including
release-to-null, which the engine's `gui_focus_changed` signal misses.

Use it for "input stops working" bugs: mobile walk/jump are gated by
the player body's focus check (`ui_root.has_focus()`), so movement
silently dies whenever `current` ≠ `ui_root_path`. The history shows which
control stole focus and on which frame. (This is how the navbar-toggle
focus-steal bug was found: the gate read true→false when a press landed focus
on the navbar's full-rect `Button`.)

## Streams — `unified-tail.sh`

```bash
scripts/unified-tail.sh                 # logs only (default)
scripts/unified-tail.sh log,network     # logs + HTTP
scripts/unified-tail.sh log,lifecycle   # logs + per-tick scene lifecycle
```

**Capture is connection-gated + opt-in.** With no consumer connected, the device
captures NOTHING (no buffering) — safe to leave the tool enabled in prod. Classic
streams (crdt/perf) flow once a consumer connects; `log`/`network`/`lifecycle` are
opt-in (the helper subscribes for you). `source` ∈ rust | godot | native
(Swift/ObjC on iOS).

**MCP / AI loop:** the hub's consumer port (`ws://127.0.0.1:9230`) is a stable
local endpoint an MCP server (or the helpers above, called from Bash) can use to
read all logs and issue queries — the same contract the external app uses.

## Important notes

- **Report state from a request/reply ACK, never from a stray `perf` frame.**
  On the hub, `perf` (and `crdt`) are always-on *pushes* that start the instant a
  consumer connects — so a naïve read of the socket returns a `perf` frame, not
  the answer to your command. Always query with the id-filtered request/reply
  (`unified.sh <cmd>`), which matches the ACK by its unique `id`. If a query comes
  back empty, that's a dropped ACK — do **not** fall back to inferring app state
  from a `perf` push (it only happens to carry `fps`/`mem`/`scene_count`; it has no
  realm/avatar/UI, and its `scene_count: 0` can mislead you into "everything's
  fine / in lobby"). On a device the ACK needs a full consumer→hub→device→hub
  round-trip, so it lands *after* the first pushed `perf`: the helper must keep the
  socket open until the id matches (`unified.sh` does this via a trailing `sleep`
  that `grep -m1` tears down on match). Closing stdin right after sending — as an
  ad-hoc `printf frame | websocat` does — drops the ACK on-device.
- The local-player avatar appears in `avatars` with `is_local: true` and lives
  on `SceneManager.player_avatar_node`, separate from the
  `AvatarScene.avatar_godot_scene` HashMap that tracks remote players.
- The `entity` cmd returns `godot.present: false` for CRDT entities the
  renderer hasn't instantiated yet — that's an instantiation throttle in
  large scenes, not a bug in the tool.
- `app_ui` skips `<root>/SceneUIContainer/scenes_ui` by default to avoid
  shadowing `ui_scene`; pass `include_scene_ui: true` to lift the skip.
- The inspection cmds are read-only; only `teleport` and `log_filter` mutate client state.
- Large replies (expanded `scene` / `app_ui`) can be huge — narrow `filters`
  (add `component` / `property_is`, drop `include_children`/`include_parents`,
  use `limit`) if a reply is unwieldy.
- `scene` defaults `include_children` and `include_parents` to `false`. Pass
  them as `true` explicitly when you want the tree expanded; `entity` still
  inlines parents/direct children by default.

## Recipes (use cases)

Setup once (`export H=.claude/skills/mobile-dev-debug-tool/scripts`):
```bash
cargo run -- debug-hub                                    # the hub (terminal 1)
cargo run -- run -- --scene-inspector=ws://127.0.0.1:9231 # desktop client -> hub
# iOS device: cargo run -- run --target ios  (export plugin auto-bakes the hub
#   address); then log in + enter a world + accept the local-network prompt.
```

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
Component histogram across the scene: `scene` with `filters.component` per component name, then count.

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
   - Rust: `tracing::debug!("[myfeat] x={:?}", x);` — use `debug!`, not `info!`: mobile's default filter is `info`, so `info!` prints in every mobile session.
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
4. Poke it live without redeploying — `state`, `teleport` and `log_filter` change what you see
   without a rebuild; for anything else add a typed command.

Caveats: only Rust logs flow (via the tracing layer). Mobile's default filter is `info`, so
`debug!` lines need `--rust-log=...=debug`. Everything is connection-gated +
opt-in, so a prod build with the tool present captures nothing until you connect.
