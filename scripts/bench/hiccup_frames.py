#!/usr/bin/env python3
"""Per-frame hiccup breakdown with asset and avatar names (issue #3015).

For every engine frame longer than --threshold ms it lists what the engine thread spent the time
on, resolved to names: the scene GLB (`models/x.glb` of scene "Genesis Plaza"), the wearable or
emote urn and the avatar it belongs to, the animation node path, the SDK component state, the
engine phase. Needs a trace recorded with the fork's Perfetto zones and the explorer marks.

Usage:
  ~/.venvs/perfetto/bin/python scripts/bench/hiccup_frames.py TRACE [--threshold 20]
      [--after-marker LoadingScreen::hidden] [--frames 40] [--items 8] [--out frames.txt]
"""
import argparse
import re
import sys
from collections import Counter, defaultdict

try:
    from perfetto.trace_processor import TraceProcessor
except ImportError:
    sys.exit("pip install perfetto (the script expects ~/.venvs/perfetto)")

HASH_RE = re.compile(r"(bafy[a-z0-9]{50,}|bafk[a-z0-9]{50,}|Qm[1-9A-HJ-NP-Za-km-z]{44})")


def rows(tp, sql):
    return [r.__dict__ for r in tp.query(sql)]


def ms(ns):
    return ns / 1e6


def kv(text):
    return dict(re.findall(r"(\w+)=(\S+)", text or ""))


class Names:
    """Resolves hashes, RIDs and avatar ids to human names from the marks in the trace."""

    def __init__(self, tp, upid):
        self.glb = {}       # content hash -> "models/x.glb (scene N)"
        self.wearable = {}  # content hash -> "urn (avatar)"
        self.emote = {}     # content hash -> "urn (avatar)"
        self.avatar = {}    # avatar id -> name
        self.mesh = {}      # rid -> resource path
        self.material = {}
        self.scene = {}     # scene id -> title
        self.last = None    # (side, asset) of the last asset() hit, read by the per-asset ledger
        self.tp = tp
        last_avatar = "?"
        for z in rows(tp, f"""select s.id, s.name, s.ts, (select display_value from args a where a.arg_set_id = s.arg_set_id
                                 and a.key = 'debug.text' limit 1) as text
                              from slice s join thread_track tt on s.track_id = tt.id join thread t using (utid)
                              where t.upid = {upid} and s.name in ('GltfCoordinator::load', 'GltfCoordinator::realize',
                                'Wearable::load', 'Emote::request', 'Emote::load_gltf', 'Emote::extract',
                                'Avatar::load_wearables', 'DclAvatar::process', 'Scene::process',
                                'ArrayMesh::_set_surfaces', 'ArrayMesh::add_surface', 'ArrayMesh::rid', 'PrimitiveMesh::_update',
                                'BaseMaterial3D::_update_shader', 'ShaderMaterial::set_shader')
                              order by s.ts"""):
            text = z["text"] or ""
            d = kv(text)
            if "avatar" in d and "name" in d:
                self.avatar.setdefault(d["avatar"], d["name"])
            if z["name"] == "Avatar::load_wearables":
                last_avatar = d.get("name") or d.get("avatar", "?")
            if z["name"] in ("GltfCoordinator::load", "GltfCoordinator::realize") and "hash" in d:
                self.glb.setdefault(d["hash"], d.get("src", "?"))
            elif z["name"] in ("Wearable::load",) and "hash" in d:
                self.wearable.setdefault(d["hash"], f"{text.split(' ')[0]} of {last_avatar}")
            elif z["name"] in ("Emote::request", "Emote::load_gltf") and "hash" in d:
                who = d.get("name") or self.avatar.get(d.get("avatar", ""), d.get("avatar", "?"))
                self.emote.setdefault(d["hash"], f"{text.split(' ')[0]} of {who}")
            elif z["name"] == "Scene::process":
                m = re.match(r"scene=(\S+)\s*(.*)", text)
                if m and m.group(2):
                    self.scene.setdefault(m.group(1), m.group(2))
            elif z["name"] in ("ArrayMesh::_set_surfaces", "ArrayMesh::add_surface", "ArrayMesh::rid", "PrimitiveMesh::_update"):
                m = re.search(r"rid=(\d+)", text)
                if m and m.group(1) != "0" and not self.mesh.get(m.group(1)):
                    path = text.replace(m.group(0), "").strip()
                    if not HASH_RE.search(path):
                        path = self._built_by(z["id"]) or path
                    self.mesh[m.group(1)] = path
            elif z["name"] in ("BaseMaterial3D::_update_shader", "ShaderMaterial::set_shader"):
                m = re.search(r"rid=(\d+)", text)
                if m and m.group(1) != "0" and not self.material.get(m.group(1)):
                    path = text.replace(m.group(0), "").strip()
                    if not HASH_RE.search(path):
                        path = self._built_by(z["id"]) or path
                    self.material[m.group(1)] = path

    def _built_by(self, zone_id):
        """Resource created at runtime: name it after the import/load zone that carries an asset hash
        or, failing that, the script function (and avatar) that built it."""
        script = ""
        for a in rows(self.tp, f"""select p.name, (select display_value from args a where a.arg_set_id = p.arg_set_id
                                    and a.key = 'debug.text' limit 1) as text
                                 from ancestor_slice({zone_id}) p order by p.depth desc"""):
            if a["text"] and HASH_RE.search(a["text"]):
                return f"{a['name']} {a['text']}"
            if not script and a["name"].startswith("res://"):
                script = a["name"].replace("res://src/", "")
            if a["name"] == "DclAvatar::process" and a["text"]:
                script += f" of avatar {a['text']}"
        return f"built by {script}" if script else ""

    def asset(self, path):
        """Resource path (scn, zip, cache file) -> scene GLB / wearable / emote name."""
        if not path:
            return ""
        h = HASH_RE.search(path)
        if h:
            key = h.group(1)
            if key in self.glb:
                out, side = f"{self.glb[key]} ({key[:12]}…)", "scene"
            elif key in self.wearable:
                out, side = f"wearable {self.wearable[key]} ({key[:12]}…)", "avatar"
            elif key in self.emote:
                out, side = f"emote {self.emote[key]} ({key[:12]}…)", "avatar"
            elif "wearable" in path:
                out, side = f"wearable {key[:16]}…", "avatar"
            elif "emote" in path:
                out, side = f"emote {key[:16]}…", "avatar"
            else:
                out, side = f"{key[:16]}… ({path.split('/')[-1][:40]})", "?"
            self.last = (side, out)
            return out
        out = path.replace("res://", "").replace("user://content/", "")[:90]
        if out.startswith("built by "):
            avatar = re.search(r" of avatar (.+)$", out)
            self.last = ("avatar", f"avatar {avatar.group(1)} (runtime mesh/material)") if avatar else ("?", out)
        elif out.startswith("assets/avatar/") or "avatar/" in out:
            self.last = ("avatar", f"avatar shader {out.split('/')[-1]}")
        return out

    def scene_name(self, sid):
        return self.scene.get(sid, f"scene {sid}")


