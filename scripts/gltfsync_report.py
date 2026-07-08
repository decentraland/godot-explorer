#!/usr/bin/env python3
"""GLTFSYNC report — settle where sync_gltf_loading_state's per-tick cost goes.

The Rust scene runner (lib/src/scene_runner/components/gltf_container.rs), when
per-tick profiling is on (same toggle as [SCENEPROF]), emits one line per call to
sync_gltf_loading_state:

    [GLTFSYNC] scene=<id> tick=<n> loading_len=<L> iterated=<I> lookup_us=<us> \
               deferred_calls=<c> deferred_us=<us> done=<bool>

where per call:
  loading_len    = entities in scene.gltf_loading at entry (the set it scans)
  iterated       = entities actually visited before the time budget cut it off
  lookup_us      = total us spent in ensure_node_3d + try_get_node_as("GltfContainer")
  deferred_calls = times async_deferred_add_child was actually invoked
                   (fires only when dcl_pending_node.is_some())
  deferred_us    = total us inside those async_deferred_add_child calls

Two questions this answers:
  1. Is async_deferred_add_child EVER called?  -> total deferred_calls.
     0 across the whole run == the legacy path is dead (dcl_pending_node never set).
  2. Where does the SyncGltfContainer per-tick cost go?  -> lookup_us per entity
     and lookup_us as a share of the full state time (cross-ref [SCENEPROF]).

Usage:
    python3 scripts/gltfsync_report.py run.log
    python3 scripts/gltfsync_report.py run.log --scene 0
"""

import sys
from collections import defaultdict, OrderedDict

GLTFSYNC = "[GLTFSYNC]"
SCENEPROF = "[SCENEPROF]"


def parse_kv(rest):
    out = OrderedDict()
    for tok in rest.split():
        if "=" in tok:
            k, v = tok.split("=", 1)
            out[k] = v
    return out


def to_int(v, default=None):
    try:
        return int(v)
    except (TypeError, ValueError):
        return default


def clean_payload(raw, marker):
    payload = raw.split(marker, 1)[1].strip()
    # drop the tracing call-site suffix " (src/...rs:NN)" and Sentry's `",`
    payload = payload.split(" (src/", 1)[0].rstrip('",').strip()
    return payload


def parse_gltfsync(lines):
    # A call emits ONE line; Sentry duplicates it byte-for-byte. Dedup exact
    # repeats per (scene, tick, payload). Unlike SCENEPROF a tick is not split
    # across partials here, but the SAME tick can legitimately produce two
    # DISTINCT records (caller at update_scene.rs:325 for tick<=3 AND the
    # SyncGltfContainer state at :463), so we key dedup on the full payload.
    seen = set()
    recs = []
    for raw in lines:
        if GLTFSYNC not in raw:
            continue
        payload = clean_payload(raw, GLTFSYNC)
        if payload in seen:
            continue
        seen.add(payload)
        kv = parse_kv(payload)
        rec = {
            "scene": to_int(kv.get("scene")),
            "tick": to_int(kv.get("tick")),
            "loading_len": to_int(kv.get("loading_len"), 0),
            "iterated": to_int(kv.get("iterated"), 0),
            "lookup_us": to_int(kv.get("lookup_us"), 0),
            "deferred_calls": to_int(kv.get("deferred_calls"), 0),
            "deferred_us": to_int(kv.get("deferred_us"), 0),
            "done": kv.get("done"),
        }
        if rec["scene"] is None or rec["tick"] is None:
            continue
        recs.append(rec)
    return recs


def parse_sceneprof_syncgltf(lines):
    # sum SyncGltfContainer us per (scene, tick), deduping Sentry doubles.
    acc = defaultdict(int)
    seen = defaultdict(set)
    for raw in lines:
        if SCENEPROF not in raw:
            continue
        if "sentry" in raw.lower():
            continue
        payload = clean_payload(raw, SCENEPROF)
        kv = parse_kv(payload)
        scene = to_int(kv.get("scene"))
        tick = to_int(kv.get("tick"))
        if scene is None or tick is None:
            continue
        key = (scene, tick)
        if payload in seen[key]:
            continue
        seen[key].add(payload)
        us = to_int(kv.get("SyncGltfContainer"))
        if us is not None:
            acc[key] += us
    return acc


def fmt_us(us):
    if us is None:
        return "-"
    if us >= 1_000_000:
        return f"{us / 1_000_000:.2f}s"
    if us >= 1000:
        return f"{us / 1000:.1f}ms"
    return f"{us:.0f}us"


def pct(sorted_vals, p):
    if not sorted_vals:
        return 0
    return sorted_vals[min(len(sorted_vals) - 1, int(len(sorted_vals) * p))]


