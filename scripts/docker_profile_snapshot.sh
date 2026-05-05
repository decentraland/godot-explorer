#!/usr/bin/env bash
#
# End-to-end builder + runner for the avatar/profile snapshot Docker image.
#
# What it does:
#   1. (Optional) Builds the Rust lib and exports the Linux Godot client.
#   2. Stages the Docker build context (binary, .pck, libdclgodot.so,
#      libsentry, crashpad_handler, entry-point.sh, Dockerfile).
#   3. Builds the Docker image.
#   4. Fetches one or more profiles from a Decentraland catalyst by wallet
#      address and transforms each into the avatars.json payload the
#      renderer expects.
#   5. Runs the image with the JSON + an output dir mounted, producing
#      <output>/<address>.png and <output>/<address>_face.png per profile.
#
# Usage:
#   scripts/docker_profile_snapshot.sh [flags] <wallet-address> [<wallet-address> ...]
#
# Flags:
#   --skip-build         Skip cargo build/export; reuse existing artifacts.
#   --skip-image         Skip docker build; reuse existing image tag.
#   --debug              Build Rust lib in dev mode (default: release).
#   --image TAG          Docker image tag to build/run (default: godot-explorer:profile-snapshot).
#   --catalyst URL       Catalyst lambdas base, e.g. https://peer.decentraland.org/lambdas
#                        (default: https://peer.decentraland.org/lambdas).
#   --content URL        Content base URL used by the renderer
#                        (default: https://peer.decentraland.org/content).
#   --output DIR         Host output directory (default: ./avatars-output).
#   --width N            Body image width (default: 256).
#   --height N           Body image height (default: 512).
#   --face-width N       Face image width (default: 256).
#   --face-height N      Face image height (default: 256).
#   --no-face            Skip face snapshot output.
#   --keep-context       Keep the staged build context dir for inspection.
#   -h | --help          Show this help.

set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
REPO_ROOT="$(cd -- "${SCRIPT_DIR}/.." >/dev/null 2>&1 && pwd)"

# Defaults
SKIP_BUILD=0
SKIP_IMAGE=0
RELEASE_MODE=1
IMAGE_TAG="godot-explorer:profile-snapshot"
CATALYST_URL="https://peer.decentraland.org/lambdas"
CONTENT_URL="https://peer.decentraland.org/content"
OUTPUT_DIR="${REPO_ROOT}/avatars-output"
BODY_W=256
BODY_H=512
FACE_W=256
FACE_H=256
EMIT_FACE=1
KEEP_CONTEXT=0

ADDRESSES=()

color() { # $1 color $2 message
  local c="$1"; shift
  if [ -t 1 ]; then printf '\033[%sm%s\033[0m\n' "$c" "$*"; else printf '%s\n' "$*"; fi
}
info()  { color "1;34" "[info]  $*"; }
ok()    { color "1;32" "[ ok ]  $*"; }
warn()  { color "1;33" "[warn]  $*"; }
fail()  { color "1;31" "[fail]  $*"; }
die()   { fail "$*"; exit 1; }

print_help() {
  sed -n '2,38p' "$0" | sed 's/^# \{0,1\}//'
}

while [ $# -gt 0 ]; do
  case "$1" in
    --skip-build) SKIP_BUILD=1 ;;
    --skip-image) SKIP_IMAGE=1 ;;
    --debug) RELEASE_MODE=0 ;;
    --image) shift; IMAGE_TAG="${1:?--image needs a value}" ;;
    --catalyst) shift; CATALYST_URL="${1:?--catalyst needs a value}" ;;
    --content) shift; CONTENT_URL="${1:?--content needs a value}" ;;
    --output) shift; OUTPUT_DIR="${1:?--output needs a value}" ;;
    --width) shift; BODY_W="${1:?--width needs a value}" ;;
    --height) shift; BODY_H="${1:?--height needs a value}" ;;
    --face-width) shift; FACE_W="${1:?--face-width needs a value}" ;;
    --face-height) shift; FACE_H="${1:?--face-height needs a value}" ;;
    --no-face) EMIT_FACE=0 ;;
    --keep-context) KEEP_CONTEXT=1 ;;
    -h|--help) print_help; exit 0 ;;
    --) shift; while [ $# -gt 0 ]; do ADDRESSES+=("$1"); shift; done; break ;;
    -*) die "unknown flag: $1 (try --help)" ;;
    *) ADDRESSES+=("$1") ;;
  esac
  shift || true
