#!/usr/bin/env python3
"""Blame main-thread hiccups from a Perfetto system trace (issue #3015).

Input: a trace recorded by scripts/bench/profile_android_hiccups.sh (scheduler
events for every thread + the "godot" track_event zones the engine emits when
built with `scons profiler=perfetto`, including the DclProfiler zones from Rust
and GDScript).

For every main-thread frame longer than --threshold ms the report says:
  * how the main thread spent the frame: Running / Runnable (preempted) /
    Sleeping / Uninterruptible;
  * the engine/explorer zones that covered most of the frame on the main thread
    (what it was doing);
  * for the longest blocked stretches, which thread woke the main thread and
    what zone that thread was in at that moment (what it was waiting on);
  * while Runnable, which threads were running on the main thread's last CPU
    (who preempted it).
It ends with totals across all hiccup frames: top zones, top wakers, top
preemptors.

Usage:
  ~/.venvs/perfetto/bin/python scripts/bench/hiccup_report.py TRACE
      [--threshold 33] [--top 20] [--frames 30] [--after-marker LoadingScreen::hidden]
      [--process org.decentraland.godotexplorer] [--json OUT]

`--after-marker` restricts the frame window to after the first zone with that
name (the explorer marks LoadingScreen::hidden and GPBench::phase).
"""
import argparse
import json
import sys
from collections import Counter, defaultdict

try:
    from perfetto.trace_processor import TraceProcessor
except ImportError:
    sys.exit("pip install perfetto (the script expects ~/.venvs/perfetto)")

FRAME_SLICE_NAMES = ("Main::iteration", "OS_Android::main_loop_iterate")
STATE_LABEL = {
    "Running": "running",
    "R": "runnable",
    "R+": "runnable",
    "S": "sleeping",
    "D": "uninterruptible",
    "DK": "uninterruptible",
    "I": "idle",
}


def rows(tp, sql):
    return [r.__dict__ for r in tp.query(sql)]


def find_process(tp, name):
    procs = rows(tp, f"""
        select upid, pid, name from process
        where name like '%{name}%' order by pid desc limit 1""")
    if not procs:
        sys.exit(f"process matching '{name}' not found in trace")
    return procs[0]


def main_thread(tp, upid):
    """The thread running the engine loop: on Android that is Godot's VkThread, not the
    process main thread, so pick the thread that owns the frame zones."""
    names = ",".join(f"'{n}'" for n in FRAME_SLICE_NAMES)
    t = rows(tp, f"""
        select t.utid, t.tid, t.name, count(*) c from slice s
        join thread_track tt on s.track_id = tt.id join thread t using (utid)
        where t.upid = {upid} and s.name in ({names})
        group by t.utid order by c desc limit 1""")
    if not t:
        sys.exit("no frame zones in the process: the engine was not built with profiler=perfetto, "
                 "or the track_event data source was not enabled")
    return t[0]


def frame_slices(tp, utid, t_min, t_max):
    names = ",".join(f"'{n}'" for n in FRAME_SLICE_NAMES)
    frames = rows(tp, f"""
        select s.id, s.ts, s.dur, s.name from slice s
        join thread_track tt on s.track_id = tt.id
        where tt.utid = {utid} and s.name in ({names}) and s.dur > 0
          and s.ts >= {t_min} and s.ts <= {t_max}
        order by s.ts""")
    # Prefer the outermost frame slice when both names are present.
    by_name = defaultdict(list)
    for f in frames:
        by_name[f["name"]].append(f)
    for n in FRAME_SLICE_NAMES:
        if by_name.get(n):
            return by_name[n]
    return []


def marker_ts(tp, upid, name, occurrence):
    """Timestamp of the first/last zone named `name` in the process (the explorer can hide the
    loading screen more than once per session, e.g. after a realm redirect)."""
    r = rows(tp, f"""
        select s.ts from slice s
        join thread_track tt on s.track_id = tt.id
        join thread t using (utid)
        where t.upid = {upid} and s.name = '{name}' order by s.ts""")
    if not r:
        return None, 0
    pick = r[0] if occurrence == "first" else r[-1]
    return pick["ts"], len(r)