def label(name, text, names):
    """Human label for a zone."""
    d = kv(text)
    if name == "SceneShaderForwardMobile::_create_pipeline":
        mesh = names.mesh.get(d.get("mesh", ""), "") or names.material.get(d.get("material", ""), "")
        shader = d.get("shader", "?").replace("res://assets/", "")
        src = {"2": "on add", "4": "specialized at draw", "3": "ubershader at draw", "1": "mesh"}.get(d.get("src"), "?")
        what = names.asset(mesh) if mesh else f"mesh rid {d.get('mesh')}"
        if not mesh and shader.startswith("avatar/"):
            names.last = ("avatar", f"avatar shader {shader.split('/')[-1]}")
        return f"pipeline compile ({src}) for {what} shader {shader}"
    if name in ("MaterialStorage::update", "BaseMaterial3D::_update_shader"):
        m = re.search(r"rid=(\d+)", text or "")
        mat = names.material.get(m.group(1), "") if m else ""
        return f"material update of {names.asset(mat) if mat else (text or '?')}"
    if name in ("ArrayMesh::_set_surfaces", "ArrayMesh::add_surface", "PrimitiveMesh::_update"):
        return f"mesh upload {names.asset(text)}"
    if name in ("ResourceLoader::_load", "ResourceLoader::load", "ResourceLoader::load_threaded_get"):
        return f"load {names.asset(text)}"
    if name == "GltfCoordinator::load":
        names.asset(text)
        return f"GLB load {d.get('src', '?')} of {names.scene_name('0')}"
    if name == "GltfCoordinator::realize":
        names.asset(text)
        return f"GLB instantiate+add {d.get('src', '?')} {text.split(' ')[-1]}"
    if name == "ContentProvider::load_resource_pack":
        return f"mount asset pack {names.asset(text)}"
    if name == "ContentProvider::extract_emote":
        return f"emote extract {names.asset(text)}"
    if name == "ContentProvider::gltf_import":
        return f"runtime GLTF import {names.asset(text)}"
    if name.startswith("Emote::") or name.startswith("Wearable::") or name.startswith("Avatar::"):
        names.asset(text)
        if not names.last or names.last[0] == "?":
            names.last = ("avatar", f"{name.split('::')[0].lower()} {text.split(' ')[0]}")
        return f"{name} {text}"
    if name == "AnimationMixer::_process_animation":
        p = text or ""
        h = HASH_RE.search(p)
        node = p.split("/")[-1]
        where = names.asset(h.group(1)) if h else "/".join(p.replace("/root/explorer/", "").split("/")[-3:-1])
        return f"animation update of {where} ({node})"
    if name == "DclAvatar::process":
        return f"avatar {text} (Rust process)"
    if not text and re.match(r"^[A-Z][A-Za-z0-9]+$", name):
        return f"node process ({name})"
    if name == "Scene::process":
        return f"scene {text}"
    if name.startswith("res://"):
        return name.replace("res://src/", "")
    if "scene=" in (text or "") and name[0].isupper() and "::" not in name:
        return f"state {name} of {names.scene_name(d.get('scene', '?'))}"
    return name + (f" [{text[:60]}]" if text else "")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("trace")
    ap.add_argument("--process", default="org.decentraland.godotexplorer")
    ap.add_argument("--threshold", type=float, default=20.0)
    ap.add_argument("--after-marker", default=None)
    ap.add_argument("--frames", type=int, default=40, help="worst frames to print (all go to --out)")
    ap.add_argument("--items", type=int, default=8)
    ap.add_argument("--out", default=None)
    ap.add_argument("--json", default=None, help="write every frame with its named breakdown as JSON")
    ap.add_argument("--ledger", default=None, help="write engine-thread time per asset (scene vs avatar) over every frame")
    ap.add_argument("--hiccup-ms", type=float, default=100.0, help="frame length counted as a hiccup in the ledger")
    args = ap.parse_args()

    tp = TraceProcessor(trace=args.trace)
    proc = rows(tp, f"select upid from process where name like '%{args.process}%' order by pid desc limit 1")
    if not proc:
        sys.exit("process not found")
    upid = proc[0]["upid"]
    vk = rows(tp, f"""select s.track_id from slice s join thread_track tt on s.track_id = tt.id join thread t using (utid)
                      where t.upid = {upid} and s.name = 'Main::iteration' group by s.track_id order by count(*) desc limit 1""")
    if not vk:
        sys.exit("no Main::iteration zones: not a profiler build")
    track = vk[0]["track_id"]
    names = Names(tp, upid)
    t0 = rows(tp, f"select min(ts) ts from slice where name = 'Main::iteration' and track_id = {track}")[0]["ts"]
    t_min = t0
    if args.after_marker:
        m = rows(tp, f"""select max(s.ts) ts from slice s join thread_track tt on s.track_id = tt.id join thread t using (utid)
                         where t.upid = {upid} and s.name = '{args.after_marker}'""")
        if m and m[0]["ts"] is not None:
            t_min = m[0]["ts"]
    frames = rows(tp, f"""select id, ts, dur from slice where name = 'Main::iteration' and track_id = {track}
                          and ts >= {t_min} and dur >= {int(args.threshold * 1e6)} order by ts""")
    total_frames = rows(tp, f"select count(*) c from slice where name = 'Main::iteration' and track_id = {track} and ts >= {t_min}")[0]["c"]
    header = (f"{len(frames)} frames over {args.threshold:.0f}ms out of {total_frames}"
              f"{' after ' + args.after_marker if args.after_marker else ''}; "
              f"scenes: {', '.join(f'{k}={v}' for k, v in names.scene.items())}")
    print(header)

    # Self time of every zone on the engine thread, clipped to each frame.
    all_z = rows(tp, f"""select s.id, s.parent_id, s.ts, s.dur, s.name,
                          (select display_value from args a where a.arg_set_id = s.arg_set_id and a.key = 'debug.text' limit 1) as text
                          from slice s where s.track_id = {track} and s.ts >= {t_min} and s.name != 'Main::iteration'
                          and s.name != 'OS_Android::main_loop_iterate' order by s.ts""")
    child_sum = Counter()
    by_id = {}
    for z in all_z:
        by_id[z["id"]] = z
        if z["parent_id"] is not None:
            child_sum[z["parent_id"]] += z["dur"]

    # Engine leaf zones (RD calls, uploads...) are charged to the nearest ancestor that names an
    # asset, a script function or a component state, so "render_pipeline_create" shows up as the
    # pipeline of a concrete mesh.
    OWNERS = ("SceneShaderForwardMobile::_create_pipeline", "ArrayMesh::_set_surfaces", "ArrayMesh::add_surface",
              "PrimitiveMesh::_update", "GltfCoordinator::load", "GltfCoordinator::realize", "ResourceLoader::_load",
              "ContentProvider::load_resource_pack", "ContentProvider::extract_emote", "ContentProvider::gltf_import",
              "ContentProvider::texture_decode", "AnimationMixer::_process_animation", "DclAvatar::process",
              "MessageQueue::flush", "render viewport", "physics_process", "3D physics", "WorkerThreadPool::wait_for_group_task_completion",
              "MaterialStorage::update")

    # Leaf engine work that belongs to whatever asked for it (uploads, RD calls, GPU waits).
    NON_OWNER = ("RenderingDevice::", "driver->", "_stall_for_frame", "QueueSubmit", "_submit_transfer", "_free_pending",
                 "draw_graph", "TextureStorage::texture_2d_initialize", "TextureStorage::_texture_2d_update",
                 "MeshStorage::mesh_add_surface", "ShaderRD::", "RendererSceneCull::_render_scene", "RenderForwardMobile::_render_scene",
                 "SceneTree::_process", "QueuePresentKHR", "BindImageMemory2")

    def is_owner(z):
        n = z["name"]
        if n.startswith(NON_OWNER) or n in NON_OWNER:
            return False
        if n in OWNERS or n.startswith("res://") or "::" in n:
            return True
        if z["text"] and "scene=" in z["text"]:
            return True
        return bool(re.match(r"^[A-Z][A-Za-z0-9]+$", n))  # node class zones

    avatar_of_process = {}
    for z in all_z:
        if z["name"] == "DclAvatar::process" and z["parent_id"] is not None and z["text"]:
            avatar_of_process[z["parent_id"]] = z["text"]

    def avatar_name_for(z):
        cur = z
        for _ in range(12):
            if cur["id"] in avatar_of_process:
                return avatar_of_process[cur["id"]]
            if cur["parent_id"] is None or cur["parent_id"] not in by_id:
                return None
            cur = by_id[cur["parent_id"]]
        return None

    def owner(z):
        cur = z
        for _ in range(12):
            if is_owner(cur):
                return cur
            if cur["parent_id"] is None or cur["parent_id"] not in by_id:
                return z
            cur = by_id[cur["parent_id"]]
        return z

    GPU_WAITS = ("driver->fence_wait", "QueuePresentKHR", "_stall_for_frame")
    READBACKS = {"MeshStorage::mesh_get_surface": "mesh readback: surface_get_arrays / get_faces / create_trimesh_shape",
                 "TextureStorage::texture_2d_get": "texture readback: Texture2D.get_image",
                 "ParticlesStorage::particles_get_current_aabb": "particles readback: capture_aabb",
                 "RenderingDevice::buffer_get_data": "buffer_get_data", "RenderingDevice::texture_get_data": "texture_get_data"}

    def label_of(z):
        o = owner(z)
        lab = label(o["name"], o["text"], names)
        if o["name"] == "WorkerThreadPool::wait_for_group_task_completion":
            # The engine waits for the SPIR-V of a shader it needs now: name the material and its asset.
            cur = o
            for _ in range(10):
                if cur["parent_id"] is None or cur["parent_id"] not in by_id:
                    break
                cur = by_id[cur["parent_id"]]
                if cur["name"] in ("MaterialStorage::update", "BaseMaterial3D::_update_shader"):
                    return "wait for shader SPIR-V: " + label(cur["name"], cur["text"], names)
                if cur["name"] in ("RendererSceneCull::update_dirty_instances", "SceneTree::_flush_delete_queue"):
                    return f"wait for shader SPIR-V under {cur['name']}"
        if re.match(r"^_?[a-z][a-z0-9_]*$", o["name"]) and not o["text"]:
            par = by_id.get(o["parent_id"]) if o["parent_id"] is not None else None
            lab = f"deferred call {o['name']}()" if par and par["name"] == "MessageQueue::flush" else o["name"]
        if o["name"].startswith("res://src/decentraland_components/avatar/") or o["name"] == "DclAvatar::process":
            who = avatar_name_for(o)
            if who:
                lab = f"avatar {who}: {lab}"
        if z["name"] in GPU_WAITS:
            if o is z:
                return f"GPU wait ({z['name']}, frame pacing / GPU bound)"
            # A GPU wait outside the render/present phase is a forced readback: name the engine entry
            # (mesh arrays, texture image, particle AABB) and the code that asked for it.
            cur, via, asker = z, "", None
            for _ in range(10):
                if cur["parent_id"] is None or cur["parent_id"] not in by_id:
                    break
                cur = by_id[cur["parent_id"]]
                if cur["name"] in READBACKS:
                    via = READBACKS[cur["name"]]
                elif via and is_owner(cur):
                    asker = cur
                    break
            if asker is not None:
                lab = label(asker["name"], asker["text"], names)
                who = avatar_name_for(asker)
                if who and (asker["name"].startswith("res://src/decentraland_components/avatar/") or asker["name"] == "DclAvatar::process"):
                    lab = f"avatar {who}: {lab}"
            return f"GPU stall ({via or z['name']}) forced by {lab}"
        return lab
    if args.ledger:
        write_ledger(args, tp, track, t_min, all_z, child_sum, label_of, names)
    by_frame = defaultdict(list)
    fi = 0
    for z in all_z:
        while fi < len(frames) and z["ts"] >= frames[fi]["ts"] + frames[fi]["dur"]:
            fi += 1
        if fi >= len(frames):
            break
        f = frames[fi]
        if z["ts"] >= f["ts"]:
            by_frame[f["id"]].append(z)

    agg = Counter()
    lines = []
    details = []
    sides = {}
    utid = rows(tp, f"select utid from thread_track where id = {track}")[0]["utid"]
    for f in frames:
        items = Counter()
        for z in by_frame.get(f["id"], []):
            self_ns = z["dur"] - child_sum.get(z["id"], 0)
            if self_ns <= 0:
                continue
            names.last = None
            lab = label_of(z)
            sides.setdefault(lab, side_asset(lab, names))
            if self_ns >= 5e6:
                b = blocker_of(tp, utid, z, names)
                if b:
                    moved = min(b[0], self_ns)
                    blab = f"lock wait behind {b[1][1]} ({b[2].replace('blocked by worker: ', '')} on a worker)"
                    sides.setdefault(blab, b[1])
                    items[blab] += moved
                    self_ns -= moved
            items[lab] += self_ns
        agg.update(items)
        details.append((f, items))
    if args.json:
        import json
        with open(args.json, "w") as fh:
            json.dump({"header": header, "threshold_ms": args.threshold, "t0": t0,
                       "frames": [{"t": round(ms(f["ts"] - t0) / 1000, 3), "ms": round(ms(f["dur"]), 2),
                                   "items": [[lab, round(ms(v), 2), sides.get(lab, ("engine", ""))[0], sides.get(lab, ("", ""))[1]]
                                             for lab, v in items.most_common(14) if v >= 0.3e6]}
                                  for f, items in details]}, fh)
        print(f"json: {args.json}")
    details.sort(key=lambda d: -d[0]["dur"])
    for f, items in details:
        block = [f"== frame @{ms(f['ts'] - t0) / 1000:7.2f}s  {ms(f['dur']):7.1f}ms"]
        for lab, v in items.most_common(args.items):
            if v >= 0.5e6:
                block.append(f"   {ms(v):7.1f}ms  {lab}")
        lines.append("\n".join(block))
    for b in lines[:args.frames]:
        print(b)
    if len(lines) > args.frames:
        print(f"... {len(lines) - args.frames} more frames ({'see ' + args.out if args.out else 'use --out'})")
    pipes = Counter()
    for lab, v in agg.items():
        if lab.startswith("pipeline compile"):
            pipes[lab.split(" for ", 1)[1].split(" shader ")[0]] += v
    if pipes:
        print(f"\n== pipeline compile time by asset over those frames")
        for k, v in pipes.most_common(25):
            print(f"   {ms(v):8.1f}ms  {k}")
    print(f"\n== totals over the {len(frames)} frames over {args.threshold:.0f}ms (self time)")
    for lab, v in agg.most_common(40):
        print(f"   {ms(v):8.1f}ms  {lab}")
    if args.out:
        with open(args.out, "w") as fh:
            fh.write(header + "\n\n" + "\n".join(lines) + "\n\n== totals\n" +
                     "\n".join(f"   {ms(v):8.1f}ms  {lab}" for lab, v in agg.most_common(200)) + "\n")
        print(f"full list: {args.out}")


