#!/usr/bin/env bash
# launch_devices.sh — launch the explorer on iPhone and/or Android via deeplink,
# tail both logs in parallel, optionally pull benchmark results back.
#
# Usage:
#   scripts/bench/launch_devices.sh [--android|--ios|--both] \
#       [--realm URL] [--preview URL] [--position X,Y] \
#       [--gp-benchmark] [--param key=value]... [--no-tail] [--pull-results]
#
# Examples:
#   # GP benchmark on both, pulling results when done. <PREVIEW_HOST> is the
#   # IP/host where you ran `npx @dcl/sdk-commands start` (your dev box reachable
#   # from the phones — Tailscale or LAN IP).
#   scripts/bench/launch_devices.sh --both --gp-benchmark \
#       --preview http://<PREVIEW_HOST>:8000 --position 0,0 --pull-results
#
#   # Just open Genesis Plaza on Android against the live catalyst, tail log
#   scripts/bench/launch_devices.sh --android \
#       --realm https://realm-provider.decentraland.org/main --position 0,0
#
#   # Send arbitrary deeplink params (e.g. rust-log)
#   scripts/bench/launch_devices.sh --both --param rust-log=debug --param dclenv=zone
#
# Notes:
#   * URL values are URL-encoded automatically.
#   * Android uses `adb shell am start`; assumes one device connected (or set
#     ANDROID_SERIAL). iOS uses `xcrun devicectl --payload-url` and picks the
#     first paired device returned by `xcrun devicectl list devices`.
#   * Logs go to /tmp/dcl-bench-{android,ios}.log; tails are prefixed and run in
#     parallel until you Ctrl-C (unless --no-tail).
#   * --pull-results pulls /sdcard/Download/gp-benchmark/*.json from Android
#     once the autoload prints `END_RESULT_JSON`. iOS pulls aren't scripted
#     (sandbox); use `xcrun devicectl device copy from --domain-type
#     appDataContainer --domain-identifier org.decentraland.godotexplorer
#     --user mobile --source Documents/output/gp-benchmark/<tag>.json ...`

set -euo pipefail

# Auto-source per-developer config (gitignored). Holds PREVIEW_HOST,
# DEVELOPMENT_TEAM, IOS_UDID, ANDROID_SERIAL. See .env.example.
ENV_FILE="$(dirname "$0")/.env"
if [[ -f "$ENV_FILE" ]]; then
  set -a; source "$ENV_FILE"; set +a
fi

PKG="org.decentraland.godotexplorer"
DEVICES="both"
# Env-var overrides:
#   PREVIEW_HOST   default for --preview when unset (e.g. http://192.168.1.10:8000)
#   IOS_UDID       skip auto-detect, use this paired iPhone UDID
#   ANDROID_SERIAL respected natively by adb to disambiguate multiple devices
REALM=""
PREVIEW="${PREVIEW_HOST:-}"
POSITION=""
GP_BENCHMARK=0
EXTRA_PARAMS=()
TAIL=1
PULL_RESULTS=0
PROFILE=0
PROFILE_GPU=0

urlencode() {
  python3 -c 'import sys, urllib.parse; print(urllib.parse.quote(sys.argv[1], safe=""))' "$1"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --android) DEVICES="android" ;;
    --ios)     DEVICES="ios" ;;
    --both)    DEVICES="both" ;;
    --realm)         REALM="$2"; shift ;;
    --preview)       PREVIEW="$2"; shift ;;
    --position)      POSITION="$2"; shift ;;
    --gp-benchmark)  GP_BENCHMARK=1 ;;
    --param)         EXTRA_PARAMS+=("$2"); shift ;;
    --no-tail)       TAIL=0 ;;
    --pull-results)  PULL_RESULTS=1 ;;
    --profile)       PROFILE=1 ;;
    --profile-gpu)   PROFILE_GPU=1 ;;
    -h|--help)       sed -n '1,33p' "$0"; exit 0 ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
  shift
done

# gp-benchmark needs `position` in the deeplink to seed scene_fetcher's
# first update_position(). Without it, fresh-state runs (after pm clear /
# first install) hang at "loading 0%" because explorer.gd:262 only
# overrides last_parcel_position when `cmd_location != Vector2i.ZERO`.
# gp_benchmark_runner pins the pose post-load, but scene_fetcher needs the
# initial parcel set explicitly to start fetching.
if [[ "$GP_BENCHMARK" -eq 1 && -z "$POSITION" ]]; then
  POSITION="0,0"
fi

