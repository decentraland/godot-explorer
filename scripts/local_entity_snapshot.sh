#!/usr/bin/env bash
#
# Local end-to-end runner for the avatar/profile snapshot tool, driven by
# **profile entity CIDs** (deployment ids) instead of wallet addresses.
#
# Companion to scripts/local_profile_snapshot.sh, intended for reproducing
# bulk profile-generation failures (e.g. issue #1880, where the Foundation
# reported 829 entity CIDs that fail to render).
#
# What it does:
#   1. (Optional) Builds Rust lib + exports the Linux Godot client.
#   2. For each CID, fetches the entity manifest from the catalyst at
#      /content/contents/<cid> and extracts metadata.avatars[0].avatar.
#   3. Splits the resulting payload into batches and invokes the renderer
#      once per batch (so a single crash doesn't take the whole run with it).
#   4. Classifies each entity as ok / fetch-fail / payload-malformed /
#      render-missing / render-blank, and writes a CSV + summary.
#
# Usage:
#   scripts/local_entity_snapshot.sh [flags] [<cid> ...]
#   scripts/local_entity_snapshot.sh --entities-file <path> [flags]
#
# Flags (in addition to the renderer/build flags below):
#   --entities-file PATH  Read CIDs from a file (one per line, '#' comments ok).
#   --limit N             Only process the first N CIDs (handy for smoke runs).
#   --batch-size N        CIDs per renderer invocation (default: 25).
#   --report-dir DIR      Where to write report.csv / summary.txt
#                         (default: <output>/_report).
#
# Renderer / build flags (mirror local_profile_snapshot.sh):
#   --compatibility | --mobile   Renderer (default: compatibility).
#   --skip-build                 Reuse existing build artifacts.
#   --debug                      Build Rust lib in dev mode.
#   --headless                   Run with --headless.
#   --catalyst URL               Catalyst lambdas base (default peer.decentraland.org/lambdas).
#   --content URL                Content base (default peer.decentraland.org/content).
#   --output DIR                 Output directory.
#   --width / --height           Body image size (default 256x512).
#   --face-width / --face-height Face image size (default 256x256).
#   --no-face                    Skip face snapshot.
#   -h | --help                  Show this help.

set -uo pipefail
# NOTE: deliberately NOT using `set -e` for the per-batch loop — one batch
# crashing must not kill the whole run.

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
REPO_ROOT="$(cd -- "${SCRIPT_DIR}/.." >/dev/null 2>&1 && pwd)"

SKIP_BUILD=0
RELEASE_MODE=1
HEADLESS=0
RENDER_MODE="compatibility"
CATALYST_URL="https://peer.decentraland.org/lambdas"   # unused for CIDs but kept for parity
CONTENT_URL="https://peer.decentraland.org/content"
OUTPUT_DIR=""
OUTPUT_DIR_OVERRIDDEN=0
REPORT_DIR=""
BODY_W=256
BODY_H=512
FACE_W=256
FACE_H=256
EMIT_FACE=1
BATCH_SIZE=25
LIMIT=0
ENTITIES_FILE=""

CIDS=()

color() { local c="$1"; shift; if [ -t 1 ]; then printf '\033[%sm%s\033[0m\n' "$c" "$*"; else printf '%s\n' "$*"; fi; }
info()  { color "1;34" "[info]  $*"; }
ok()    { color "1;32" "[ ok ]  $*"; }
warn()  { color "1;33" "[warn]  $*"; }
fail()  { color "1;31" "[fail]  $*"; }
die()   { fail "$*"; exit 1; }

print_help() { sed -n '2,46p' "$0" | sed 's/^# \{0,1\}//'; }

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
    --report-dir) shift; REPORT_DIR="${1:?--report-dir needs a value}" ;;
    --width) shift; BODY_W="${1:?--width needs a value}" ;;
    --height) shift; BODY_H="${1:?--height needs a value}" ;;
    --face-width) shift; FACE_W="${1:?--face-width needs a value}" ;;
    --face-height) shift; FACE_H="${1:?--face-height needs a value}" ;;
    --no-face) EMIT_FACE=0 ;;
    --batch-size) shift; BATCH_SIZE="${1:?--batch-size needs a value}" ;;
    --limit) shift; LIMIT="${1:?--limit needs a value}" ;;
    --entities-file) shift; ENTITIES_FILE="${1:?--entities-file needs a value}" ;;
    -h|--help) print_help; exit 0 ;;
    --) shift; while [ $# -gt 0 ]; do CIDS+=("$1"); shift; done; break ;;
    -*) die "unknown flag: $1 (try --help)" ;;
    *) CIDS+=("$1") ;;
  esac
  shift || true