KINDS = (("pipeline compile", "pipeline compile"), ("wait for shader SPIR-V", "shader SPIR-V wait"),
         ("material update", "material update"), ("GPU stall", "GPU readback"), ("mesh upload", "mesh upload"),
         ("mount asset pack", "pack mount"), ("load ", "resource load"), ("GLB load", "GLB load"),
         ("GLB instantiate", "GLB instantiate"), ("runtime GLTF import", "runtime GLTF import"),
         ("emote extract", "emote extract"), ("animation update", "animation"), ("Emote::", "emote"),
         ("Wearable::", "wearable"), ("state ", "SDK component state"))


def kind_of(lab):
    for prefix, k in KINDS:
        if lab.startswith(prefix):
            return k
    if lab.startswith("avatar "):
        return "avatar script / process"
    return "other"


def side_asset(lab, names):
    """(scene|avatar|engine, asset) for a label; names.last holds the asset hit while labelling."""
    if names.last and names.last[0] != "?":
        return names.last
    if " forced by " in lab:
        asker = lab.split(" forced by ", 1)[1]
        side, asset = side_asset(asker, names)
        return side, asset
    m = re.match(r"avatar (.+?): ", lab) or re.match(r"avatar (.+) \(Rust process\)$", lab)
    if m:
        return "avatar", f"avatar {m.group(1)} (per-frame)"
    if re.search(r"avatar_scene/|DclAvatar|decentraland_components/avatar/|nameplate|nickname|social", lab):
        what = "AnimationTree update" if lab.startswith("animation update") else lab.split(" [")[0][:70]
        return "avatar", f"all avatars: {what}"
    if "mesh_renderer.gd" in lab:
        return "scene", "SDK MeshRenderer (mesh_renderer.gd)"
    m = re.match(r"state (\w+) of (.+)$", lab)
    if m:
        return "scene", f"{m.group(2)}: {m.group(1)} state"
    if lab.startswith("scene ") or "scene_runner" in lab or "GltfContainer" in lab:
        return "scene", "scene runtime (other)"
    if names.last:
        return "?", names.last[1]
    return "engine", lab.split(" [")[0][:60]


