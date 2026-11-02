# Unified Debug Channel

A single bidirectional channel to observe and control a running Decentraland
Godot client from a desktop tool (or an AI), on **any platform — including an iOS
device**. It unifies what used to be several overlapping dev tools (log-stream,
log-server, the DebugWs tree server, the scene-inspector, the network
inspector) into one protocol over one transport.

## Why it exists

The tools didn't compose because of **transport direction**:

- **DebugWs** (a GDScript server, since removed) LISTENed on `127.0.0.1:9230` — rich
  tree queries + `eval`, but a listening server *on a device* is unreachable from
  the Mac without a reverse tunnel (and iOS has no `adb reverse`).
- **scene-inspector** + **log-stream** connect **OUT** — they work from any device
  because the device dials the desktop, never the other way around.

The scene-inspector already had a connect-out transport **and** an extensible,
additive-safe wire protocol **already consumed by an external inspector app**, so
it became the **source of truth**. Everything else folds onto it additively.

## Architecture

```
        ┌──────────── device (iOS / Android / desktop) ────────────┐
        │ scene_inspector dispatcher (mpsc) = the one event bus:    │
        │   crdt · op_call_* · scene_lifecycle · perf · session_*   │
        │   + log   (Rust + GDScript + native Swift/ObjC)           │
        │   + network (HTTP)                                        │
        │                                                           │
        │ SCENE_INSPECTOR_CMD verbs (shared run_command backend):   │
        │   ping · scenes · scene · entity · ui_scene · ui_entity   │
        │   · avatars · avatar · app_ui · focus · state · teleport      │
        │   · subscribe/unsubscribe                                 │
        │                                                           │
        │ Transport: connect-OUT ws://hub   (capture is gated:      │
        │   nothing runs until a consumer connects AND subscribes)  │
        └────────────────────────────────────────────────────────────┘
                 │ connect-out (scene-inspector protocol)
                 ▼
   ┌── desktop debug-hub (`cargo run -- debug-hub`) ──┐
   │ device-facing :9231  ◀── device dials out         │
   │ consumer-facing :9230 (loopback) ◀── AI / websocat / MCP / app
   │ fans device frames to all consumers; relays cmds back to device
   └───────────────────────────────────────────────────┘
```

The hub's consumer port (`:9230`) is the only local entry point; the old listening server and its
GDScript `eval` are gone, replaced by typed commands.

## The protocol (source of truth — additive only)

This is the contract an external inspector app already parses. **Never change or
remove existing keys; only add new `entries[].type` values or new CMD verbs.**

Outbound stream (device → consumer), `SceneInspectorBridge` (`debug_hub/bridge.rs`):
```json
{"type":"SCENE_INSPECTOR","payload":{"sessionId":"<uuid>","entries":[ {"type": "...", ...} ]}}
```
Entry types: `crdt`, `op_call_start`, `op_call_end`, `scene_lifecycle`, `perf`,
`session_start`, `session_end`, **`log`**, **`network`**.
- `log`  = `{t, source:"rust"|"godot"|"native", level?, target?, file?, line?, msg}`
- `network` = `{t, id, phase, url?, method?, requester?, status?, ok?, error?}`

Inbound command (consumer → device):
```json
{"type":"SCENE_INSPECTOR_CMD","cmd":"<verb>","args":{...},"id":"<id>"}
```
Reply: `{"type":"SCENE_INSPECTOR_CMD_ACK","id":"<id>","ok":bool,"data":...}`.

Verbs: `pause`, `resume`, `reload_scene`, `get_status`, `set_file_logging`,
`set_perf_interval`, `set_lifecycle_verbose`, `set_include_bin_payload`,
`subscribe`/`unsubscribe {streams:[...]}`, and (delegated to the shared backend)
`ping`, `scenes`, `scene`, `entity`, `ui_scene`, `ui_entity`, `avatars`,
`avatar`, `app_ui`, `focus`, `node_query`, and the typed `state`, `teleport {x,y}`,
`screenshot {inline?}`, `log_filter {filter}` (`eval` replies with an error pointing to them).
Production builds serve only the inspection verbs (`PRODUCTION_COMMANDS` in `debug_hub/bridge.rs`): no `teleport`, `log_filter`, `pause`/`resume`,
`reload_scene` or `set_file_logging`, and `node_query` returns props but skips `calls`.

