#!/usr/bin/env python3
"""Pre-aggregate a loadprof run.json into a compact viz_data.json for the HTML report.

The full run.json is multi-MB (every GLTF instance + raw event stream). This reduces
it to the ~tens-of-KB of derived series an embedded, self-contained chart page needs:
histograms, a temporal stage heatmap, queue/in-flight concurrency over time, episode
milestones, and top-N tables.

    python3 scripts/loadprof_viz.py run2.json viz_data.json
"""

import sys
import json
from collections import defaultdict


def short(src, hsh):
    s = src or hsh or "?"
    parts = s.split("/")
    return "/".join(parts[-2:]) if len(parts) > 1 else s


def pct(sorted_vals, p):
    if not sorted_vals:
        return None
    return sorted_vals[min(len(sorted_vals) - 1, int(len(sorted_vals) * p))]


def summarize(vals):
    v = sorted(x for x in vals if isinstance(x, (int, float)) and x >= 0)
    if not v:
        return None
    return {
        "n": len(v),
        "sum": round(sum(v)),
        "avg": round(sum(v) / len(v), 1),
        "p50": round(pct(v, 0.50), 1),
        "p95": round(pct(v, 0.95), 1),
        "max": round(max(v), 1),
    }


def histogram(vals, edges):
    counts = [0] * (len(edges) - 1)
    for x in vals:
        if x is None or x < 0:
            continue
        for i in range(len(edges) - 1):
            if edges[i] <= x < edges[i + 1] or (i == len(edges) - 2 and x >= edges[-1]):
                counts[i] += 1
                break
    return counts


HIST_EDGES = {
    "queue_wait_ms": [0, 500, 1000, 2000, 5000, 10000, 20000, 30000, 40000, 50000],
    "download_ms": [0, 250, 500, 1000, 2000, 5000, 10000, 25000],
    "load_ms": [0, 1, 2, 5, 10, 25, 50, 100, 250, 500, 1500],
    "gpu_ms": [0, 50, 100, 250, 500, 1000, 2000, 3000],
}


def main():
    run = sys.argv[1] if len(sys.argv) > 1 else "run2.json"
    out = sys.argv[2] if len(sys.argv) > 2 else "viz_data.json"
    d = json.load(open(run))
    ins = d["gltf_instances"]

    # ---- episodes (milestones for the timeline) ----
    episodes = []
    for e in d["episodes"]:
        s = e.get("summary", {})
        ms = {k: v["dt"] for k, v in e.get("milestones", {}).items()}
        episodes.append({
            "ep": e["ep"],
            "when": s.get("when", "-"),
            "realm": s.get("realm", "-"),
            "total": s.get("total"),
            "frames": s.get("frames"),
            "pframes": s.get("pframes"),
            "ms": ms,
        })

    # ---- per-scene stage summaries ----
    by_scene = defaultdict(list)
    for i in ins:
        by_scene[i.get("scene_id")].append(i)
    stage_keys = ["queue_wait_ms", "download_ms", "load_ms", "gpu_ms", "total_ms"]
    scenes = {}
    for sid, items in by_scene.items():
        scenes[sid] = {
            "count": len(items),
            "stages": {k: summarize([i.get(k) for i in items]) for k in stage_keys},
        }

    # ---- histograms (all instances) ----
    histograms = {}
    for k, edges in HIST_EDGES.items():
        histograms[k] = {"edges": edges, "counts": histogram([i.get(k) for i in ins], edges)}

    # ---- Genesis (scene 0) temporal series ----
    g = [i for i in ins if i.get("scene_id") == 0 and isinstance(i.get("t_start"), int)]
    heatmap = None
    concurrency = None
    if g:
        t0 = min(i["t_start"] for i in g)
        t_end = max((i.get("t_added") or i["t_start"]) for i in g)
        span = max(1, t_end - t0)

        # stage heatmap: 2s buckets
        bucket_ms = 2000
        nb = span // bucket_ms + 1
        stages = ["t_start", "t_download_begin", "t_downloaded", "t_instantiated", "t_added"]
        stage_labels = ["start", "queued→dl", "downloaded", "instantiated", "added"]
        hm = [[0] * nb for _ in stages]
        for i in g:
            for si, sk in enumerate(stages):
                t = i.get(sk)
                if isinstance(t, int):
                    b = (t - t0) // bucket_ms
                    if 0 <= b < nb:
                        hm[si][b] += 1
        heatmap = {"t0": t0, "bucket_s": bucket_ms / 1000, "n_buckets": nb,
                   "stages": stage_labels, "data": hm}

        # concurrency: 1s buckets — queued vs in-flight vs cumulative done
        cb = 1000
        ncb = span // cb + 1
        conc = []
        for b in range(ncb):
            tb = t0 + b * cb
            queued = inflight = done = 0
            for i in g:
                ts = i.get("t_start")
                tdb = i.get("t_download_begin")
                ta = i.get("t_added")
                if ta is not None and ta <= tb:
                    done += 1
                elif tdb is not None and tdb <= tb and (ta is None or tb < ta):
                    inflight += 1
                elif ts is not None and ts <= tb and (tdb is None or tb < tdb):
                    queued += 1
            conc.append({"t": round(b * cb / 1000, 1), "queued": queued,
                         "inflight": inflight, "done": done})
        concurrency = {"total": len(g), "series": conc}

    # ---- top-N tables ----
    def topn(field, n=15):
        have = [i for i in ins if isinstance(i.get(field), (int, float)) and i.get(field) >= 0]
        have.sort(key=lambda i: i[field], reverse=True)
        return [{"src": short(i.get("src"), i.get("hash")), "scene": i.get("scene_id"),
                 "entity": i.get("entity"), "val": round(i[field], 1)} for i in have[:n]]

    top = {k.replace("_ms", ""): topn(k) for k in ["queue_wait_ms", "download_ms", "load_ms", "gpu_ms"]}

    # ---- instances per hash ----
    by_hash = defaultdict(list)
    for i in ins:
        by_hash[i.get("hash")].append(i)
    inst_rows = []
    for h, items in by_hash.items():
        def avg(k):
            v = [x.get(k) for x in items if isinstance(x.get(k), (int, float)) and x.get(k) >= 0]
            return round(sum(v) / len(v), 1) if v else None
        inst_rows.append({
            "src": short(items[0].get("src"), h),
            "count": len(items),
            "queue_avg": avg("queue_wait_ms"),
            "download_avg": avg("download_ms"),
            "load_avg": avg("load_ms"),
            "gpu_avg": avg("gpu_ms"),
        })
    inst_rows.sort(key=lambda r: r["count"], reverse=True)
    instances = inst_rows[:20]

    doc = {
        "meta": {
            "gltf_instances": len(ins),
            "distinct_hashes": len(by_hash),
            "episodes": len(episodes),
        },
        "episodes": episodes,
        "scenes": scenes,
        "histograms": histograms,
        "heatmap_genesis": heatmap,
        "concurrency_genesis": concurrency,
        "top": top,
        "instances": instances,
    }
    js = json.dumps(doc, separators=(",", ":"))
    with open(out, "w") as f:
        f.write(js)
    print(f"Wrote {out}  ({len(js) // 1024} KB)")


if __name__ == "__main__":
    main()
