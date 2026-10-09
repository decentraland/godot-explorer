#!/usr/bin/env python3
"""Attribute hiccup causes to assets and code paths (issue #3015).

Companion of hiccup_report.py for traces recorded with the fork's Perfetto zones. Three sections:

  pipelines  every pipeline compiled on the engine thread: shader, mesh (resolved to the resource
             path of its ArrayMesh/PrimitiveMesh creation zone), material (resolved the same way),
             the call chain that created the mesh, and the frame the compile landed in.
  emotes     every Emote::* / Wearable::* / Avatar::* mark with the duration of the function it
             sits in, its caller chain and the frame.
  process    where the engine thread's `process` time goes: Node::process by class, animation
             mixers, SceneTree phases, and GDScript functions by self time.

Usage:
  ~/.venvs/perfetto/bin/python scripts/bench/hiccup_attribution.py TRACE
      [--after-marker LoadingScreen::hidden] [--top 25] [--section pipelines|emotes|process]
"""
import argparse
import re
import sys
from collections import Counter, defaultdict

try:
    from perfetto.trace_processor import TraceProcessor
except ImportError:
    sys.exit("pip install perfetto (the script expects ~/.venvs/perfetto)")


def rows(tp, sql):
    return [r.__dict__ for r in tp.query(sql)]


def ms(ns):
    return ns / 1e6


def text_of(name):
    return f"(select display_value from args a where a.arg_set_id = s.arg_set_id and a.key = 'debug.text' limit 1) as {name}"


def ancestors(tp, slice_id, limit=8):
    out = []
    cur = slice_id
    for _ in range(limit):
        r = rows(tp, f"""select s.parent_id, s.name, {text_of('text')} from slice s where s.id = {cur}""")
        if not r or r[0]["parent_id"] is None:
            break
        cur = r[0]["parent_id"]
        p = rows(tp, f"""select s.name, {text_of('text')} from slice s where s.id = {cur}""")[0]
        out.append(p["name"] + (f" [{p['text']}]" if p["text"] else ""))
    return out


