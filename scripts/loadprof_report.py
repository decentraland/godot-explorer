#!/usr/bin/env python3
"""LoadingProfiler report — parse [LOADPROF] lines and print loading-timing tables.

The LoadingProfiler autoload (godot/src/logic/loading_profiler.gd) emits one line
per loading milestone. This script groups them by episode (a loading-screen-bounded
journey) and by current-scene render, prints a per-phase breakdown, and aggregates
timings across many loads grouped by entry reason (`when`).

Usage:
    # live capture on Android:
    adb logcat -c
    adb logcat -v time | grep --line-buffered LOADPROF | tee loadprof.log
    # ...do several loads / teleports / discover on the device, Ctrl-C, then:
    python3 scripts/loadprof_report.py loadprof.log

    # or pipe straight through:
    adb logcat | python3 scripts/loadprof_report.py

Line grammar (see loading_profiler.gd):
    [LOADPROF] v=1 ev=<event> ep=<id|-> t=<ms> dt=<ms|-> gap=<ms|-> k=v ...
    [LOADPROF-SUM] v=1 ep=<id> when=<..> realm=<..> reason=<..> total=<ms> label=ms ...
"""

import sys
import re
from collections import defaultdict, OrderedDict

LOADPROF = "[LOADPROF]"
LOADPROF_SUM = "[LOADPROF-SUM]"

# Ordered milestones shown in the per-episode table. label -> event name.
EPISODE_MILESTONES = [
    ("realm.changing", "realm.changing"),
    ("realm.about_start", "realm.about_fetch_start"),
    ("realm.about_end", "realm.about_fetch_end"),
    ("realm.changed", "realm.changed"),
    ("discovery", "discovery.desired_changed"),
    ("loading.started", "loading.started"),
    ("first_spawn", "scene.spawned"),
    ("first_ready", "loading.progress.ready"),
    ("loading.complete", "loading.complete"),
    ("scene.rendered", "scene.rendered"),
    ("place_data", "screen.place_data"),
    ("screen.hidden", "screen.hidden"),
]

# Aggregate columns (grouped by `when`). label -> event name (dt since episode).
AGG_COLUMNS = [
    ("realm_resolve", "realm.changed"),
    ("discovery", "discovery.desired_changed"),
    ("first_spawn", "scene.spawned"),
    ("complete", "loading.complete"),
    ("rendered", "scene.rendered"),
    ("hidden", "screen.hidden"),
]


def parse_kv(rest):
    """Parse space-separated k=v tokens into an OrderedDict."""
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


class Episode:
    def __init__(self, ep_id):
        self.ep_id = ep_id
        self.events = []  # list of (event, dt, kv)
        self.first_dt = {}  # event -> first dt seen
        self.first_frame = {}  # event -> render frame (f=) of first occurrence
        self.start_frame = None  # render frame at episode.begin
        self.frames_total = None  # render frames elapsed (from summary)
        self.pframes_total = None  # physics frames elapsed (from summary)
        self.when = "-"
        self.realm = "-"
        self.reason = "-"
        self.total = None

    def add_event(self, event, dt, kv):
        self.events.append((event, dt, kv))
        if event not in self.first_dt and dt is not None:
            self.first_dt[event] = dt
        fr = to_int(kv.get("f"))
        if event not in self.first_frame and fr is not None:
            self.first_frame[event] = fr
        if event == "episode.begin" and self.start_frame is None and fr is not None:
            self.start_frame = fr

    def absorb_summary(self, kv):
        self.when = kv.get("when", self.when)
        self.realm = kv.get("realm", self.realm)
        self.reason = kv.get("reason", self.reason)
        self.total = to_int(kv.get("total"), self.total)
        self.frames_total = to_int(kv.get("frames"), self.frames_total)
        self.pframes_total = to_int(kv.get("pframes"), self.pframes_total)
        # summary carries authoritative milestone dts too
        for label, ev in EPISODE_MILESTONES:
            key = {
                "realm.changing": "realm_changing",
                "realm.about_fetch_start": "about_start",
                "realm.about_fetch_end": "about_end",
                "realm.changed": "realm_changed",
                "discovery.desired_changed": "discovery",
                "loading.started": "loading_started",
                "scene.spawned": "first_spawn",
                "loading.progress.ready": "first_ready",
                "loading.complete": "complete",
                "scene.rendered": "scene_rendered",
                "screen.place_data": "place_data",
                "screen.hidden": "hidden",
            }.get(ev)
            if key and key in kv:
                val = to_int(kv[key])
                if val is not None and val >= 0 and ev not in self.first_dt:
                    self.first_dt[ev] = val


def parse(lines):
    episodes = OrderedDict()
    scene_renders = []  # (title, scene_id, ms)
    scene_current = {}  # scene_id -> title (last seen)

    def get_ep(ep_id):
        if ep_id not in episodes:
            episodes[ep_id] = Episode(ep_id)
        return episodes[ep_id]

    for raw in lines:
        if LOADPROF_SUM in raw:
            rest = raw.split(LOADPROF_SUM, 1)[1].strip()
            kv = parse_kv(rest)
            ep_id = kv.get("ep")
            if ep_id is not None:
                get_ep(ep_id).absorb_summary(kv)
            continue
        if LOADPROF not in raw:
            continue
        rest = raw.split(LOADPROF, 1)[1].strip()
        kv = parse_kv(rest)
        event = kv.get("ev")
        if event is None:
            continue
        ep_id = kv.get("ep")
        dt = to_int(kv.get("dt"))

        # scene render tracking (may be outside any episode, ep=-)
        if event == "scene.current":
            scene_current[kv.get("scene_id")] = kv.get("title", "?")
        elif event == "scene.rendered":
            sid = kv.get("scene_id")
            scene_renders.append(
                (scene_current.get(sid, "?"), sid, to_int(kv.get("ms")))
            )

        if ep_id and ep_id != "-":
            get_ep(ep_id).add_event(event, dt, kv)

    return episodes, scene_renders