def report_scene(scene, recs, syncgltf):
    recs.sort(key=lambda r: r["tick"])
    n = len(recs)
    span = (recs[0]["tick"], recs[-1]["tick"])
    sum_lookup = sum(r["lookup_us"] for r in recs)
    sum_iter = sum(r["iterated"] for r in recs)
    sum_defc = sum(r["deferred_calls"] for r in recs)
    sum_defus = sum(r["deferred_us"] for r in recs)
    max_len = max(r["loading_len"] for r in recs)
    max_iter = max(r["iterated"] for r in recs)
    recs_with_deferred = sum(1 for r in recs if r["deferred_calls"] > 0)

    lookups = sorted(r["lookup_us"] for r in recs)
    per_entity = (sum_lookup / sum_iter) if sum_iter else 0.0

    # cross-ref: total SyncGltfContainer state time for this scene from SCENEPROF
    sp_total = sum(us for (s, _t), us in syncgltf.items() if s == scene)

    print("\n" + "=" * 72)
    print(f"SCENE {scene}  —  {n} GLTFSYNC calls (ticks #{span[0]}..#{span[1]})")
    print("=" * 72)

    print("\n  Q1: is async_deferred_add_child EVER called?")
    print(f"      deferred_calls total ....... {sum_defc}")
    print(f"      calls with deferred>0 ...... {recs_with_deferred} of {n}")
    print(f"      deferred_us total .......... {fmt_us(sum_defus)}")
    verdict = (
        "NEVER called — legacy path is dead (dcl_pending_node never set)"
        if sum_defc == 0
        else f"CALLED {sum_defc}x — legacy path IS live"
    )
    print(f"      -> {verdict}")

    print("\n  Q2: where does the per-tick scan cost go?")
    print(f"      entities scanned (sum) ..... {sum_iter}")
    print(f"      max loading_len ............ {max_len}  (max iterated {max_iter})")
    print(f"      lookup_us total ............ {fmt_us(sum_lookup)}")
    print(f"        per entity (avg) ......... {fmt_us(per_entity)}")
    print(
        f"        lookup_us p50/p95/max/call {fmt_us(pct(lookups,0.50))} / "
        f"{fmt_us(pct(lookups,0.95))} / {fmt_us(lookups[-1])}"
    )
    if sp_total:
        share = 100.0 * sum_lookup / sp_total
        print(
            f"      SyncGltfContainer state time (SCENEPROF) ... {fmt_us(sp_total)}"
        )
        print(
            f"      -> lookup_us is {share:.1f}% of the state; "
            f"other work (state read/put/remove) {fmt_us(sp_total - sum_lookup - sum_defus)}"
        )
    else:
        print("      (no matching [SCENEPROF] SyncGltfContainer totals to cross-ref)")

    # heaviest calls by lookup_us
    worst = sorted(recs, key=lambda r: r["lookup_us"], reverse=True)[:6]
    print("\n  heaviest calls by lookup_us:")
    for r in worst:
        print(
            f"    tick #{r['tick']:<5} len={r['loading_len']:<5} iter={r['iterated']:<5} "
            f"lookup={fmt_us(r['lookup_us']):>8} deferred={r['deferred_calls']} done={r['done']}"
        )


def main():
    args = sys.argv[1:]
    only_scene = None
    path = None
    i = 0
    while i < len(args):
        if args[i] == "--scene":
            only_scene = to_int(args[i + 1])
            i += 2
        else:
            path = args[i]
            i += 1

    if path and path not in ("-", "/dev/stdin"):
        with open(path, "r", errors="replace") as f:
            lines = f.readlines()
    else:
        lines = sys.stdin.readlines()

    recs = parse_gltfsync(lines)
    if not recs:
        print("No [GLTFSYNC] lines found. Is per-tick profiling on and the .so rebuilt?")
        return
    syncgltf = parse_sceneprof_syncgltf(lines)

    by_scene = defaultdict(list)
    for r in recs:
        by_scene[r["scene"]].append(r)

    print(f"Parsed {len(recs)} GLTFSYNC calls across {len(by_scene)} scene(s).")
    grand_def = sum(r["deferred_calls"] for r in recs)
    print(
        f"GRAND deferred_calls across all scenes: {grand_def}  "
        f"({'async_deferred_add_child NEVER called' if grand_def == 0 else 'IS called'})"
    )

    for scene in sorted(by_scene.keys()):
        if only_scene is not None and scene != only_scene:
            continue
        report_scene(scene, by_scene[scene], syncgltf)


if __name__ == "__main__":
    main()