def thread_states_in(tp, utid, ts, te):
    return rows(tp, f"""
        select ts, dur, state, cpu, waker_utid, waker_id, blocked_function, io_wait
        from thread_state
        where utid = {utid} and ts < {te} and ts + dur > {ts}
        order by ts""")


def zone_at(tp, utid, ts):
    """Innermost 'godot' zone active on utid at ts."""
    r = rows(tp, f"""
        select s.name, s.depth, s.ts, s.dur,
               (select display_value from args a where a.arg_set_id = s.arg_set_id
                  and a.key = 'debug.text' limit 1) as text
        from slice s join thread_track tt on s.track_id = tt.id
        where tt.utid = {utid} and s.ts <= {ts} and s.ts + s.dur >= {ts}
        order by s.depth desc limit 1""")
    return r[0] if r else None


def zones_in_frame(tp, utid, ts, te):
    """Zones on utid overlapping [ts, te), with their clipped durations."""
    return rows(tp, f"""
        select s.name, s.depth,
               (select display_value from args a where a.arg_set_id = s.arg_set_id
                  and a.key = 'debug.text' limit 1) as text,
               (min(s.ts + s.dur, {te}) - max(s.ts, {ts})) as clipped
        from slice s join thread_track tt on s.track_id = tt.id
        where tt.utid = {utid} and s.ts < {te} and s.ts + s.dur > {ts}
          and s.name not in ({",".join(f"'{n}'" for n in FRAME_SLICE_NAMES)})
        order by clipped desc""")


def self_time_by_zone(tp, utid, ts, te):
    """Per-zone self time (exclusive of child zones) within [ts, te)."""
    zs = rows(tp, f"""
        select s.id, s.parent_id, s.name, max(s.ts, {ts}) as cts, min(s.ts + s.dur, {te}) as cte
        from slice s join thread_track tt on s.track_id = tt.id
        where tt.utid = {utid} and s.ts < {te} and s.ts + s.dur > {ts}""")
    child_sum = Counter()
    for z in zs:
        if z["parent_id"] is not None:
            child_sum[z["parent_id"]] += z["cte"] - z["cts"]
    out = Counter()
    for z in zs:
        out[z["name"]] += (z["cte"] - z["cts"]) - child_sum.get(z["id"], 0)
    return out


def running_on_cpu(tp, cpu, ts, te, exclude_utid):
    return rows(tp, f"""
        select t.name as tname, t.tid, p.name as pname,
               sum(min(s.ts + s.dur, {te}) - max(s.ts, {ts})) as run
        from sched s join thread t using (utid) left join process p using (upid)
        where s.cpu = {cpu} and s.ts < {te} and s.ts + s.dur > {ts}
          and s.utid != {exclude_utid} and s.utid != 0
        group by s.utid order by run desc limit 3""")


def thread_name(tp, utid, cache={}):
    if utid not in cache:
        r = rows(tp, f"""select t.name as tname, t.tid, p.name as pname
                         from thread t left join process p using (upid) where t.utid = {utid}""")
        cache[utid] = r[0] if r else {"tname": "?", "tid": utid, "pname": "?"}
    return cache[utid]


