#!/usr/bin/env python3
"""LoadingProfiler → JSON exporter for deep analysis.

Parses a raw `adb logcat` capture (or an already-filtered file) and emits a single
structured JSON with:
  - meta            : counts + run span
  - episodes[]      : loading-screen journeys with milestone timings (ms + frame)
  - gltf_instances[]: one record per GLTF load, paired start→downloaded→
                      instantiated→added by (scene_id, entity), with
                      download_ms / load_ms / gpu_ms / total_ms
  - gltf_by_hash{}  : per-asset aggregates — instance count + stats for each stage
  - downloads[]     : aggregate ContentProvider download waves

Sentry breadcrumb duplicate lines are dropped automatically.

Usage:
    python3 scripts/loadprof_json.py raw_logcat.log            # -> raw_logcat.json
    python3 scripts/loadprof_json.py raw_logcat.log out.json   # explicit output
    python3 scripts/loadprof_json.py raw_logcat.log - | jq ... # JSON to stdout

Also prints top-N slowest / most-instantiated tables to stderr for a quick look.
"""

import sys
import json
from collections import OrderedDict, defaultdict, deque

LOADPROF = "[LOADPROF]"
LOADPROF_SUM = "[LOADPROF-SUM]"

INT_FIELDS = {"t", "f", "pf", "dt", "gap", "scene_id", "entity", "session",
              "expected", "ready", "total", "pct", "loading", "loaded", "pending",
              "loadable", "frames", "pframes"}
FLOAT_FIELDS = {"ms", "gpu_ms", "mbps"}


def parse_kv(rest):
    out = OrderedDict()
    for tok in rest.split():
        if "=" in tok:
            k, v = tok.split("=", 1)
            out[k] = v
    return out


def coerce(kv):
    d = {}
    for k, v in kv.items():
        if k in INT_FIELDS:
            try:
                d[k] = int(v)
            except ValueError:
                d[k] = v
        elif k in FLOAT_FIELDS:
            try:
                d[k] = float(v)
            except ValueError:
                d[k] = v
        else:
            d[k] = v
    return d


def parse(lines):
    events = []
    summaries = []
    for raw in lines:
        if "sentry" in raw.lower():
            continue  # drop Sentry breadcrumb duplicates
        if LOADPROF_SUM in raw:
            summaries.append(coerce(parse_kv(raw.split(LOADPROF_SUM, 1)[1].strip())))
            continue
        if LOADPROF not in raw:
            continue
        kv = coerce(parse_kv(raw.split(LOADPROF, 1)[1].strip()))
        if "ev" in kv:
            events.append(kv)
    return events, summaries