done

[ "${#ADDRESSES[@]}" -gt 0 ] || die "at least one wallet address is required"

# Validate addresses (0x + 40 hex)
for addr in "${ADDRESSES[@]}"; do
  if ! [[ "$addr" =~ ^0x[0-9a-fA-F]{40}$ ]]; then
    die "invalid wallet address: $addr"
  fi
done

# Tools we need
need() { command -v "$1" >/dev/null 2>&1 || die "missing required tool: $1"; }
need docker
need curl
need python3
[ "$SKIP_BUILD" -eq 1 ] || need cargo

cd "$REPO_ROOT"

# 1. Build artifacts
if [ "$SKIP_BUILD" -eq 0 ]; then
  info "Building Rust lib (linux, $([ $RELEASE_MODE -eq 1 ] && echo release || echo dev))..."
  if [ "$RELEASE_MODE" -eq 1 ]; then
    cargo run -- build -r
  else
    cargo run -- build
  fi

  info "Exporting Godot Linux client..."
  cargo run -- export --target linux
else
  info "Skipping build/export (--skip-build)."
fi

# 2. Stage docker build context
EXPORT_BIN="${REPO_ROOT}/exports/decentraland.godot.client.x86_64"
EXPORT_PCK="${REPO_ROOT}/exports/decentraland.godot.client.pck"
LIB_DCL_GODOT="${REPO_ROOT}/lib/target/libdclgodot_linux/libdclgodot.so"
SENTRY_LIB="${REPO_ROOT}/godot/addons/sentry/bin/linux/x86_64/libsentry.linux.debug.x86_64.so"
CRASHPAD="${REPO_ROOT}/godot/addons/sentry/bin/linux/x86_64/crashpad_handler"
ENTRY_POINT="${REPO_ROOT}/entry-point.sh"
DOCKERFILE="${REPO_ROOT}/Dockerfile"

for f in "$EXPORT_BIN" "$EXPORT_PCK" "$LIB_DCL_GODOT" "$SENTRY_LIB" "$CRASHPAD" "$ENTRY_POINT" "$DOCKERFILE"; do
  [ -f "$f" ] || die "missing artifact: $f (re-run without --skip-build, or run cargo run -- build && cargo run -- export --target linux)"
done

if [ "$SKIP_IMAGE" -eq 0 ]; then
  CTX_DIR="$(mktemp -d -t dcl-snap-ctx-XXXXXX)"
  cleanup_ctx() { [ "$KEEP_CONTEXT" -eq 0 ] && rm -rf "$CTX_DIR" || info "Kept build context at: $CTX_DIR"; }
  trap cleanup_ctx EXIT

  info "Staging build context at $CTX_DIR ..."
  cp "$EXPORT_BIN"     "$CTX_DIR/decentraland.godot.client.x86_64"
  cp "$EXPORT_PCK"     "$CTX_DIR/decentraland.godot.client.pck"
  cp "$LIB_DCL_GODOT"  "$CTX_DIR/libdclgodot.so"
  cp "$SENTRY_LIB"     "$CTX_DIR/libsentry.linux.debug.x86_64.so"
  cp "$CRASHPAD"       "$CTX_DIR/crashpad_handler"
  cp "$ENTRY_POINT"    "$CTX_DIR/entry-point.sh"
  cp "$DOCKERFILE"     "$CTX_DIR/Dockerfile"

  info "Building Docker image: $IMAGE_TAG"
  docker build -t "$IMAGE_TAG" "$CTX_DIR"
  ok "Image built."
