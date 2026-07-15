# Genesis Plaza profiling benchmark

Reproducible FPS / frame-time / memory benchmark for Genesis Plaza on the
target devices (iPhone, Android — Android is min-spec). Used to triage where
the explorer is spending CPU before committing to a fix
(SDK7 toggles, RenderingServer migration, MultiMesh batching, etc.).

Tracked in [issue #1862](https://github.com/decentraland/godot-explorer/issues/1862).

## What it measures

Per-frame samples during a fixed window after a fixed warmup, summarised as
mean / p50 / p95 / min / max:

- FPS, process frame time, physics frame time
- Video / texture / buffer memory (MEMORY_STATIC and OS RSS report 0 on
  mobile builds — use `dumpsys meminfo <pid>` while the app is alive if you
  need real RSS for Android)
- Draw calls, render objects in frame, primitives
- Object / node / resource / orphan-node counts
- Physics active objects, collision pairs, island count

At end-of-run, the runner also walks the SceneTree and emits:

- **node type breakdown** — top-15 types with counts (so you can see how much
  of `node_count` is `MeshInstance3D` vs `CollisionShape3D` vs UI etc.)
- **mesh dedup buckets** — how many unique meshes are used 1× / 2-5× / 6-20×
  / 21+× (anything with high duplication is a `MultiMesh` candidate)

## How it stays reproducible

- **Genesis Plaza is pinned**: `genesis_plaza_commit` in
  `godot/bench/genesis_plaza.config.json` is checked out before every run.
- **Single trigger**: `--gp-benchmark` (desktop CLI) or `gp-benchmark=true`
  (deeplink). All knobs (durations, toggles, tag, output path) live in the
  config or as deeplink overrides — the CLI surface stays minimal.
- **Canonical pose**: warmup+sampling pin the player to a hardcoded
  position+look-at every frame, so two devices render the exact same
  viewpoint. Comms are held throughout so remote avatars don't appear.
- **Screenshot sanity check**: `scripts/bench/compare_screenshots.py` does a
  pHash compare against a reference; runs that diverge >20% are flagged.

## Configuration

The scripts read env vars when present. Copy `scripts/bench/.env.example` to
`scripts/bench/.env` (gitignored), fill in your values, and `launch_devices.sh`
will source it automatically.

| var | default | what it controls |
|---|---|---|
| `PREVIEW_HOST` | _none_ | `--preview` URL (e.g. `http://192.168.1.10:8000`); the IP/host the phones can reach where you ran `sdk-commands start` |
| `DEVELOPMENT_TEAM` | _none_ | iOS signing team passed to `xcodebuild`; ask the project lead for the value |
| `IOS_UDID` | autodetect | override which paired iPhone to target |
| `ANDROID_SERIAL` | autodetect | adb-native; use when multiple Android devices are connected |

```bash
cp scripts/bench/.env.example scripts/bench/.env
# edit scripts/bench/.env with your values
set -a; source scripts/bench/.env; set +a
```

## Running

### Mobile (primary path — both devices)

```bash
# 1. Start the pinned preview server on your dev box (one-time per session)
GP_DIR="$HOME/Library/Application Support/Godot/app_userdata/Decentraland/benchmark/genesis-plaza"
mkdir -p "$(dirname "$GP_DIR")"
[ -d "$GP_DIR/.git" ] || git clone https://github.com/decentraland-scenes/Genesis-Plaza-2025 "$GP_DIR"
(cd "$GP_DIR" && git checkout 30cdaffd752a02bc811dfdbd7e5aaaa4b97f595d && npm ci)
(cd "$GP_DIR" && npx @dcl/sdk-commands@7.23.0 start --port 8000)
# Note: pin sdk-commands to 7.23.0 — 7.22.4 returns 500 on /content/entities/active

# 2. Build + deploy to both devices (release)
cargo run -- export --target android --format apk --release
cargo run -- export --target ios --release
xcodebuild -project exports/decentraland-godot-client.xcodeproj \
    -scheme decentraland-godot-client -configuration Release \
    -destination "generic/platform=iOS" -derivedDataPath exports/build \
    -allowProvisioningUpdates CODE_SIGN_STYLE=Automatic \
    DEVELOPMENT_TEAM="${DEVELOPMENT_TEAM:?export DEVELOPMENT_TEAM in scripts/bench/.env}" build
adb install -r exports/decentraland.godot.client.apk
xcrun devicectl device install app --device "${IOS_UDID:-$(xcrun devicectl list devices | awk '/available \(paired\)/ {print $4; exit}')}" \
    exports/build/Build/Products/Release-iphoneos/decentraland-godot-client.app

# 3. Launch — PREVIEW_HOST env var fills --preview when set
scripts/bench/launch_devices.sh --both \
    --gp-benchmark \
    --param bench-tag=baseline \
    --param bench-warmup=20 \
    --param bench-sample=10 \
    --pull-results

# 4. Sanity-check screenshots
.bench-venv/bin/python scripts/bench/compare_screenshots.py \
    bench-results/devices-<run>/android/baseline.png \
    bench-results/devices-<run>/ios/baseline.png
```

### Toggle matrix without re-exporting

Mobile launches accept deeplink overrides, so the matrix doesn't need a
rebuild between runs:

| tag | extra params | what it isolates |
|---|---|---|
| `baseline` | (none) | current behavior |
| `no_tweens` | `--param bench-disable-tweens=true` | cost of SDK7 Tween updates |
| `no_transforms` | `--param bench-disable-transforms=true` | cost of applying CRDT Transform → Node3D |
| `no_tweens_no_xforms` | both | static-scene SDK7 floor |

### Profile (Android flamegraph)

```bash
# Run alongside launch_devices.sh — waits for PROFILE_WINDOW_BEGIN, captures
# simpleperf during the sampling phase, generates report.html + perf.data.
scripts/bench/profile_android.sh <tag>
```

iOS profiling needs Instruments / xctrace and isn't scripted here — attach
manually with `xcrun xctrace record --template "Time Profiler" --attach <pid>
--time-limit 30s` while the run is in `sampling` phase.

## Where the toggles live

| Toggle | Code path | Effect |
|---|---|---|
| `disable_tweens` | `lib/src/scene_runner/update_scene.rs` (`SceneUpdateState::Tween`) | Skips `update_tween` per-scene |
| `disable_transforms` | same file (`SceneUpdateState::TransformAndParent`) | Drops the dirty Transform set without applying to Node3D |

Both are `#[var(get, set)]` on `SceneManager`, set by `gp_benchmark_runner.gd`
once the explorer scene is loaded.