def ms(ns):
    return ns / 1e6


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("trace")
    ap.add_argument("--process", default="org.decentraland.godotexplorer")
    ap.add_argument("--threshold", type=float, default=None,
                    help="hiccup frame threshold in ms (default: max(33, 2 x median frame))")
    ap.add_argument("--top", type=int, default=20)
    ap.add_argument("--frames", type=int, default=30, help="how many hiccup frames to detail")
    ap.add_argument("--after-marker", default=None,
                    help="only frames after this zone/marker name (e.g. LoadingScreen::hidden)")
    ap.add_argument("--marker-occurrence", choices=("first", "last"), default="last",
                    help="which occurrence of --after-marker to use (default: last)")
    ap.add_argument("--after-s", type=float, default=None,
                    help="only frames after this many seconds from the first frame (alternative to --after-marker)")
    ap.add_argument("--json", default=None)
    args = ap.parse_args()

    tp = TraceProcessor(trace=args.trace)
    proc = find_process(tp, args.process)
    mt = main_thread(tp, proc["upid"])
    bounds = rows(tp, "select start_ts, end_ts from trace_bounds")[0]
    all_frames = frame_slices(tp, mt["utid"], bounds["start_ts"], bounds["end_ts"])
    if not all_frames:
        sys.exit("no frame zones on the engine thread")
    t0 = all_frames[0]["ts"]
    t_min = bounds["start_ts"]
    if args.after_marker:
        mts, count = marker_ts(tp, proc["upid"], args.after_marker, args.marker_occurrence)
        if mts is None:
            print(f"marker {args.after_marker} not found; using whole trace")
        else:
            t_min = mts
            print(f"window starts at {args.marker_occurrence} {args.after_marker} "
                  f"({count} occurrences) @{ms(mts - t0) / 1000:.1f}s after the first frame")
    if args.after_s is not None:
        t_min = t0 + int(args.after_s * 1e9)
        print(f"window starts @{args.after_s:.1f}s after the first frame")
    frames = [f for f in all_frames if f["ts"] >= t_min]
    if not frames:
        sys.exit("no frame zones in the selected window")

    durs = sorted(f["dur"] for f in frames)
    n = len(durs)
    if args.threshold is None:
        args.threshold = max(33.0, 2 * ms(durs[n // 2]))
    thr_ns = int(args.threshold * 1e6)
    hiccups = [f for f in frames if f["dur"] >= thr_ns]
    print(f"process {proc['name']} pid {proc['pid']} engine thread {mt['name']}({mt['tid']})")
    print(f"frames {n}  window {ms(frames[-1]['ts'] + frames[-1]['dur'] - frames[0]['ts']) / 1000:.1f}s  "
          f"p50 {ms(durs[n // 2]):.1f}ms  p95 {ms(durs[int(n * 0.95)]):.1f}ms  p99 {ms(durs[int(n * 0.99)]):.1f}ms  "
          f"max {ms(durs[-1]):.1f}ms")
    print(f"hiccups (>= {args.threshold:.1f}ms): {len(hiccups)}  "
          f"total {ms(sum(f['dur'] for f in hiccups)):.0f}ms")
    print()

    agg_states = Counter()
    agg_zone_self = Counter()
    agg_wakers = Counter()
    agg_waker_zone = Counter()
    agg_preempt = Counter()
    details = []

    hiccups_sorted = sorted(hiccups, key=lambda f: -f["dur"])
    for f in hiccups_sorted:
        ts, te = f["ts"], f["ts"] + f["dur"]
        states = thread_states_in(tp, mt["utid"], ts, te)
        st = Counter()
        blocked = []
        runnable = []
        last_cpu = None
        for i, s in enumerate(states):
            cts, cte = max(s["ts"], ts), min(s["ts"] + s["dur"], te)
            d = cte - cts
            label = STATE_LABEL.get(s["state"], s["state"])
            st[label] += d
            if label == "running":
                last_cpu = s["cpu"]
            elif s["cpu"] is None:
                # Runnable rows carry no cpu: charge the wait to the cpu it last ran on.
                s = dict(s, cpu=last_cpu)
            if label in ("sleeping", "uninterruptible"):
                # The waker is recorded on the Runnable interval that ends the wait.
                nxt = states[i + 1] if i + 1 < len(states) else None
                waker = nxt["waker_utid"] if nxt and nxt["state"] in ("R", "R+") else None
                blocked.append((d, dict(s, waker_utid=waker), cts, cte))
            elif label == "runnable":
                runnable.append((d, s, cts, cte))
        agg_states.update(st)
        zone_self = self_time_by_zone(tp, mt["utid"], ts, te)
        agg_zone_self.update(zone_self)
        assets = [z for z in zones_in_frame(tp, mt["utid"], ts, te) if z["text"]][:4]

        blocked.sort(key=lambda b: -b[0])
        waits = []
        for d, s, cts, cte in blocked[:5]:
            own = zone_at(tp, mt["utid"], cts + 1)
            w = None
            if s["waker_utid"] is not None:
                wn = thread_name(tp, s["waker_utid"])
                wz = zone_at(tp, s["waker_utid"], cte - 1)
                w = {"thread": f"{wn['tname']}({wn['tid']})", "process": wn["pname"],
                     "zone": wz["name"] if wz else None, "zone_text": (wz or {}).get("text")}
                agg_wakers[w["thread"]] += d
                agg_waker_zone[(w["thread"], w["zone"])] += d
            waits.append({"ms": ms(d), "state": s["state"], "main_zone": own["name"] if own else None,
                          "main_zone_text": (own or {}).get("text"), "waker": w})

        runnable.sort(key=lambda b: -b[0])
        preempts = []
        for d, s, cts, cte in runnable[:3]:
            if s["cpu"] is None:
                continue
            for r in running_on_cpu(tp, s["cpu"], cts, cte, mt["utid"]):
                key = f"{r['tname']}({r['tid']}) [{r['pname']}]"
                agg_preempt[key] += r["run"]
                preempts.append({"ms": ms(d), "cpu": s["cpu"], "thread": key, "ran_ms": ms(r["run"])})

        details.append({
            "ts_ms": ms(f["ts"] - frames[0]["ts"]), "dur_ms": ms(f["dur"]),
            "states_ms": {k: ms(v) for k, v in st.items()},
            "top_zones_ms": [(k, ms(v)) for k, v in zone_self.most_common(6)],
            "assets": [(z["name"], z["text"], ms(z["clipped"])) for z in assets],
            "waits": waits, "preempted_by": preempts,
        })

    for d in details[:args.frames]:
        print(f"== frame @{d['ts_ms'] / 1000:.2f}s  {d['dur_ms']:.1f}ms  " +
              "  ".join(f"{k}={v:.1f}" for k, v in sorted(d["states_ms"].items(), key=lambda kv: -kv[1])))
        for name, v in d["top_zones_ms"]:
            if v >= 0.5:
                print(f"   zone {v:7.1f}ms  {name}")
        for name, text, v in d["assets"]:
            print(f"   asset{v:7.1f}ms  {name} [{text}]")
        for w in d["waits"]:
            if w["ms"] < 1.0:
                continue
            own = f"{w['main_zone']}" + (f" [{w['main_zone_text']}]" if w["main_zone_text"] else "")
            if w["waker"]:
                wk = w["waker"]
                wz = (wk["zone"] or "-") + (f" [{wk['zone_text']}]" if wk.get("zone_text") else "")
                print(f"   wait {w['ms']:7.1f}ms  {w['state']:2}  in {own}  <- woken by {wk['thread']} doing {wz}")
            else:
                print(f"   wait {w['ms']:7.1f}ms  {w['state']:2}  in {own}  <- waker unknown")
        for p in d["preempted_by"]:
            print(f"   runnable {p['ms']:5.1f}ms on cpu{p['cpu']}: {p['thread']} ran {p['ran_ms']:.1f}ms")
    if len(details) > args.frames:
        print(f"... {len(details) - args.frames} more hiccup frames")
    print()

    total = sum(agg_states.values()) or 1
    print("== main thread inside hiccup frames")
    for k, v in agg_states.most_common():
        print(f"   {k:16} {ms(v):9.1f}ms  {100 * v / total:5.1f}%")
    print("== top zones by self time on main thread (hiccup frames)")
    for k, v in agg_zone_self.most_common(args.top):
        print(f"   {ms(v):9.1f}ms  {k}")
    print("== top wakers of the main thread (time main spent blocked before they woke it)")
    for k, v in agg_wakers.most_common(args.top):
        print(f"   {ms(v):9.1f}ms  {k}")
    print("== top (waker, zone) pairs")
    for (t, z), v in agg_waker_zone.most_common(args.top):
        print(f"   {ms(v):9.1f}ms  {t}  {z}")
    print("== top preemptors while main was runnable (ran on main's cpu)")
    for k, v in agg_preempt.most_common(args.top):
        print(f"   {ms(v):9.1f}ms  {k}")

    if args.json:
        with open(args.json, "w") as fh:
            json.dump({"process": proc, "main_thread": mt, "frames": n, "hiccups": len(hiccups),
                       "threshold_ms": args.threshold, "details": details,
                       "states_ms": {k: ms(v) for k, v in agg_states.items()},
                       "zones_self_ms": {k: ms(v) for k, v in agg_zone_self.items()},
                       "wakers_ms": {k: ms(v) for k, v in agg_wakers.items()},
                       "preemptors_ms": {k: ms(v) for k, v in agg_preempt.items()}}, fh, indent=1)
        print(f"json: {args.json}")


if __name__ == "__main__":
    main()