done

if [ -n "$ENTITIES_FILE" ]; then
  [ -f "$ENTITIES_FILE" ] || die "entities file not found: $ENTITIES_FILE"
  while IFS= read -r line; do
    line="${line%%#*}"               # strip comments
    line="$(echo -n "$line" | tr -d '[:space:]')"
    [ -n "$line" ] && CIDS+=("$line")
  done < "$ENTITIES_FILE"
fi

if [ "$OUTPUT_DIR_OVERRIDDEN" -eq 0 ]; then
  if [ "$RENDER_MODE" = "mobile" ]; then
    OUTPUT_DIR="${REPO_ROOT}/avatars-output-entities-mobile"
  else
    OUTPUT_DIR="${REPO_ROOT}/avatars-output-entities"
  fi
fi
[ -n "$REPORT_DIR" ] || REPORT_DIR="$OUTPUT_DIR/_report"

[ "${#CIDS[@]}" -gt 0 ] || die "no entity CIDs provided (use positional args or --entities-file)"

# de-dupe CIDs while preserving order, validate format
declare -A SEEN=()
DEDUP=()
for cid in "${CIDS[@]}"; do
  [[ "$cid" =~ ^bafkrei[a-z0-9]{52}$ || "$cid" =~ ^Qm[1-9A-HJ-NP-Za-km-z]{44}$ ]] \
    || { warn "skipping invalid CID: $cid"; continue; }
  if [ -z "${SEEN[$cid]:-}" ]; then
    SEEN[$cid]=1
    DEDUP+=("$cid")
  fi
done
CIDS=("${DEDUP[@]}")

if [ "$LIMIT" -gt 0 ] && [ "${#CIDS[@]}" -gt "$LIMIT" ]; then
  CIDS=("${CIDS[@]:0:$LIMIT}")
fi

info "Entities to process: ${#CIDS[@]}  (batch size: $BATCH_SIZE, render: $RENDER_MODE)"

need() { command -v "$1" >/dev/null 2>&1 || die "missing required tool: $1"; }
need curl
need python3
[ "$SKIP_BUILD" -eq 1 ] || need cargo

cd "$REPO_ROOT"

if [ "$SKIP_BUILD" -eq 0 ]; then
  info "Building Rust lib (linux, $([ $RELEASE_MODE -eq 1 ] && echo release || echo dev))..."
  if [ "$RELEASE_MODE" -eq 1 ]; then cargo run -- build -r || die "cargo build failed"
  else cargo run -- build || die "cargo build failed"; fi

  info "Exporting Godot Linux client..."
  cargo run -- export --target linux || die "godot export failed"
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

mkdir -p "$OUTPUT_DIR" "$REPORT_DIR"
ENTITY_DIR="$OUTPUT_DIR/_entities"
mkdir -p "$ENTITY_DIR"

# ---------------------------------------------------------------------------
# Step 1: fetch all entity manifests up-front. Record fetch failures.
# ---------------------------------------------------------------------------
FETCH_LOG="$REPORT_DIR/fetch.log"
: > "$FETCH_LOG"

info "Fetching ${#CIDS[@]} entity manifest(s) from $CONTENT_URL ..."
declare -a FETCHED_CIDS=()           # CIDs whose manifest we successfully got
declare -A CID_STATUS=()             # cid -> status string (final report)

i=0
for cid in "${CIDS[@]}"; do
  i=$((i + 1))
  dest="$ENTITY_DIR/$cid.json"
  if [ -s "$dest" ]; then
    FETCHED_CIDS+=("$cid")
    continue
  fi
  url="${CONTENT_URL%/}/contents/$cid"
  if curl -fsSL --retry 2 --max-time 30 -H 'Accept: application/json' "$url" -o "$dest" 2>>"$FETCH_LOG"; then
    if [ -s "$dest" ]; then
      FETCHED_CIDS+=("$cid")
    else
      CID_STATUS[$cid]="fetch-empty"
      rm -f "$dest"
    fi
  else
    CID_STATUS[$cid]="fetch-fail"
    rm -f "$dest"
  fi
  if [ $((i % 50)) -eq 0 ]; then info "  fetched $i/${#CIDS[@]}"; fi
done
ok "Fetched ${#FETCHED_CIDS[@]}/${#CIDS[@]} manifests."

