#!/usr/bin/env bash
# profile_android_hiccups.sh — record a Perfetto system trace on the connected
# Android device while the explorer loads a scene, then pull it for
# scripts/bench/hiccup_report.py.
#
# The trace covers scheduling for every thread plus the engine/explorer zones
# (engine built with `scons profiler=perfetto`, see hiccup_report.py --help).
#
# Usage:
#   scripts/bench/profile_android_hiccups.sh [--duration SECONDS] [--out FILE]
#       [--launch 'DEEPLINK'] [--fresh]
#
#   --duration  trace length (default 240)
#   --out       local trace path (default bench-results/hiccups/<ts>.perfetto-trace)
#   --launch    deeplink to `am start` right after tracing begins, e.g.
#               'decentraland://open?realm=https%3A%2F%2Frealm-provider.decentraland.org%2Fmain&position=0%2C0'
#   --fresh     `pm clear` the app first (cold caches, new guest session)
#
# Reads $ANDROID_SERIAL from scripts/bench/.env if present.

set -euo pipefail

ENV_FILE="$(dirname "$0")/.env"
[[ -f "$ENV_FILE" ]] && set -a && source "$ENV_FILE" && set +a

PKG="org.decentraland.godotexplorer"
DURATION=240
OUT=""
LAUNCH=""
FRESH=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --duration) DURATION="$2"; shift ;;
    --out)      OUT="$2"; shift ;;
    --launch)   LAUNCH="$2"; shift ;;
    --fresh)    FRESH=1 ;;
    -h|--help)  sed -n '2,20p' "$0"; exit 0 ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
  shift
done

cd "$(git rev-parse --show-toplevel)"
if [[ -z "$OUT" ]]; then
  mkdir -p bench-results/hiccups
  OUT="bench-results/hiccups/$(date -u +%Y%m%dT%H%M%SZ).perfetto-trace"
fi

DEVICE_CFG=/data/local/tmp/perfetto_hiccup.cfg
DEVICE_OUT=/data/misc/perfetto-traces/hiccup.perfetto-trace

sed "s/__DURATION_MS__/$((DURATION * 1000))/" scripts/bench/perfetto_hiccup.cfg \
  | adb shell "cat > $DEVICE_CFG"

if [[ "$FRESH" == 1 ]]; then
  echo "[hiccups] pm clear $PKG"
  adb shell pm clear "$PKG" >/dev/null
fi
adb shell "am force-stop $PKG; rm -f $DEVICE_OUT" >/dev/null 2>&1 || true

echo "[hiccups] tracing ${DURATION}s -> $DEVICE_OUT"
adb shell "cat $DEVICE_CFG | perfetto -c - --txt -o $DEVICE_OUT" &
PERFETTO_PID=$!
sleep 2

if [[ -n "$LAUNCH" ]]; then
  echo "[hiccups] launching: $LAUNCH"
  adb shell "am start -W -a android.intent.action.VIEW -d '$LAUNCH' $PKG" >/dev/null
fi

# `adb shell` may report 255 when the shell session drops after perfetto finished writing.
wait "$PERFETTO_PID" || echo "[hiccups] perfetto session ended with status $? (pulling anyway)"
adb pull "$DEVICE_OUT" "$OUT" >/dev/null
echo "[hiccups] trace: $OUT ($(du -h "$OUT" | cut -f1))"
echo "[hiccups] report: ~/.venvs/perfetto/bin/python scripts/bench/hiccup_report.py $OUT"
