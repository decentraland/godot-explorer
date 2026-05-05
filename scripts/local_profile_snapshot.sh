#!/usr/bin/env bash
#
# Local (no-Docker) end-to-end runner for the avatar/profile snapshot tool.
# Defaults to the Godot Compatibility renderer (--rendering-driver opengl3).
#
# What it does:
#   1. (Optional) Builds the Rust lib and exports the Linux Godot client.
#   2. Fetches one or more profiles from a Decentraland catalyst by wallet
#      address and transforms each into the avatars.json payload.
#   3. Invokes ./exports/decentraland.godot.client.x86_64 directly with
#      the chosen rendering method/driver and --avatar-renderer.
#
# Usage:
#   scripts/local_profile_snapshot.sh [flags] <wallet-address> [<wallet-address> ...]
#
# Flags:
#   --compatibility     Use the Compatibility renderer (default).
#                       Equivalent to: --rendering-method gl_compatibility
#                                      --rendering-driver opengl3
#   --mobile            Use the Mobile renderer (Vulkan).
#                       Equivalent to: --rendering-method mobile
#                                      --rendering-driver vulkan
#   --skip-build        Skip cargo build/export; reuse existing artifacts.
#   --debug             Build Rust lib in dev mode (default: release).
#   --headless          Run with --headless (no window). Off by default,
#                       since the renderer needs a real GL context to
#                       actually draw the avatar; only enable if you have
#                       Xvfb / EGL surfaceless set up.
#   --catalyst URL      Catalyst lambdas base
#                       (default: https://peer.decentraland.org/lambdas).
#   --content URL       Content base URL used by the renderer
#                       (default: https://peer.decentraland.org/content).
#   --output DIR        Host output directory (default: ./avatars-output).
#                       In --mobile mode, defaults to ./avatars-output-mobile
#                       unless overridden, so files don't overwrite the
#                       compatibility output.
#   --width N           Body image width (default: 256).
#   --height N          Body image height (default: 512).
#   --face-width N      Face image width (default: 256).
#   --face-height N     Face image height (default: 256).
#   --no-face           Skip face snapshot output.
#   -h | --help         Show this help.

set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
REPO_ROOT="$(cd -- "${SCRIPT_DIR}/.." >/dev/null 2>&1 && pwd)"

SKIP_BUILD=0
RELEASE_MODE=1
HEADLESS=0
RENDER_MODE="compatibility"   # one of: compatibility | mobile
CATALYST_URL="https://peer.decentraland.org/lambdas"
CONTENT_URL="https://peer.decentraland.org/content"
OUTPUT_DIR=""                 # resolved below depending on RENDER_MODE
OUTPUT_DIR_OVERRIDDEN=0
BODY_W=256
BODY_H=512
FACE_W=256
FACE_H=256
EMIT_FACE=1

ADDRESSES=()

color() { local c="$1"; shift; if [ -t 1 ]; then printf '\033[%sm%s\033[0m\n' "$c" "$*"; else printf '%s\n' "$*"; fi; }
info()  { color "1;34" "[info]  $*"; }
ok()    { color "1;32" "[ ok ]  $*"; }
warn()  { color "1;33" "[warn]  $*"; }
fail()  { color "1;31" "[fail]  $*"; }
die()   { fail "$*"; exit 1; }

print_help() { sed -n '2,42p' "$0" | sed 's/^# \{0,1\}//'; }