# ---------------------------------------------------------------------------
# Step 2: batch + run.
# ---------------------------------------------------------------------------
case "$RENDER_MODE" in
  compatibility) RENDER_ARGS=(--rendering-method gl_compatibility --rendering-driver opengl3) ;;
  mobile)        RENDER_ARGS=(--rendering-method mobile --rendering-driver vulkan) ;;
  *) die "unknown render mode: $RENDER_MODE" ;;
esac

run_batch() {
  local batch_idx="$1"; shift
  local -a batch=("$@")
  local payload_path="$REPORT_DIR/payload.batch-${batch_idx}.json"
  local stdout_log="$REPORT_DIR/render.batch-${batch_idx}.log"

  CONTENT_URL="$CONTENT_URL" \
  ENTITY_DIR="$ENTITY_DIR" \
  CIDS_JSON="$(printf '%s\n' "${batch[@]}" | python3 -c 'import json,sys; print(json.dumps([l.strip() for l in sys.stdin if l.strip()]))')" \
  OUTPUT_DIR="$OUTPUT_DIR" \
  BODY_W="$BODY_W" BODY_H="$BODY_H" FACE_W="$FACE_W" FACE_H="$FACE_H" \
  EMIT_FACE="$EMIT_FACE" \
  STATUS_FILE="$REPORT_DIR/payload-status.batch-${batch_idx}.csv" \
  python3 - "$payload_path" <<'PY'
import json, os, sys, csv

out_path  = sys.argv[1]
cids      = json.loads(os.environ["CIDS_JSON"])
edir      = os.environ["ENTITY_DIR"]
content   = os.environ["CONTENT_URL"]
out_dir   = os.environ["OUTPUT_DIR"]
body_w    = int(os.environ["BODY_W"])
body_h    = int(os.environ["BODY_H"])
face_w    = int(os.environ["FACE_W"])
face_h    = int(os.environ["FACE_H"])
emit_face = os.environ["EMIT_FACE"] == "1"
status_f  = os.environ["STATUS_FILE"]

payload = []
status = []  # (cid, "ok" | reason)
for cid in cids:
    p = os.path.join(edir, f"{cid}.json")
    try:
        with open(p) as f:
            data = json.load(f)
    except Exception as e:
        status.append((cid, f"manifest-parse-fail:{type(e).__name__}"))
        continue

    if data.get("type") and data["type"] != "profile":
        status.append((cid, f"not-a-profile:{data.get('type')}"))
        continue

    md = data.get("metadata") or {}
    avatars = md.get("avatars") or []
    if not avatars:
        status.append((cid, "no-avatars-in-metadata"))
        continue
    avatar = (avatars[0] or {}).get("avatar")
    if not avatar:
        status.append((cid, "missing-avatar-field"))
        continue

    entry = {
        "entity": cid,
        "destPath": os.path.join(out_dir, f"{cid}.png"),
        "width": body_w,
        "height": body_h,
        "avatar": avatar,
    }
    if emit_face:
        entry["faceDestPath"] = os.path.join(out_dir, f"{cid}_face.png")
        entry["faceWidth"]    = face_w
        entry["faceHeight"]   = face_h
    payload.append(entry)
    status.append((cid, "ok"))

with open(out_path, "w") as f:
    json.dump({"baseUrl": content, "payload": payload}, f, indent=2)

with open(status_f, "w", newline="") as f:
    w = csv.writer(f)
    w.writerow(["cid", "payload_status"])
    w.writerows(status)

print(f"payload entries: {len(payload)} / {len(cids)}")
PY

  if [ ! -s "$payload_path" ]; then
    warn "batch $batch_idx: payload empty, skipping renderer"
    return 0
  fi

  local run_args=("${RENDER_ARGS[@]}" --avatar-renderer --avatars "$payload_path")
  [ "$HEADLESS" -eq 1 ] && run_args=(--headless "${run_args[@]}")

  info "batch $batch_idx: running renderer (${#batch[@]} cids)"
  if "$EXPORT_BIN" "${run_args[@]}" >"$stdout_log" 2>&1; then
    ok   "batch $batch_idx: renderer ok"
  else
    warn "batch $batch_idx: renderer exited non-zero (see $stdout_log)"
  fi
}