else
  info "Skipping docker build (--skip-image). Expecting $IMAGE_TAG to exist."
  docker image inspect "$IMAGE_TAG" >/dev/null 2>&1 || die "image $IMAGE_TAG not found locally"
fi

# 3. Fetch profiles + build avatars.json
mkdir -p "$OUTPUT_DIR"
JSON_TMP="$(mktemp -t avatars-XXXXXX.json)"
trap 'rm -f "$JSON_TMP"' EXIT

info "Fetching ${#ADDRESSES[@]} profile(s)..."
declare -a PROFILE_FILES=()
for addr in "${ADDRESSES[@]}"; do
  pf="$(mktemp -t profile-XXXXXX.json)"
  PROFILE_FILES+=("$pf")
  url="${CATALYST_URL%/}/profiles/${addr}"
  info "  GET $url"
  if ! curl -fsSL --retry 3 -H 'Accept: application/json' "$url" -o "$pf"; then
    die "failed to fetch profile for $addr from $url"
  fi
  if ! [ -s "$pf" ]; then
    die "empty response for $addr"
  fi
done

info "Building avatars.json payload..."
ADDRS_JSON="$(printf '%s\n' "${ADDRESSES[@]}" | python3 -c 'import json,sys; print(json.dumps([l.strip() for l in sys.stdin if l.strip()]))')"
FILES_JSON="$(printf '%s\n' "${PROFILE_FILES[@]}" | python3 -c 'import json,sys; print(json.dumps([l.strip() for l in sys.stdin if l.strip()]))')"

CONTENT_URL="$CONTENT_URL" \
ADDRS_JSON="$ADDRS_JSON" \
FILES_JSON="$FILES_JSON" \
BODY_W="$BODY_W" BODY_H="$BODY_H" FACE_W="$FACE_W" FACE_H="$FACE_H" \
EMIT_FACE="$EMIT_FACE" \
python3 - "$JSON_TMP" <<'PY'
import json, os, sys

out_path   = sys.argv[1]
addrs      = json.loads(os.environ["ADDRS_JSON"])
files      = json.loads(os.environ["FILES_JSON"])
content    = os.environ["CONTENT_URL"]
body_w     = int(os.environ["BODY_W"])
body_h     = int(os.environ["BODY_H"])
face_w     = int(os.environ["FACE_W"])
face_h     = int(os.environ["FACE_H"])
emit_face  = os.environ["EMIT_FACE"] == "1"

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
        "destPath": f"output/{addr}.png",
        "width": body_w,
        "height": body_h,
        "avatar": avatar,
    }
    if emit_face:
        entry["faceDestPath"] = f"output/{addr}_face.png"
        entry["faceWidth"]    = face_w
        entry["faceHeight"]   = face_h
    payload.append(entry)

with open(out_path, "w") as f:
    json.dump({"baseUrl": content, "payload": payload}, f, indent=2)
PY

ok "Wrote payload: $JSON_TMP"

# 4. Run the container
info "Running snapshot generator (output: $OUTPUT_DIR)..."
docker run --rm \
  -v "$JSON_TMP:/app/avatars.json:ro" \
  -v "$OUTPUT_DIR:/app/output" \
  "$IMAGE_TAG"

# 5. Verify output
MISSING=0
for addr in "${ADDRESSES[@]}"; do
  if [ -s "$OUTPUT_DIR/${addr}.png" ]; then
    ok "body : $OUTPUT_DIR/${addr}.png"
  else
    fail "body missing for $addr"
    MISSING=1
  fi
  if [ "$EMIT_FACE" -eq 1 ]; then
    if [ -s "$OUTPUT_DIR/${addr}_face.png" ]; then
      ok "face : $OUTPUT_DIR/${addr}_face.png"
    else
      fail "face missing for $addr"
      MISSING=1
    fi
  fi
done

[ "$MISSING" -eq 0 ] || die "some snapshots were not produced"
ok "Done."