while [ $# -gt 0 ]; do
  case "$1" in
    --compatibility) RENDER_MODE="compatibility" ;;
    --mobile)        RENDER_MODE="mobile" ;;
    --skip-build) SKIP_BUILD=1 ;;
    --debug) RELEASE_MODE=0 ;;
    --headless) HEADLESS=1 ;;
    --catalyst) shift; CATALYST_URL="${1:?--catalyst needs a value}" ;;
    --content) shift; CONTENT_URL="${1:?--content needs a value}" ;;
    --output) shift; OUTPUT_DIR="${1:?--output needs a value}"; OUTPUT_DIR_OVERRIDDEN=1 ;;
    --width) shift; BODY_W="${1:?--width needs a value}" ;;
    --height) shift; BODY_H="${1:?--height needs a value}" ;;
    --face-width) shift; FACE_W="${1:?--face-width needs a value}" ;;
    --face-height) shift; FACE_H="${1:?--face-height needs a value}" ;;
    --no-face) EMIT_FACE=0 ;;
    -h|--help) print_help; exit 0 ;;
    --) shift; while [ $# -gt 0 ]; do ADDRESSES+=("$1"); shift; done; break ;;
    -*) die "unknown flag: $1 (try --help)" ;;
    *) ADDRESSES+=("$1") ;;
  esac
  shift || true
done

# Default output dir depends on render mode (so runs don't overwrite each other)
if [ "$OUTPUT_DIR_OVERRIDDEN" -eq 0 ]; then
  if [ "$RENDER_MODE" = "mobile" ]; then
    OUTPUT_DIR="${REPO_ROOT}/avatars-output-mobile"
  else
    OUTPUT_DIR="${REPO_ROOT}/avatars-output"
  fi
fi

[ "${#ADDRESSES[@]}" -gt 0 ] || die "at least one wallet address is required"

for addr in "${ADDRESSES[@]}"; do
  [[ "$addr" =~ ^0x[0-9a-fA-F]{40}$ ]] || die "invalid wallet address: $addr"
done

need() { command -v "$1" >/dev/null 2>&1 || die "missing required tool: $1"; }
need curl
need python3
[ "$SKIP_BUILD" -eq 1 ] || need cargo

cd "$REPO_ROOT"

if [ "$SKIP_BUILD" -eq 0 ]; then
  info "Building Rust lib (linux, $([ $RELEASE_MODE -eq 1 ] && echo release || echo dev))..."
  if [ "$RELEASE_MODE" -eq 1 ]; then cargo run -- build -r; else cargo run -- build; fi

  info "Exporting Godot Linux client..."
  cargo run -- export --target linux
else
  info "Skipping build/export (--skip-build)."
fi

EXPORT_BIN="${REPO_ROOT}/exports/decentraland.godot.client.x86_64"
EXPORT_PCK="${REPO_ROOT}/exports/decentraland.godot.client.pck"
LIB_DCL_GODOT="${REPO_ROOT}/lib/target/libdclgodot_linux/libdclgodot.so"

for f in "$EXPORT_BIN" "$EXPORT_PCK" "$LIB_DCL_GODOT"; do
  [ -f "$f" ] || die "missing artifact: $f (re-run without --skip-build)"
done
[ -x "$EXPORT_BIN" ] || chmod +x "$EXPORT_BIN"

mkdir -p "$OUTPUT_DIR"

info "Fetching ${#ADDRESSES[@]} profile(s)..."
declare -a PROFILE_FILES=()
for addr in "${ADDRESSES[@]}"; do
  pf="$(mktemp -t profile-XXXXXX.json)"
  PROFILE_FILES+=("$pf")
  url="${CATALYST_URL%/}/profiles/${addr}"
  info "  GET $url"
  curl -fsSL --retry 3 -H 'Accept: application/json' "$url" -o "$pf" \
    || die "failed to fetch profile for $addr from $url"
  [ -s "$pf" ] || die "empty response for $addr"
done

JSON_TMP="$(mktemp -t avatars-XXXXXX.json)"
cleanup() { rm -f "$JSON_TMP" "${PROFILE_FILES[@]}"; }
trap cleanup EXIT

info "Building avatars.json payload (absolute output paths)..."
ADDRS_JSON="$(printf '%s\n' "${ADDRESSES[@]}" | python3 -c 'import json,sys; print(json.dumps([l.strip() for l in sys.stdin if l.strip()]))')"
FILES_JSON="$(printf '%s\n' "${PROFILE_FILES[@]}" | python3 -c 'import json,sys; print(json.dumps([l.strip() for l in sys.stdin if l.strip()]))')"