total=${#FETCHED_CIDS[@]}
batch_idx=0
for ((start=0; start<total; start+=BATCH_SIZE)); do
  batch_idx=$((batch_idx + 1))
  end=$((start + BATCH_SIZE))
  [ "$end" -gt "$total" ] && end=$total
  batch=("${FETCHED_CIDS[@]:start:end-start}")
  run_batch "$batch_idx" "${batch[@]}"
done

# ---------------------------------------------------------------------------
# Step 3: classify outputs and write report.
# ---------------------------------------------------------------------------
info "Classifying outputs..."

CIDS_INPUT_FILE="$REPORT_DIR/_cids-input.txt"
printf '%s\n' "${CIDS[@]}" > "$CIDS_INPUT_FILE"

PRECLASSIFY_FILE="$REPORT_DIR/_preclassify.tsv"
: > "$PRECLASSIFY_FILE"
for cid in "${!CID_STATUS[@]}"; do
  printf '%s\t%s\n' "$cid" "${CID_STATUS[$cid]}" >> "$PRECLASSIFY_FILE"
done

CIDS_INPUT_FILE="$CIDS_INPUT_FILE" \
PRECLASSIFY_FILE="$PRECLASSIFY_FILE" \
REPORT_DIR="$REPORT_DIR" \
OUTPUT_DIR="$OUTPUT_DIR" \
EMIT_FACE="$EMIT_FACE" \
python3 - <<'PY'
import csv, glob, json, os, struct, sys

with open(os.environ["CIDS_INPUT_FILE"]) as f:
    cids = [l.strip() for l in f if l.strip()]
preclassify = {}
with open(os.environ["PRECLASSIFY_FILE"]) as f:
    for line in f:
        line = line.rstrip("\n")
        if not line: continue
        k, _, v = line.partition("\t")
        preclassify[k] = v
report_dir  = os.environ["REPORT_DIR"]
out_dir     = os.environ["OUTPUT_DIR"]
emit_face   = os.environ["EMIT_FACE"] == "1"

# Merge per-batch payload status files
payload_status = {}
for csv_path in sorted(glob.glob(os.path.join(report_dir, "payload-status.batch-*.csv"))):
    with open(csv_path) as f:
        next(f, None)
        for row in csv.reader(f):
            if not row: continue
            payload_status[row[0]] = row[1]

# Heuristic: a successfully-rendered avatar PNG at 256x512 is ~tens of KB.
# A blank/transparent PNG at the same size is < ~3KB. Tune threshold per
# image dims if you change them.
BLANK_BYTES_THRESHOLD = 3000

def png_dims(path):
    try:
        with open(path, "rb") as f:
            sig = f.read(8)
            if sig != b"\x89PNG\r\n\x1a\n":
                return None
            f.read(4)  # length of IHDR
            if f.read(4) != b"IHDR":
                return None
            w, h = struct.unpack(">II", f.read(8))
            return (w, h)
    except Exception:
        return None

def classify_png(path):
    if not os.path.exists(path):
        return ("render-missing", 0)
    sz = os.path.getsize(path)
    if sz == 0:
        return ("render-empty", 0)
    if png_dims(path) is None:
        return ("render-corrupt", sz)
    if sz < BLANK_BYTES_THRESHOLD:
        return ("render-blank", sz)
    return ("ok", sz)

rows = []
counts = {}
for cid in cids:
    pre = preclassify.get(cid)
    if pre:
        rows.append((cid, pre, "", 0, "", 0)); counts[pre] = counts.get(pre, 0) + 1
        continue
    ps = payload_status.get(cid)
    if ps and ps != "ok":
        rows.append((cid, ps, "", 0, "", 0)); counts[ps] = counts.get(ps, 0) + 1
        continue

    body_path = os.path.join(out_dir, f"{cid}.png")
    body_status, body_sz = classify_png(body_path)

    face_status, face_sz = "", 0
    if emit_face:
        face_path = os.path.join(out_dir, f"{cid}_face.png")
        face_status, face_sz = classify_png(face_path)

    final = body_status
    if final == "ok" and emit_face and face_status not in ("ok", ""):
        final = "face-" + face_status
    rows.append((cid, final, body_status, body_sz, face_status, face_sz))
    counts[final] = counts.get(final, 0) + 1

report_csv = os.path.join(report_dir, "report.csv")
with open(report_csv, "w", newline="") as f:
    w = csv.writer(f)
    w.writerow(["cid", "status", "body_status", "body_bytes", "face_status", "face_bytes"])
    w.writerows(rows)

summary = os.path.join(report_dir, "summary.txt")
with open(summary, "w") as f:
    f.write(f"total: {len(rows)}\n")
    for k in sorted(counts, key=lambda k: -counts[k]):
        f.write(f"  {counts[k]:>6}  {k}\n")

print(open(summary).read())
print(f"report: {report_csv}")
PY

ok "Done. Report dir: $REPORT_DIR"