def fmt_ms(v):
    return "-" if v is None else f"{v}"


def print_episode(ep):
    total = fmt_ms(ep.total)
    frames = "-" if ep.frames_total is None else str(ep.frames_total)
    pframes = "-" if ep.pframes_total is None else str(ep.pframes_total)
    print(
        f"\n── Episode {ep.ep_id}  when={ep.when}  realm={ep.realm}"
        f"  reason={ep.reason}  total={total}ms  frames={frames}  pframes={pframes} ──"
    )
    print(
        f"  {'milestone':<20} {'t(ms)':>8} {'Δ(ms)':>8} {'frame':>8} {'Δf':>6}"
    )
    prev = None
    for label, ev in EPISODE_MILESTONES:
        dt = ep.first_dt.get(ev)
        if dt is None:
            continue
        delta = "-" if prev is None else str(dt - prev)
        fr = ep.first_frame.get(ev)
        fr_s = "-" if fr is None else str(fr)
        df_s = "-" if (fr is None or ep.start_frame is None) else str(fr - ep.start_frame)
        print(f"  {label:<20} {dt:>8} {delta:>8} {fr_s:>8} {df_s:>6}")
        prev = dt
    print_episode_assets(ep)


def print_episode_assets(ep):
    counts = defaultdict(int)
    dts = defaultdict(list)
    for event, dt, _kv in ep.events:
        counts[event] += 1
        if dt is not None:
            dts[event].append(dt)
    g_start = counts.get("asset.gltf_start", 0)
    waves = counts.get("asset.download_wave_start", 0)
    if g_start == 0 and waves == 0:
        return
    line = (
        "  assets:"
        f" gltf_start={g_start}"
        f" downloaded={counts.get('asset.gltf_downloaded', 0)}"
        f" added={counts.get('asset.gltf_added', 0)}"
        f" error={counts.get('asset.gltf_error', 0)}"
        f" download_waves={waves}"
    )
    added = dts.get("asset.gltf_added", [])
    if added:
        line += f" | gltf_added {min(added)}..{max(added)}ms"
    dl = dts.get("asset.download_wave_start", [])
    dl_end = dts.get("asset.download_wave_end", [])
    if dl:
        end = max(dl_end) if dl_end else -1
        line += f" | downloads {min(dl)}..{end}ms"
    print(line)


def stats(values):
    vals = [v for v in values if v is not None and v >= 0]
    if not vals:
        return None
    vals_sorted = sorted(vals)
    n = len(vals_sorted)
    avg = sum(vals_sorted) / n
    p50 = vals_sorted[n // 2]
    return (n, min(vals_sorted), avg, p50, max(vals_sorted))


def print_aggregate(episodes):
    by_when = defaultdict(list)
    for ep in episodes.values():
        by_when[ep.when].append(ep)

    print("\n" + "=" * 78)
    print("AGGREGATE by entry reason (`when`) — ms since episode begin")
    print("=" * 78)
    for when, eps in sorted(by_when.items()):
        print(f"\n[{when}]  ({len(eps)} episode(s))")
        print(
            f"  {'metric':<16} {'n':>3} {'min':>8} {'avg':>8} {'p50':>8} {'max':>8}"
        )
        for label, ev in AGG_COLUMNS:
            s = stats([ep.first_dt.get(ev) for ep in eps])
            if s is None:
                continue
            n, mn, avg, p50, mx = s
            print(
                f"  {label:<16} {n:>3} {mn:>8} {avg:>8.0f} {p50:>8} {mx:>8}"
            )
        totals = stats([ep.total for ep in eps])
        if totals:
            n, mn, avg, p50, mx = totals
            print(f"  {'total(ms)':<16} {n:>3} {mn:>8} {avg:>8.0f} {p50:>8} {mx:>8}")
        frames = stats([ep.frames_total for ep in eps])
        if frames:
            n, mn, avg, p50, mx = frames
            print(f"  {'frames':<16} {n:>3} {mn:>8} {avg:>8.0f} {p50:>8} {mx:>8}")


def print_scene_renders(scene_renders):
    if not scene_renders:
        return
    print("\n" + "=" * 78)
    print("SCENE RENDER (current parcel scene → tick>=10 & GLTFs done)")
    print("=" * 78)
    print(f"  {'title':<40} {'scene_id':>9} {'ms':>8}")
    for title, sid, ms in scene_renders:
        print(f"  {title[:40]:<40} {str(sid):>9} {fmt_ms(ms):>8}")
    s = stats([ms for _, _, ms in scene_renders])
    if s:
        n, mn, avg, p50, mx = s
        print(f"\n  render ms  n={n}  min={mn}  avg={avg:.0f}  p50={p50}  max={mx}")


def main():
    if len(sys.argv) > 1 and sys.argv[1] not in ("-", "/dev/stdin"):
        with open(sys.argv[1], "r", errors="replace") as f:
            lines = f.readlines()
    else:
        lines = sys.stdin.readlines()

    episodes, scene_renders = parse(lines)

    if not episodes and not scene_renders:
        print("No [LOADPROF] lines found. Is the profiler enabled and logcat captured?")
        return

    print(f"Parsed {len(episodes)} episode(s), {len(scene_renders)} scene render(s).")
    for ep in episodes.values():
        print_episode(ep)
    print_aggregate(episodes)
    print_scene_renders(scene_renders)


if __name__ == "__main__":
    main()