CONTENT_URL="$CONTENT_URL" \
ADDRS_JSON="$ADDRS_JSON" \
FILES_JSON="$FILES_JSON" \
OUTPUT_DIR="$OUTPUT_DIR" \
BODY_W="$BODY_W" BODY_H="$BODY_H" FACE_W="$FACE_W" FACE_H="$FACE_H" \
EMIT_FACE="$EMIT_FACE" \
python3 - "$JSON_TMP" <<'PY'
import json, os, sys

out_path  = sys.argv[1]
addrs     = json.loads(os.environ["ADDRS_JSON"])
files     = json.loads(os.environ["FILES_JSON"])
content   = os.environ["CONTENT_URL"]
out_dir   = os.environ["OUTPUT_DIR"]
body_w    = int(os.environ["BODY_W"])
body_h    = int(os.environ["BODY_H"])
face_w    = int(os.environ["FACE_W"])
face_h    = int(os.environ["FACE_H"])
emit_face = os.environ["EMIT_FACE"] == "1"

payload = []
for addr, pf in zip(addrs, files):
    with open(pf) as f:
        data = json.load(f)
    avatars = data.get("avatars") or []
    if not avatars:
        sys.exit(f"no avatars in profile for {addr}")
    avatar = avatars[0].get("avatar")
    if not avatar:
        sys.exit(f"profile {addr} missing avatar field")

    entry = {
        "destPath": os.path.join(out_dir, f"{addr}.png"),
        "width": body_w,
        "height": body_h,
        "avatar": avatar,
    }
    if emit_face:
        entry["faceDestPath"] = os.path.join(out_dir, f"{addr}_face.png")
        entry["faceWidth"]    = face_w
        entry["faceHeight"]   = face_h
    payload.append(entry)

with open(out_path, "w") as f:
    json.dump({"baseUrl": content, "payload": payload}, f, indent=2)
PY

ok "Wrote payload: $JSON_TMP"

# Run the renderer locally. We run from REPO_ROOT so the gdextension's
# relative path res://../lib/target/libdclgodot_linux/libdclgodot.so resolves
# correctly (executable lives in ./exports/, lib in ./lib/target/...).
case "$RENDER_MODE" in
  compatibility)
    RENDER_ARGS=(--rendering-method gl_compatibility --rendering-driver opengl3)
    ;;
  mobile)
    RENDER_ARGS=(--rendering-method mobile --rendering-driver vulkan)
    ;;
  *) die "unknown render mode: $RENDER_MODE" ;;
esac

info "Render mode: $RENDER_MODE  (${RENDER_ARGS[*]})"

RUN_ARGS=("${RENDER_ARGS[@]}" --avatar-renderer --avatars "$JSON_TMP")
[ "$HEADLESS" -eq 1 ] && RUN_ARGS=(--headless "${RUN_ARGS[@]}")

info "Running: $EXPORT_BIN ${RUN_ARGS[*]}"
"$EXPORT_BIN" "${RUN_ARGS[@]}" || die "renderer exited non-zero"

MISSING=0
for addr in "${ADDRESSES[@]}"; do
  if [ -s "$OUTPUT_DIR/${addr}.png" ]; then ok "body : $OUTPUT_DIR/${addr}.png"; else fail "body missing for $addr"; MISSING=1; fi
  if [ "$EMIT_FACE" -eq 1 ]; then
    if [ -s "$OUTPUT_DIR/${addr}_face.png" ]; then ok "face : $OUTPUT_DIR/${addr}_face.png"; else fail "face missing for $addr"; MISSING=1; fi
  fi
done
[ "$MISSING" -eq 0 ] || die "some snapshots were not produced"
ok "Done."