## Production safety (connection-gated, opt-in)

**With no consumer connected, producers do nothing — no buffering, even if the
tool is left enabled in a prod build.** A master atomic `CONSUMER_CONNECTED`
(`lib/src/tools/scene_inspector/mod.rs`) gates every producer; the WS bridge flips
it on WS open / off on close. `log`/`network` are additionally opt-in via
`subscribe` (default off). Classic streams (crdt/perf/lifecycle) flow once a
consumer connects (back-compat). On disconnect, opt-in streams reset off. Impact
in prod with no connection ≈ zero.

## How to use it

```bash
# 1. Start the rendezvous hub (device port 9231, consumer port 9230)
cargo run -- debug-hub

# 2. Launch the client pointed at the hub's device port (LAN IP shown in banner):
cargo run -- run -- --scene-inspector=ws://<this-mac-ip>:9231              # desktop
cargo run -- run --target ios -- --scene-inspector=ws://<this-mac-ip>:9231 # device

#   A `decentraland://open?scene-inspector=ws://<this-mac-ip>:9231` deeplink also
#   works on non-production builds, as long as the target is on the local network.
#   Production builds ignore the deeplink: a tappable URL is not device access, so
#   there the target only comes from the CLI flag or the baked iOS key.

# 3. Drive it (helpers in .claude/skills/mobile-dev-debug-tool/scripts/):
unified.sh ping
unified.sh scene  '{"scene_id":0,"filters":{"component":["Transform"]}}'
unified.sh avatar '{"by":"local"}'
unified.sh state
unified-tail.sh   log,network         # subscribe + tail (opt-in streams)
```

The hub's consumer port (`ws://127.0.0.1:9230`) is a stable local endpoint — an
MCP server (or the helpers above from Bash) reads all logs and issues
queries there, which is the **AI loop**. See the `mobile-dev-debug-tool` skill.

## iOS specifics

- **`godot_cmdline` Info.plist array** is how Godot's iOS template receives launch
  args (`add_cmdline` in godotengine `drivers/apple_embedded/main_utilities.mm`).
  Pass the hub's `--scene-inspector=` address yourself; the local-network plist keys
  (`NSLocalNetworkUsageDescription`, `NSAllowsLocalNetworking`) belong to debug builds only.
- The bridge connects **in-world** (the world root), so the channel comes alive
  after login + entering a world; accept the iOS "local network" prompt once.

## Native Godot remote-debug (separate, optional)

The editor's Debugger panel (breakpoints / Remote scene tree / profiler) is a
**different channel** from the hub and uses `EditorSettings
network/debug/remote_host`:

- Set `remote_host` to the **Mac's LAN IP** (not `0.0.0.0`) — it's both the editor
  bind addr and the connect-target baked into the app.
- **Android**: `cargo run -- run --target android -- --remote-debug tcp://127.0.0.1:6007`
  auto-runs `adb reverse tcp:6007` (`deploy_and_run_android`).
- **iOS / editor deploy**: the editor auto-injects `--remote-debug` from
  `remote_host`; for the xtask path, pass it via `DCL_IOS_GODOT_CMDLINE`.
- The hub (logs + commands) does NOT use `remote_host`.

## What was removed

The standalone **`log-stream`** feature and **`log-server`** xtask subcommand were
removed — the unified channel + hub supersede them by design. The **capture
machinery was kept** (the Godot logger, the `LogHubLayer`, the iOS fd capture in
`lib/src/tools/log_stream.rs`), repurposed to feed the scene-inspector stream via
`emit_log`. The `debug-hub` replaces the `log-server` collector (and is a superset:
it also serves consumers + relays commands).

## Key files

- Protocol, commands and transport: `lib/crates/dcl_engine/src/tools/debug_hub/`,
  `lib/crates/dcl_engine/src/tools/scene_inspector/`
- Log capture: `lib/src/tools/log_stream.rs`; network: `lib/src/tools/network_inspector.rs`
- Hub (xtask): `src/log_server.rs` (`debug-hub`), `src/main.rs`
- Helpers + docs: `.claude/skills/mobile-dev-debug-tool/`