def blocker_of(tp, utid, z, names):
    """Sleep inside an engine-thread zone, charged to what the thread that woke it was running.
    Returns (sleep_ns, (side, asset), kind) or None."""
    end = z["ts"] + z["dur"]
    st = rows(tp, f"""select sum(min(ts + dur, {end}) - max(ts, {z['ts']})) ns from thread_state
                       where utid = {utid} and ts < {end} and ts + dur > {z['ts']} and state in ('S', 'D')""")
    sleep = st[0]["ns"] if st and st[0]["ns"] else 0
    if sleep < 2e6:
        return None
    w = rows(tp, f"""select waker_utid, ts from thread_state where utid = {utid} and ts > {z['ts']} and ts <= {end}
                      and waker_utid is not null and waker_utid != {utid} order by ts desc limit 1""")
    if not w:
        return None
    stack = rows(tp, f"""select s.name, (select display_value from args a where a.arg_set_id = s.arg_set_id
                          and a.key = 'debug.text' limit 1) as text from slice s join thread_track tt on s.track_id = tt.id
                          where tt.utid = {w[0]['waker_utid']} and s.ts <= {w[0]['ts']} and s.ts + s.dur >= {w[0]['ts']} - 1000000
                          order by s.depth""")
    if not stack:
        return None
    names.last = None
    for fz in stack:
        if fz["text"] and HASH_RE.search(fz["text"]):
            names.asset(fz["text"])
    if not names.last:
        return None
    leaf = stack[-1]["name"]
    kind = "blocked by worker: " + ("MESH pipeline compile" if leaf in ("RenderForwardMobile::mesh_generate_pipelines",
            "SceneShaderForwardMobile::_create_pipeline", "RenderingDevice::render_pipeline_create") else leaf)
    side, asset = names.last
    return sleep, (side if side != "?" else "?", asset), kind