# Build the deeplink query string
PARAMS=()
[[ -n "$REALM"    ]] && PARAMS+=("realm=$(urlencode "$REALM")")
[[ -n "$PREVIEW"  ]] && PARAMS+=("preview=$(urlencode "$PREVIEW")")
[[ -n "$POSITION" ]] && PARAMS+=("position=$(urlencode "$POSITION")")
[[ "$GP_BENCHMARK" -eq 1 ]] && PARAMS+=("gp-benchmark=true")
for p in "${EXTRA_PARAMS[@]:-}"; do
  [[ -z "$p" ]] && continue
  k="${p%%=*}"; v="${p#*=}"
  PARAMS+=("$k=$(urlencode "$v")")
done

if [[ ${#PARAMS[@]} -eq 0 ]]; then
  echo "[launch] no params provided — opening app with bare deeplink"
  DEEPLINK="decentraland://open"
else
  IFS='&'; QUERY="${PARAMS[*]}"; unset IFS
  DEEPLINK="decentraland://open?${QUERY}"
fi
echo "[launch] deeplink: $DEEPLINK"

PIDS=()
PREVIEW_OWNED_PID=""
cleanup() {
  for p in "${PIDS[@]:-}"; do
    [[ -n "$p" ]] && kill "$p" 2>/dev/null || true
  done
  # Only kill the preview if we spawned it ourselves; pre-existing servers
  # belong to the user.
  if [[ -n "$PREVIEW_OWNED_PID" ]]; then
    kill "$PREVIEW_OWNED_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT INT TERM

# When --gp-benchmark is set and the user didn't pass --realm, spin up a
# pinned local preview from the commit declared in
# godot/bench/genesis_plaza.config.json. Idempotent: clones into
# ~/.cache/dcl-bench/Genesis-Plaza-2025-<short-sha>/ — outside any worktree
# so multiple branches share the same checkout. Reuses any preview already
# listening on port 8000.
ensure_pinned_preview() {
  local config="$REPO_ROOT/godot/bench/genesis_plaza.config.json"
  if [[ ! -f "$config" ]]; then
    echo "[gp-preview] config not found at $config; skipping clone" >&2
    return
  fi
  local repo_url commit
  repo_url=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("genesis_plaza_repo",""))' "$config")
  commit=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("genesis_plaza_commit",""))' "$config")
  if [[ -z "$repo_url" || -z "$commit" ]]; then
    echo "[gp-preview] config missing genesis_plaza_repo / genesis_plaza_commit; skipping" >&2
    return
  fi

  local short="${commit:0:8}"
  local cache_dir="${HOME}/.cache/dcl-bench/Genesis-Plaza-2025-${short}"

  if [[ ! -d "$cache_dir/.git" ]]; then
    echo "[gp-preview] cloning $repo_url @ $short -> $cache_dir"
    mkdir -p "$(dirname "$cache_dir")"
    # No --depth: we need to be able to checkout an arbitrary commit. The
    # repo is small enough that a full clone is cheap.
    git clone "$repo_url" "$cache_dir"
  fi

  local current_head
  current_head=$(git -C "$cache_dir" rev-parse HEAD 2>/dev/null || echo "")
  if [[ "$current_head" != "$commit" ]]; then
    echo "[gp-preview] checking out pinned commit $short"
    git -C "$cache_dir" fetch origin "$commit" 2>/dev/null || git -C "$cache_dir" fetch --all
    git -C "$cache_dir" -c advice.detachedHead=false checkout "$commit"
  fi

  local scene_dir="$cache_dir/central-plaza"
  if [[ ! -f "$scene_dir/scene.json" ]]; then
    echo "[gp-preview] central-plaza/scene.json not found in clone; aborting" >&2
    return
  fi

  if lsof -iTCP:8000 -sTCP:LISTEN >/dev/null 2>&1; then
    echo "[gp-preview] port 8000 already in use; reusing existing server"
    return
  fi

  local log_file="/tmp/gp-preview-${short}.log"
  echo "[gp-preview] starting sdk-commands from $scene_dir (logs: $log_file)"
  ( cd "$scene_dir" && nohup npx -y @dcl/sdk-commands@latest start \
      --no-debug --skip-build --no-browser --web-explorer --port 8000 \
      > "$log_file" 2>&1 & echo $! ) > /tmp/gp-preview-pid
  PREVIEW_OWNED_PID=$(cat /tmp/gp-preview-pid)
  rm -f /tmp/gp-preview-pid

  # Wait up to 90 s for the port to come up. First run downloads
  # @dcl/sdk-commands so the install can dominate the timeout.
  local waited=0
  while ! lsof -iTCP:8000 -sTCP:LISTEN >/dev/null 2>&1; do
    sleep 2
    waited=$((waited + 2))
    if [[ $waited -ge 90 ]]; then
      echo "[gp-preview] timed out waiting for port 8000; tail log:" >&2
      tail -20 "$log_file" >&2 || true
      return
    fi
  done
  echo "[gp-preview] preview ready (pid=$PREVIEW_OWNED_PID, ${waited}s)"
}

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
if [[ "$GP_BENCHMARK" -eq 1 && -z "$REALM" ]]; then
  ensure_pinned_preview
fi

# --profile spawns the matching profile_*.sh per device in parallel. Each
# profiler waits on its device's log for `PROFILE_WINDOW_BEGIN` and runs
# simpleperf (Android) / xctrace (iOS) for the duration emitted by
# gp_benchmark_runner.gd. Output → bench-results/profiles/<platform>-<tag>-<ts>/.
if [[ "$PROFILE" -eq 1 ]]; then
  bench_tag="profile"
  for p in "${EXTRA_PARAMS[@]:-}"; do
    [[ "$p" == bench-tag=* ]] && bench_tag="${p#bench-tag=}"
  done
  script_dir="$(dirname "$0")"
  if [[ "$DEVICES" == "android" || "$DEVICES" == "both" ]]; then
    if [[ -x "$script_dir/profile_android.sh" ]]; then
      echo "[profile] spawning Android profiler (tag=$bench_tag)"
      ( "$script_dir/profile_android.sh" "$bench_tag" 2>&1 \
          | sed -u 's/^/[profile-android] /' ) &
      PIDS+=($!)
    else
      echo "[profile] WARN: profile_android.sh not executable, skipping" >&2
    fi
  fi
  if [[ "$DEVICES" == "ios" || "$DEVICES" == "both" ]]; then
    if [[ -x "$script_dir/profile_ios.sh" ]]; then
      echo "[profile] spawning iOS profiler (tag=$bench_tag)"
      ( "$script_dir/profile_ios.sh" "$bench_tag" 2>&1 \
          | sed -u 's/^/[profile-ios] /' ) &
      PIDS+=($!)
    else
      echo "[profile] WARN: profile_ios.sh not present yet, skipping iOS profile" >&2
    fi
  fi
fi

# --profile-gpu spawns the perfetto/AGI Mali GPU profiler in parallel.
# Captures gpu.renderstages + gpu.counters during the bench's sampling
# window (matches duration_s emitted by gp_benchmark_runner.gd). Output
# → bench-results/profiles/android-gpu-<tag>-<ts>/.
if [[ "$PROFILE_GPU" -eq 1 ]]; then
  bench_tag="profile-gpu"
  for p in "${EXTRA_PARAMS[@]:-}"; do
    [[ "$p" == bench-tag=* ]] && bench_tag="${p#bench-tag=}"
  done
  script_dir="$(dirname "$0")"
  if [[ "$DEVICES" == "android" || "$DEVICES" == "both" ]]; then
    if [[ -x "$script_dir/profile_android_gpu.sh" ]]; then
      echo "[profile-gpu] spawning Android GPU profiler (tag=$bench_tag)"
      ( "$script_dir/profile_android_gpu.sh" "$bench_tag" 2>&1 \
          | sed -u 's/^/[profile-gpu-android] /' ) &
      PIDS+=($!)
    else
      echo "[profile-gpu] WARN: profile_android_gpu.sh not executable, skipping" >&2
    fi
  fi
fi

launch_android() {
  if ! command -v adb >/dev/null 2>&1; then
    echo "[android] adb not found, skipping" >&2
    return
  fi
  if ! adb get-state >/dev/null 2>&1; then
    echo "[android] no device connected, skipping" >&2
    return
  fi
  echo "[android] force-stop $PKG"
  adb shell am force-stop "$PKG" >/dev/null
  # gp-benchmark runs need a clean slate — the optimized scene fetcher
  # short-circuits to a locally cached zip whenever one exists, so any
  # asset-pipeline change made on the server would be invisible until
  # the device-side cache is wiped. `pm clear` removes app data
  # (including user://content/*-mobile.zip) but keeps the install.
  if [[ "$GP_BENCHMARK" -eq 1 ]]; then
    echo "[android] pm clear $PKG (gp-benchmark: wipe cached optimized zips)"
    adb shell pm clear "$PKG" >/dev/null
  fi
  adb logcat -c
  echo "[android] launching deeplink"
  # Quote the URL inside the device-side `am start` arg list — adb shell
  # re-tokenizes the command on the device, so a bare `&` in the URL would
  # background half the URL on the device side.
  adb shell "am start -W -a android.intent.action.VIEW -d '$DEEPLINK' $PKG" >/dev/null
  if [[ "$TAIL" -eq 1 ]]; then
    echo "[android] tailing logcat → /tmp/dcl-bench-android.log"
    ( adb logcat godot:V '*:S' 2>&1 | tee /tmp/dcl-bench-android.log \
        | sed -u 's/^/[android] /' ) &
    PIDS+=($!)
  fi
}

launch_ios() {
  if ! command -v xcrun >/dev/null 2>&1; then
    echo "[ios] xcrun not found, skipping" >&2
    return
  fi
  local udid="${IOS_UDID:-}"
  if [[ -z "$udid" ]]; then
    udid="$(xcrun devicectl list devices 2>/dev/null \
      | awk '/available \(paired\)/ {print $4; exit}')"
  fi
  if [[ -z "$udid" ]]; then
    echo "[ios] no paired iPhone found (set IOS_UDID to override), skipping" >&2
    return
  fi
  echo "[ios] launching on $udid"
  xcrun devicectl device process launch --device "$udid" \
    --terminate-existing --payload-url "$DEEPLINK" "$PKG" >/dev/null
  if [[ "$TAIL" -eq 1 ]]; then
    if command -v idevicesyslog >/dev/null 2>&1; then
      echo "[ios] tailing idevicesyslog → /tmp/dcl-bench-ios.log"
      ( idevicesyslog 2>&1 | grep --line-buffered -i decentraland \
          | tee /tmp/dcl-bench-ios.log | sed -u 's/^/[ios] /' ) &
      PIDS+=($!)
    else
      echo "[ios] idevicesyslog missing — install with 'brew install libimobiledevice'" >&2
    fi
  fi
}

[[ "$DEVICES" == "android" || "$DEVICES" == "both" ]] && launch_android
[[ "$DEVICES" == "ios"     || "$DEVICES" == "both" ]] && launch_ios

if [[ "$PULL_RESULTS" -eq 1 && "$TAIL" -eq 0 ]]; then
  # Pulling needs the END_RESULT_JSON marker to land in /tmp/dcl-bench-*.log,
  # which only happens when the tailers are running. Force them on.
  TAIL=1
  [[ "$DEVICES" == "android" || "$DEVICES" == "both" ]] && \
    ( adb logcat godot:V '*:S' 2>&1 | tee /tmp/dcl-bench-android.log >/dev/null ) &
  PIDS+=($!)
  if [[ "$DEVICES" == "ios" || "$DEVICES" == "both" ]] \
      && command -v idevicesyslog >/dev/null 2>&1; then
    ( idevicesyslog 2>&1 | grep --line-buffered -i decentraland \
        | tee /tmp/dcl-bench-ios.log >/dev/null ) &
    PIDS+=($!)
  fi
fi

if [[ "$PULL_RESULTS" -eq 1 ]]; then
  echo "[pull] waiting for END_RESULT_JSON in either log (30 min cap)"
  out_dir="bench-results/devices-$(date -u +%Y%m%dT%H%M%SZ)"
  mkdir -p "$out_dir"
  done_android=0
  done_ios=0
  [[ "$DEVICES" == "ios" ]] && done_android=1
  [[ "$DEVICES" == "android" ]] && done_ios=1
  for _ in {1..360}; do
    if [[ "$done_android" -eq 0 ]] \
        && grep -q "END_RESULT_JSON" /tmp/dcl-bench-android.log 2>/dev/null; then
      echo "[android] result detected, pulling"
      adb pull /sdcard/Download/gp-benchmark "$out_dir/android" 2>&1 | tail -3 || true
      done_android=1
    fi
    if [[ "$done_ios" -eq 0 ]] \
        && grep -q "END_RESULT_JSON" /tmp/dcl-bench-ios.log 2>/dev/null; then
      echo "[ios] result detected — manual pull required (sandbox)"
      done_ios=1
    fi
    [[ "$done_android" -eq 1 && "$done_ios" -eq 1 ]] && break
    sleep 5
  done
  echo "[pull] results dir: $out_dir"
fi

if [[ "$TAIL" -eq 1 && ${#PIDS[@]} -gt 0 ]]; then
  echo "[launch] tailing... Ctrl-C to stop"
  wait
fi
