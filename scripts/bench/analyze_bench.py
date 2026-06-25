#!/usr/bin/env python3
"""Aggregate a gp-benchmark JSON into Visual-Profiler-style per-frame breakdown.

Usage:
  scripts/bench/analyze_bench.py <bench.json>...

Prints summary table per file. Multi-file mode shows side-by-side delta vs
the first file (treated as baseline).
"""
import json
import sys
from collections import defaultdict
from statistics import mean, median


def stats(xs):
    if not xs:
        return None
    xs = sorted(xs)
    n = len(xs)
    return {
        "n": n,
        "mean": sum(xs) / n,
        "p50": xs[n // 2],
        "p90": xs[(n * 9) // 10],
        "min": xs[0],
        "max": xs[-1],
    }


def per_viewport(samples):
    """Aggregate per-viewport CPU/GPU ms over all frames."""
    by_path = defaultdict(lambda: {"cpu": [], "gpu": []})
    for s in samples:
        for path, t in (s.get("per_viewport_render_time") or {}).items():
            by_path[path]["cpu"].append(t.get("cpu_ms", 0.0))
            by_path[path]["gpu"].append(t.get("gpu_ms", 0.0))
    out = {}
    for path, t in by_path.items():
        out[path] = {"cpu_ms_mean": mean(t["cpu"]), "gpu_ms_mean": mean(t["gpu"])}
    return out


def report(path):
    with open(path) as f:
        d = json.load(f)
    samples = d.get("samples", [])
    if not samples:
        print(f"{path}: no samples")
        return None
    fps = stats([s.get("fps", 0) for s in samples])
    proc = stats([s.get("frame_time_process_ms", 0) for s in samples])
    phys = stats([s.get("frame_time_physics_ms", 0) for s in samples])
    gpu_main = stats([s.get("render_gpu_ms", 0) for s in samples])
    cpu_main = stats([s.get("render_cpu_ms", 0) for s in samples])
    draws = mean([s.get("visible_draws", 0) for s in samples])
    prim = mean([s.get("visible_prim", 0) for s in samples])
    pv = per_viewport(samples)

    tag = d.get("tag", path.split("/")[-1])
    print(f"\n=== {tag} ({len(samples)} samples) ===")
    print(f"  fps        : mean={fps['mean']:>5.2f}  p50={fps['p50']:>3.0f}  p90={fps['p90']:>3.0f}  max={fps['max']:>3.0f}")
    print(f"  proc_ms    : mean={proc['mean']:>5.1f}  p50={proc['p50']:>5.1f}  p90={proc['p90']:>5.1f}  max={proc['max']:>5.1f}")
    print(f"  phys_ms    : mean={phys['mean']:>5.1f}  p50={phys['p50']:>5.1f}")
    print(f"  render_main: cpu_mean={cpu_main['mean']:>5.2f}  gpu_mean={gpu_main['mean']:>5.2f}")
    print(f"  visible    : draws={draws:>6.0f}  prim={prim:>9.0f}")
    if pv:
        print(f"  per-viewport (cpu_ms / gpu_ms means):")
        # Sort by gpu_ms desc — biggest cost first
        sorted_pv = sorted(pv.items(), key=lambda kv: -kv[1]["gpu_ms_mean"])
        for path, t in sorted_pv:
            short = path.split("/")[-1] if len(path) > 50 else path
            print(f"    [{short:>40}]  cpu={t['cpu_ms_mean']:>5.2f}  gpu={t['gpu_ms_mean']:>5.2f}")
    return {"tag": tag, "fps_mean": fps["mean"], "proc_mean": proc["mean"], "draws": draws, "prim": prim, "per_viewport": pv}


def main():
    if len(sys.argv) < 2:
        print(__doc__, file=sys.stderr)
        sys.exit(2)

    rows = []
    for p in sys.argv[1:]:
        r = report(p)
        if r:
            rows.append(r)

    if len(rows) > 1:
        baseline = rows[0]
        print(f"\n=== DELTA vs {baseline['tag']} ===")
        print(f"{'tag':<32}  {'fps_mean':>8}  {'Δfps':>7}  {'proc_ms':>8}  {'Δproc':>7}  {'draws':>7}  {'Δdraws':>8}")
        for r in rows:
            d_fps = r["fps_mean"] - baseline["fps_mean"]
            d_proc = r["proc_mean"] - baseline["proc_mean"]
            d_draws = r["draws"] - baseline["draws"]
            print(f"{r['tag']:<32}  {r['fps_mean']:>8.2f}  {d_fps:>+7.2f}  {r['proc_mean']:>8.2f}  {d_proc:>+7.2f}  {r['draws']:>7.0f}  {d_draws:>+8.0f}")


if __name__ == "__main__":
    main()
