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
    -h|--help)       sed -n '1,33p' "$0"; exit 0 ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
  shift
done

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
cleanup() {
  for p in "${PIDS[@]:-}"; do
    [[ -n "$p" ]] && kill "$p" 2>/dev/null || true
  done
}
trap cleanup EXIT INT TERM

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