def stats(values):
    vals = sorted(v for v in values if v is not None)
    if not vals:
        return None
    n = len(vals)
    return {
        "n": n,
        "min": round(vals[0], 2),
        "avg": round(sum(vals) / n, 2),
        "p50": round(vals[n // 2], 2),
        "p95": round(vals[min(n - 1, int(n * 0.95))], 2),
        "max": round(vals[-1], 2),
    }


def build_gltf_instances(events):
    """Pair gltf stage events into per-load instance records.

    Key = (scene_id, entity) when entity is present (new data), else hash (old
    data). Stages attach FIFO to the oldest open instance for that key.
    """
    open_by_key = defaultdict(deque)  # key -> deque of open instance dicts
    instances = []

    def key_of(e):
        if "entity" in e and "scene_id" in e:
            return (e.get("scene_id"), e.get("entity"), e.get("hash"))
        return ("hash", e.get("hash"))

    def oldest_missing(key, field):
        for inst in open_by_key[key]:
            if field not in inst:
                return inst
        return None

    for e in events:
        ev = e["ev"]
        if not ev.startswith("asset.gltf_"):
            continue
        key = key_of(e)
        if ev == "asset.gltf_start":
            inst = {
                "scene_id": e.get("scene_id"),
                "entity": e.get("entity"),
                "hash": e.get("hash"),
                "src": e.get("src"),
                "ep": e.get("ep"),
                "t_start": e.get("t"),
                "f_start": e.get("f"),
            }
            open_by_key[key].append(inst)
            instances.append(inst)
        elif ev == "asset.gltf_download_begin":
            inst = oldest_missing(key, "t_download_begin")
            if inst is not None:
                inst["t_download_begin"] = e.get("t")
        elif ev == "asset.gltf_downloaded":
            inst = oldest_missing(key, "t_downloaded")
            if inst is not None:
                inst["t_downloaded"] = e.get("t")
                inst["opt"] = e.get("opt")
        elif ev == "asset.gltf_instantiated":
            inst = oldest_missing(key, "t_instantiated")
            if inst is not None:
                inst["t_instantiated"] = e.get("t")
                inst["load_ms"] = e.get("ms")
        elif ev == "asset.gltf_added":
            inst = oldest_missing(key, "t_added")
            if inst is not None:
                inst["t_added"] = e.get("t")
                inst["gpu_ms"] = e.get("gpu_ms")
                inst["opt"] = e.get("opt", inst.get("opt"))
                try:
                    open_by_key[key].remove(inst)
                except ValueError:
                    pass
        elif ev == "asset.gltf_error":
            inst = oldest_missing(key, "t_added")
            if inst is not None:
                inst["error"] = e.get("reason", "error")
                inst["t_added"] = e.get("t")
                try:
                    open_by_key[key].remove(inst)
                except ValueError:
                    pass

    # derive per-instance durations
    for i in instances:
        ts, td = i.get("t_start"), i.get("t_downloaded")
        tb = i.get("t_download_begin")
        ti, ta = i.get("t_instantiated"), i.get("t_added")
        i["queue_wait_ms"] = (tb - ts) if (ts is not None and tb is not None) else None
        if td is not None and tb is not None:
            i["download_ms"] = td - tb  # actual fetch, queue wait excluded
        elif td is not None and ts is not None:
            i["download_ms"] = td - ts  # fallback (no download_begin mark): queue+fetch
        else:
            i["download_ms"] = None
        i["add_ms"] = (ta - ti) if (ti is not None and ta is not None) else None
        i["total_ms"] = (ta - ts) if (ts is not None and ta is not None) else None
        # load_ms and gpu_ms are direct fields (may be absent in old captures)
    return instances


def aggregate_by_hash(instances):
    by = defaultdict(list)
    for i in instances:
        by[i.get("hash")].append(i)
    out = {}
    for h, insts in by.items():
        out[h] = {
            "instances": len(insts),
            "errors": sum(1 for i in insts if i.get("error")),
            "queue_wait_ms": stats([i.get("queue_wait_ms") for i in insts]),
            "download_ms": stats([i.get("download_ms") for i in insts]),
            "load_ms": stats([i.get("load_ms") for i in insts]),
            "gpu_ms": stats([i.get("gpu_ms") for i in insts if i.get("gpu_ms", -1) and i.get("gpu_ms", -1) >= 0]),
            "total_ms": stats([i.get("total_ms") for i in insts]),
            "src": next((i.get("src") for i in insts if i.get("src")), None),
        }
    return out


def build_episodes(events, summaries):
    eps = OrderedDict()
    for e in events:
        ep = e.get("ep")
        if ep in (None, "-"):
            continue
        d = eps.setdefault(ep, {"ep": ep, "milestones": {}, "events": 0})
        d["events"] += 1
        ev = e["ev"]
        if ev not in d["milestones"] and isinstance(e.get("dt"), int):
            d["milestones"][ev] = {"dt": e["dt"], "frame": e.get("f")}
    for s in summaries:
        ep = str(s.get("ep"))
        d = eps.setdefault(ep, {"ep": ep, "milestones": {}, "events": 0})
        d["summary"] = s
    return list(eps.values())


def top(instances, field, n=12):
    have = [i for i in instances if i.get(field) is not None]
    have.sort(key=lambda i: i[field], reverse=True)
    return have[:n]


def print_table(title, rows, field, out=sys.stderr):
    print(f"\n== {title} ==", file=out)
    print(f"  {'hash/src':<44} {'scene':>5} {'entity':>7} {field:>10}", file=out)
    for i in rows:
        label = (i.get("src") or i.get("hash") or "?")[:44]
        print(
            f"  {label:<44} {str(i.get('scene_id')):>5} "
            f"{str(i.get('entity')):>7} {str(i.get(field)):>10}",
            file=out,
        )


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)
    inp = sys.argv[1]
    out_path = sys.argv[2] if len(sys.argv) > 2 else inp.rsplit(".", 1)[0] + ".json"

    with open(inp, "r", errors="replace") as f:
        events, summaries = parse(f.readlines())

    instances = build_gltf_instances(events)
    by_hash = aggregate_by_hash(instances)
    episodes = build_episodes(events, summaries)
    downloads = [e for e in events if e["ev"].startswith("asset.download_")]

    ts = [e["t"] for e in events if isinstance(e.get("t"), int)]
    doc = {
        "meta": {
            "events": len(events),
            "episodes": len(episodes),
            "gltf_instances": len(instances),
            "distinct_gltf_hashes": len(by_hash),
            "t_first_ms": min(ts) if ts else None,
            "t_last_ms": max(ts) if ts else None,
            "has_gpu_data": any(i.get("gpu_ms", -1) is not None and i.get("gpu_ms", -1) >= 0 for i in instances),
            "has_load_data": any(i.get("load_ms") is not None for i in instances),
        },
        "episodes": episodes,
        "gltf_instances": instances,
        "gltf_by_hash": by_hash,
        "downloads": downloads,
        "events": events,
    }

    js = json.dumps(doc, indent=2)
    if out_path == "-":
        print(js)
    else:
        with open(out_path, "w") as f:
            f.write(js)
        print(f"Wrote {out_path}  ({len(js) // 1024} KB)", file=sys.stderr)

    # quick human summary to stderr
    m = doc["meta"]
    print(
        f"\nevents={m['events']} episodes={m['episodes']} "
        f"gltf_instances={m['gltf_instances']} distinct_hashes={m['distinct_gltf_hashes']} "
        f"gpu_data={m['has_gpu_data']} load_data={m['has_load_data']}",
        file=sys.stderr,
    )
    print_table("Slowest QUEUE WAIT (ms)", top(instances, "queue_wait_ms"), "queue_wait_ms")
    print_table("Slowest DOWNLOAD (ms)", top(instances, "download_ms"), "download_ms")
    if m["has_load_data"]:
        print_table("Slowest LOAD/instantiate (ms)", top(instances, "load_ms"), "load_ms")
    if m["has_gpu_data"]:
        print_table("Slowest GPU/post-add frame (ms)", top(instances, "gpu_ms"), "gpu_ms")
    # most instances per hash
    most = sorted(by_hash.items(), key=lambda kv: kv[1]["instances"], reverse=True)[:12]
    print("\n== Most INSTANCES per GLTF hash ==", file=sys.stderr)
    for h, agg in most:
        label = (agg.get("src") or h or "?")[:50]
        print(f"  {label:<50} x{agg['instances']}", file=sys.stderr)


if __name__ == "__main__":
    main()