def short(path):
    if not path:
        return path
    return re.sub(r"^(user://content/|res://|/data/data/[^/]+/files/content/)", "", path)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("trace")
    ap.add_argument("--process", default="org.decentraland.godotexplorer")
    ap.add_argument("--after-marker", default=None)
    ap.add_argument("--top", type=int, default=25)
    ap.add_argument("--section", choices=("pipelines", "emotes", "process", "all"), default="all")
    args = ap.parse_args()

    tp = TraceProcessor(trace=args.trace)
    proc = rows(tp, f"select upid, pid from process where name like '%{args.process}%' order by pid desc limit 1")
    if not proc:
        sys.exit("process not found")
    upid = proc[0]["upid"]
    vk = rows(tp, f"""select s.track_id, t.utid from slice s join thread_track tt on s.track_id = tt.id
                      join thread t using (utid) where t.upid = {upid} and s.name = 'Main::iteration'
                      group by s.track_id order by count(*) desc limit 1""")
    if not vk:
        sys.exit("no Main::iteration zones: not a profiler build")
    track, utid = vk[0]["track_id"], vk[0]["utid"]
    t0 = rows(tp, f"select min(ts) ts from slice where name = 'Main::iteration' and track_id = {track}")[0]["ts"]
    t_min = t0
    if args.after_marker:
        m = rows(tp, f"""select max(s.ts) ts from slice s join thread_track tt on s.track_id = tt.id
                         join thread t using (utid) where t.upid = {upid} and s.name = '{args.after_marker}'""")
        if m and m[0]["ts"] is not None:
            t_min = m[0]["ts"]
            print(f"window starts at {args.after_marker} @{ms(t_min - t0) / 1000:.1f}s after the first frame")
    frames = rows(tp, f"""select id, ts, dur from slice where name = 'Main::iteration' and track_id = {track}
                          and ts >= {t_min} order by ts""")
    durs = sorted(f["dur"] for f in frames)
    thr = max(33e6, 2 * durs[len(durs) // 2]) if durs else 33e6
    print(f"frames {len(frames)}  hiccup threshold {ms(thr):.0f}ms")

    def frame_of(ts):
        lo, hi = 0, len(frames) - 1
        while lo <= hi:
            mid = (lo + hi) // 2
            f = frames[mid]
            if ts < f["ts"]:
                hi = mid - 1
            elif ts >= f["ts"] + f["dur"]:
                lo = mid + 1
            else:
                return f
        return None

    # Resource creation zones: rid -> (path, ts, slice id)
    created = {}
    for z in rows(tp, f"""select s.id, s.ts, s.name, {text_of('text')} from slice s
                          where s.name in ('ArrayMesh::_set_surfaces', 'PrimitiveMesh::_update',
                                           'BaseMaterial3D::_update_shader', 'ShaderMaterial::set_shader')
                          and s.ts >= {t0} order by s.ts"""):
        m = re.search(r"rid=(\d+)", z["text"] or "")
        if m and int(m.group(1)) not in created:
            created[int(m.group(1))] = (z["name"], (z["text"] or "").replace(m.group(0), "").strip(), z["ts"], z["id"])

    if args.section in ("pipelines", "all"):
        print("\n== pipelines compiled on the engine thread (draw time)")
        pipes = rows(tp, f"""select s.id, s.ts, s.dur, {text_of('text')} from slice s
                             where s.track_id = {track} and s.name = 'SceneShaderForwardMobile::_create_pipeline'
                             and s.ts >= {t_min} order by s.dur desc""")
        by_asset = Counter()
        by_shader = Counter()
        for p in pipes:
            kv = dict(re.findall(r"(\w+)=(\S+)", p["text"] or ""))
            mesh = created.get(int(kv.get("mesh", 0)))
            mat = created.get(int(kv.get("material", 0)))
            asset = short(mesh[1]) if mesh else f"mesh rid {kv.get('mesh')} (created before the trace or not an ArrayMesh)"
            by_asset[asset] += p["dur"]
            by_shader[(kv.get("shader"), kv.get("version"), kv.get("ubershader"))] += p["dur"]
            if p is pipes[0] or p["dur"] >= 50e6:
                f = frame_of(p["ts"])
                print(f"-- {ms(p['dur']):7.1f}ms @{ms(p['ts'] - t0) / 1000:.2f}s  frame {ms(f['dur']) if f else 0:.0f}ms  "
                      f"shader={kv.get('shader')} version={kv.get('version')} ubershader={kv.get('ubershader')} src={kv.get('src')}")
                if mesh:
                    print(f"   mesh: {short(mesh[1])}  created {ms(p['ts'] - mesh[2]) / 1000:.1f}s earlier by {mesh[0]}")
                    for a in ancestors(tp, mesh[3]):
                        print(f"      <- {a[:150]}")
                else:
                    print(f"   mesh rid {kv.get('mesh')}: no creation zone in the trace")
                if mat:
                    print(f"   material: {short(mat[1])}  created {ms(p['ts'] - mat[2]) / 1000:.1f}s earlier by {mat[0]}")
        print(f"\n   {len(pipes)} pipelines, {ms(sum(p['dur'] for p in pipes)):.0f}ms total")
        print("   by asset (mesh):")
        for k, v in by_asset.most_common(args.top):
            print(f"   {ms(v):8.1f}ms  {k}")
        print("   by shader / version / ubershader:")
        for k, v in by_shader.most_common(args.top):
            print(f"   {ms(v):8.1f}ms  {k}")

    if args.section in ("emotes", "all"):
        print("\n== emote / wearable / avatar loads on the engine thread")
        marks = rows(tp, f"""select s.id, s.ts, s.name, s.parent_id, {text_of('text')} from slice s
                             where s.track_id = {track} and s.ts >= {t_min}
                             and (s.name like 'Emote::%' or s.name like 'Wearable::%' or s.name like 'Avatar::%'
                                  or s.name = 'ContentProvider::extract_emote')
                             order by s.ts""")
        per_kind = Counter()
        per_id = Counter()
        for m in marks:
            parent = rows(tp, f"select name, dur from slice where id = {m['parent_id']}")[0] if m["parent_id"] else None
            d = parent["dur"] if parent else 0
            per_kind[m["name"]] += d
            per_id[(m["name"], (m["text"] or "")[:90])] += d
            if d >= 30e6:
                f = frame_of(m["ts"])
                print(f"-- {m['name']} {m['text']}  in {parent['name'].replace('res://src/', '') if parent else '-'} {ms(d):.1f}ms"
                      f"  @{ms(m['ts'] - t0) / 1000:.2f}s frame {ms(f['dur']) if f else 0:.0f}ms")
                for a in ancestors(tp, m["parent_id"], 5):
                    print(f"      <- {a.replace('res://src/', '')[:150]}")
        print("   total by kind (time of the enclosing function):")
        for k, v in per_kind.most_common():
            print(f"   {ms(v):8.1f}ms  {k}")
        print("   top ids:")
        for (k, t), v in per_id.most_common(args.top):
            print(f"   {ms(v):8.1f}ms  {k} {t}")

    if args.section in ("process", "all"):
        print("\n== engine thread `process`: where it goes (self time over the window)")
        z = rows(tp, f"""
            with z as (select s.id, s.parent_id, s.name, s.dur, {text_of('text')} from slice s
                       where s.track_id = {track} and s.ts >= {t_min}),
                 ch as (select parent_id, sum(dur) d from z group by parent_id)
            select z.name, z.text, sum(z.dur - coalesce(ch.d, 0)) self_ns, count(*) n, max(z.dur) mx
            from z left join ch on ch.parent_id = z.id
            where z.name like 'Node::process%' or z.name like 'SceneTree::%' or z.name like 'AnimationMixer%'
               or z.name like 'Viewport::%' or z.name like 'res://%' or z.name = 'process' or z.name = 'MessageQueue::flush'
            group by z.name, z.text order by self_ns desc limit {args.top}""")
        for r in z:
            label = r["name"].replace("res://src/", "")
            if r["text"] and r["name"].startswith(("Node::process", "AnimationMixer")):
                label += f" [{r['text']}]"
            print(f"   {ms(r['self_ns']):9.1f}ms {r['n']:7d} calls  max {ms(r['mx']):6.1f}ms  {label}")


if __name__ == "__main__":
    main()