def write_ledger(args, tp, track, t_min, all_z, child_sum, label_of, names):
    frames = rows(tp, f"select ts, dur from slice where name = 'Main::iteration' and track_id = {track} and ts >= {t_min} order by ts")
    utid = rows(tp, f"select utid from thread_track where id = {track}")[0]["utid"]
    hic_ns = args.hiccup_ms * 1e6
    led = defaultdict(lambda: [0, 0, Counter(), 0])  # (side, asset) -> [total, in hiccups, kinds, frames touched]
    fi, last_frame = 0, {}
    for z in all_z:
        while fi < len(frames) and z["ts"] >= frames[fi]["ts"] + frames[fi]["dur"]:
            fi += 1
        if fi >= len(frames):
            break
        f = frames[fi]
        if z["ts"] < f["ts"]:
            continue
        self_ns = z["dur"] - child_sum.get(z["id"], 0)
        if self_ns <= 0:
            continue
        names.last = None
        lab = label_of(z)
        parts = [(side_asset(lab, names), kind_of(lab), self_ns)]
        if self_ns >= 5e6:
            b = blocker_of(tp, utid, z, names)
            if b:
                moved = min(b[0], self_ns)
                parts = [(parts[0][0], parts[0][1], self_ns - moved), (b[1], b[2], moved)]
        for key, kind, ns in parts:
            if ns <= 0:
                continue
            e = led[key]
            e[0] += ns
            if f["dur"] >= hic_ns:
                e[1] += ns
            e[2][kind] += ns
            if last_frame.get(key) != fi:
                last_frame[key] = fi
                e[3] += 1
    window = sum(f["dur"] for f in frames)
    hic_total = sum(f["dur"] for f in frames if f["dur"] >= hic_ns)
    sides = Counter()
    sides_h = Counter()
    for (side, _), e in led.items():
        sides[side] += e[0]
        sides_h[side] += e[1]
    out = [f"engine-thread self time per asset after {args.after_marker or 'start'}: {len(frames)} frames, "
           f"{window / 1e9:.1f}s; frames >= {args.hiccup_ms:.0f}ms: "
           f"{sum(1 for f in frames if f['dur'] >= hic_ns)} = {hic_total / 1e9:.1f}s", "",
           "== by side            total     in hiccup frames"]
    for side, v in sides.most_common():
        out.append(f"   {side:10s} {v / 1e9:9.2f}s  {sides_h[side] / 1e9:9.2f}s")
    for side in ("scene", "avatar", "?", "engine"):
        items = sorted(((k, e) for k, e in led.items() if k[0] == side), key=lambda kv: -kv[1][1] - kv[1][0] * 0.1)
        if not items:
            continue
        out.append(f"\n== {side}: per asset (total / in hiccup frames / frames touched / top kinds)")
        for (s_, asset), e in items[:60]:
            kinds = ", ".join(f"{k} {v / 1e6:.0f}ms" for k, v in e[2].most_common(3))
            out.append(f"   {e[0] / 1e6:8.1f}ms {e[1] / 1e6:8.1f}ms {e[3]:5d}  {asset}  [{kinds}]")
    with open(args.ledger, "w") as fh:
        fh.write("\n".join(out) + "\n")
    import json
    with open(args.ledger + ".json", "w") as fh:
        json.dump({"window_s": window / 1e9, "frames": len(frames), "hiccup_ms": args.hiccup_ms,
                   "hiccup_frames": sum(1 for f in frames if f["dur"] >= hic_ns), "hiccup_s": hic_total / 1e9,
                   "rows": [{"side": k[0], "asset": k[1], "total_ms": round(e[0] / 1e6, 2), "hiccup_ms": round(e[1] / 1e6, 2),
                             "frames": e[3], "kinds": [[kk, round(v / 1e6, 2)] for kk, v in e[2].most_common()]}
                            for k, e in led.items() if e[0] >= 1e6]}, fh)
    print(f"ledger: {args.ledger}")
    print("\n".join(out[:8]))


if __name__ == "__main__":
    main()
