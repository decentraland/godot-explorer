#!/usr/bin/env python3
"""SceneProfiler report — parse [SCENEPROF] lines and show per-component tick cost.

The Rust scene runner (lib/src/scene_runner/update_scene.rs), when per-tick
profiling is enabled (SceneManager.set_scene_tick_profiling(true), flipped on by
the LoadingProfiler autoload), emits ONE line per completed tick per scene:

    [SCENEPROF] scene=<id> tick=<n> total_us=<us> n=<c> State=us State=us ...

where each `State=us` is the CPU microseconds that component system (one
SceneUpdateState) spent during that tick, accumulated across however many frames
the tick's processing spanned. This groups them by scene and by component so you
can see where per-tick scene CPU actually goes and how it changes over ticks.

Usage:
    # live capture on Android (both profilers share one grep):
    adb logcat -c
    adb logcat -v threadtime | grep --line-buffered -E "LOADPROF|SCENEPROF" | tee run.log
    # ...then:
    python3 scripts/sceneprof_report.py run.log
    python3 scripts/sceneprof_report.py run.log --scene 0     # one scene
    python3 scripts/sceneprof_report.py run.log --json out.json
"""

import sys
import json
from collections import defaultdict, OrderedDict

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


# Fields that are metadata, not component timings.
META = {"scene", "tick", "total_us", "n"}


def parse(lines):
    # A single tick can emit MULTIPLE partial [SCENEPROF] lines (its component
    # pass completes in segments — e.g. one line for most components, another
    # for ComputeCrdtState). We SUM those partials per (scene, tick). Sentry
    # duplicates the exact same line, so we drop byte-identical repeats of a
    # payload we've already folded into that tick.
    #   acc[(scene, tick)] = {"components": {name: us}, "seen": set(payloads)}
    acc = {}
    for raw in lines:
        if SCENEPROF not in raw:
            continue
        if "sentry" in raw.lower():
            continue
        # trailing chars from the tracing call site (e.g. " (src/...rs:56)") and
        # the Sentry breadcrumb's `",` suffix are stripped before parsing.
        payload = raw.split(SCENEPROF, 1)[1].strip()
        payload = payload.split(" (src/", 1)[0].rstrip('",').strip()
        kv = parse_kv(payload)
        scene = to_int(kv.get("scene"))
        tick = to_int(kv.get("tick"))
        if scene is None or tick is None:
            continue
        key = (scene, tick)
        bucket = acc.setdefault(key, {"components": defaultdict(int), "seen": set()})
        if payload in bucket["seen"]:
            continue  # exact-duplicate partial (Sentry double) — already folded
        bucket["seen"].add(payload)
        for k, v in kv.items():
            if k in META:
                continue
            us = to_int(v)
            if us is not None:
                bucket["components"][k] += us

    ticks = defaultdict(list)
    for (scene, tick), bucket in acc.items():
        comps = dict(bucket["components"])
        ticks[scene].append(
            {"tick": tick, "total_us": sum(comps.values()), "components": comps}
        )
    return ticks


def pct(sorted_vals, p):
    if not sorted_vals:
        return 0
    return sorted_vals[min(len(sorted_vals) - 1, int(len(sorted_vals) * p))]


def stats(vals):
    v = sorted(vals)
    if not v:
        return None
    n = len(v)
    return {
        "n": n,
        "sum": sum(v),
        "avg": sum(v) / n,
        "p50": pct(v, 0.50),
        "p95": pct(v, 0.95),
        "max": v[-1],
    }


def fmt_us(us):
    if us is None:
        return "-"
    if us >= 1000:
        return f"{us / 1000:.1f}ms"
    return f"{us:.0f}us"


def report_scene(scene, scene_ticks):
    scene_ticks.sort(key=lambda t: t["tick"])
    n_ticks = len(scene_ticks)
    span = (scene_ticks[0]["tick"], scene_ticks[-1]["tick"]) if scene_ticks else (0, 0)

    # per-component aggregate across all ticks
    per_comp = defaultdict(list)
    total_all = 0
    for t in scene_ticks:
        total_all += t["total_us"]
        for name, us in t["components"].items():
            per_comp[name].append(us)

    print("\n" + "=" * 78)
    print(
        f"SCENE {scene}  —  {n_ticks} ticks (#{span[0]}..#{span[1]})  "
        f"total {fmt_us(total_all)} of component CPU"
    )
    print("=" * 78)
    print(
        f"  {'component':<24} {'ticks':>6} {'sum':>10} {'avg/tick':>10} "
        f"{'p50':>9} {'p95':>9} {'max':>9} {'%':>6}"
    )
    rows = []
    for name, vals in per_comp.items():
        s = stats(vals)
        rows.append((name, s))
    rows.sort(key=lambda r: r[1]["sum"], reverse=True)
    for name, s in rows:
        share = (100.0 * s["sum"] / total_all) if total_all else 0.0
        print(
            f"  {name:<24} {s['n']:>6} {fmt_us(s['sum']):>10} {fmt_us(s['avg']):>10} "
            f"{fmt_us(s['p50']):>9} {fmt_us(s['p95']):>9} {fmt_us(s['max']):>9} {share:>5.1f}%"
        )

    # heaviest single ticks
    worst = sorted(scene_ticks, key=lambda t: t["total_us"], reverse=True)[:8]
    print(f"\n  heaviest ticks:")
    for t in worst:
        top = sorted(t["components"].items(), key=lambda kv: kv[1], reverse=True)[:3]
        top_s = ", ".join(f"{n} {fmt_us(u)}" for n, u in top)
        print(f"    tick #{t['tick']:<5} {fmt_us(t['total_us']):>9}   ({top_s})")


def main():
    args = [a for a in sys.argv[1:]]
    only_scene = None
    json_out = None
    path = None
    i = 0
    while i < len(args):
        if args[i] == "--scene":
            only_scene = to_int(args[i + 1])
            i += 2
        elif args[i] == "--json":
            json_out = args[i + 1]
            i += 2
        else:
            path = args[i]
            i += 1

    if path and path not in ("-", "/dev/stdin"):
        with open(path, "r", errors="replace") as f:
            lines = f.readlines()
    else:
        lines = sys.stdin.readlines()

    ticks = parse(lines)
    if not ticks:
        print("No [SCENEPROF] lines found. Is per-tick profiling enabled and logcat captured?")
        return

    total_ticks = sum(len(v) for v in ticks.values())
    print(f"Parsed {total_ticks} tick-records across {len(ticks)} scene(s).")

    for scene in sorted(ticks.keys()):
        if only_scene is not None and scene != only_scene:
            continue
        report_scene(scene, ticks[scene])

    if json_out:
        doc = {
            str(scene): sorted(t, key=lambda x: x["tick"]) for scene, t in ticks.items()
        }
        with open(json_out, "w") as f:
            json.dump(doc, f, separators=(",", ":"))
        print(f"\nWrote {json_out}")


if __name__ == "__main__":
    main()
