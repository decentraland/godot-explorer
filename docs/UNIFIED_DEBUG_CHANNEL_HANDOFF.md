# Unified Debug Channel — Handoff

Session handoff (context ran out). Companion to `docs/UNIFIED_DEBUG_CHANNEL.md`
(the what/why/how) — this is the **state, what's pending, and how to continue**.

## State

- **Branch:** `tool/improve-logging-and-dev-connection` (tip `73dedb01`, merged with
  `origin/main`). **NOTHING IS COMMITTED** — all work is in the working tree.
- **Validated END-TO-END on a real iPhone 13** (see below). Builds + lints clean.
- The branch already had (committed, prior to this session): the `ios devtool`
  log-stream commit (`9110541b`), the plist on-demand export plugin
  (`c025b8d0`), and an audit-fixes `wip` (`eb6d7f98`).

### Uncommitted changes (21 modified + 2 new)
Rust lib: `scene_inspector/{logger,mod,dispatcher}.rs`, `network_inspector.rs`,
`log_stream.rs` (gutted to capture-only), `dcl_global.rs`, `dcl_cli.rs`.
GDScript: `scene_inspector_bridge.gd`, `scene_inspector_websocket.gd`,
`debug_ws_server.gd`, `dcl-ios-devtools/export_plugin.gd`, and removals in
`global.gd`/`explorer.gd`/`deep_link_router.gd`.
xtask: `log_server.rs` (now the hub), `main.rs`, `build_config.rs`, `run.rs`,
`Cargo.toml`.
New: `.claude/skills/debug-ws-inspector/scripts/unified.sh` + `unified-tail.sh`
(+ a "Unified channel" section appended to that skill's `SKILL.md`).

## What's done (all 5 phases + extras)

1. **Phase 1** — `log` + `network` folded into the scene-inspector bus as additive
   `entries[].type` (Rust). Unit tests pin the JSON shape.
2. **Phase 2** — `eval` + tree-query + `subscribe` verbs on `SCENE_INSPECTOR_CMD`;
   **connection-gated, opt-in capture** (the prod-safety requirement); shared
   `run_command` backend between DebugWs(:9230) and the bridge.
3. **Phase 3** — `cargo run -- debug-hub`: rendezvous broker (device connect-out ↔
   N consumers; fan-out + command relay). Relay verified.
4. **Phase 4** — native remote-debug groundwork: iOS `godot_cmdline` Info.plist
   injection (export plugin, opt-in via `DCL_IOS_GODOT_CMDLINE`, else auto-bakes the
   hub address); Android `adb reverse` on `--remote-debug` in `deploy_and_run_android`.
5. **Phase 5** — AI loop via `unified.sh` / `unified-tail.sh` helpers + skill docs
   (websocat against the hub consumer port). A formal MCP server is NOT built (see
   pending).
6. **Removed** `log-stream` + `log-server` (the unified channel supersedes them);
   kept the capture sinks.
7. **Simplification** — the iOS export plugin auto-bakes `--scene-inspector=ws://
   <lan-ip>:9231` in debug builds, so ALL launch methods (xtask deploy, Godot editor
   deploy) converge on the hub; "attach" = run `cargo run -- debug-hub`.

## Live validation (iPhone 13, "Tower of Madness")

Captured from a real device via the hub:
- Streaming in 7s: `crdt 600, scene_lifecycle 238, perf 4, log 7` entries (126 frames).
- `subscribe {log,network}` → `ok` and logs appeared (gated). 
- `ping` → `{engine:"4.6.2-stable", scenes_loaded:1}`.
- `eval Engine.get_frames_per_second()` → `18`.
- `scenes` → `[{scene_id:0, title:"Tower of Madness", entity_count:273, ...}]`.
- multiline `eval` (root children) → `["UiSounds","Global","DebugWs",...]`.
- Built `.app` Info.plist confirmed `godot_cmdline=[--scene-inspector=ws://192.168.2.50:9231]`
  + `NSAllowsLocalNetworking=true`.

Validation tooling used: a Node script over the global `WebSocket` (Node v24 via
nvm) against `ws://127.0.0.1:9230`; the iOS deploy was `DCL_IOS_GODOT_CMDLINE=
"--scene-inspector=ws://<mac-ip>:9231" cargo run -- run --target ios`, with the hub
started separately.

## Pending / not done

1. **Android end-to-end validation.** Phase 4 Android (`adb reverse` + `--remote-debug`)
   compiles but was NOT run on the device. The hub path on Android should also work
   (the export-plugin auto-bake is iOS-only; Android needs `--scene-inspector=` passed
   as an extra, or an Android equivalent of the auto-bake).
2. **xtask hub auto-start** for `cargo run -- run --target ios/android` — deferred. Now
   that `log-server`/port-9231 is gone there's no conflict; add `spawn_hub_background`
   in `log_server.rs` + call it (+ set `DCL_IOS_GODOT_CMDLINE`) in the run handler so a
   single command does build+deploy+hub.
3. **Native remote-debug not validated on device** (the editor-attach last mile). Needs
   `remote_host` = Mac LAN IP + editor listening. iOS uses the same `godot_cmdline`
   mechanism (`--remote-debug tcp://<mac-ip>:6007`).
4. **Formal MCP server** — only the websocat helpers exist. A small Node MCP server
   (zero-dep, global `WebSocket` + stdio JSON-RPC) wrapping the hub consumer port would
   give the assistant native tools (`eval`, `query`, `tail_logs`). Needs registration
   + a live client to test.
5. **Rebuild the dylib before any further run** — the host dylib was rebuilt +
   codesigned during validation, but any new lib change needs `cargo run -- build` +
   `codesign --force --sign -` on `libdclgodot.dylib` (macOS SIGKILL otherwise).
6. **Android auto-bake** of `--scene-inspector` (the iOS export plugin only covers iOS;
   Android currently needs the arg passed explicitly).

## Gotchas learned

- macOS: a rebuilt `libdclgodot.dylib` SIGKILLs Godot silently → `codesign --force
  --sign -` it (also the copies under `lib/target/libdclgodot_macos/` and `godot/`).
- `remote_host` = Mac **LAN IP**, NOT `0.0.0.0` (it's both bind addr and the
  connect-target baked into the app; `0.0.0.0` gives the app an invalid target).
- iOS GDScript `print()` → os_log, NOT the `--console` stdout stream; native NSLog
  does reach `--console`. The unified channel captures all of them via the sinks.
- The bridge connects in-world (`explorer.gd`), so the hub sees the device only after
  login + entering a world; accept the iOS local-network prompt once.
- `node`/`npm` aren't on the Bash PATH — `zsh -ic 'enable_nvm; node ...'` or the
  absolute path under `~/.nvm/...`.
- gdformat reformats the whole folder; `addons/` is gdlint-excluded but gdformat still
  touches it. Watch for unrelated churn (e.g. `global.gd`); `git checkout` it if so.

## How to continue (quick start)

```bash
# rebuild + sign the dylib if the lib changed
cargo run -- build && codesign --force --sign - lib/target/libdclgodot_macos/libdclgodot.dylib

# bring up the channel (desktop)
cargo run -- debug-hub &                 # hub
cargo run -- run -- --scene-inspector=ws://127.0.0.1:9231   # client

# drive it
.claude/skills/debug-ws-inspector/scripts/unified.sh eval 'Engine.get_frames_per_second()'
.claude/skills/debug-ws-inspector/scripts/unified-tail.sh log
```

For iOS: `DCL_IOS_GODOT_CMDLINE="--scene-inspector=ws://<mac-lan-ip>:9231" cargo run --
run --target ios` (or rely on the export plugin's auto-bake), with `cargo run --
debug-hub` running. Log in + enter a world + accept the local-network prompt.

## Validation status

`cargo check` (lib + xtask) ✅ · `cargo clippy` (lib) ✅ · `cargo fmt` ✅ ·
`gdformat`/`gdlint` ✅ · `cargo run -- check-gdscript` exit 0 (whole project) ✅ ·
Rust unit tests (JSON shape) ✅ · hub relay ✅ · **iOS device end-to-end ✅**.

Nothing committed — commit when ready (the user commits; do not auto-commit).
