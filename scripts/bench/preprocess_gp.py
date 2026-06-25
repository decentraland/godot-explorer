#!/usr/bin/env python3
"""Trigger asset-processor preprocessing for every Genesis Plaza scene
exposed by the local GP preview server, against the asset processor running
on localhost:8080.

Flow:
  1. GET {preview}/about/0,0 -> follow scenesUrn / pointers to discover the
     full set of GP parcels and their scene IDs (the local preview returns
     b64-... scene IDs that map to local file paths in dcl-bench cache).
  2. For each unique scene ID, POST /process-scene to the asset server with
     content_base_url pointing at the preview server.
  3. Poll /status/{batch_id} until all jobs complete.
  4. Print one summary line per scene: tag, jobs done/failed, host_zips
     count.

The asset processor writes per-asset .zip files to /output (bind-mounted at
~/godot-asset-output on host). The python static server at :9090 serves them
back to the phone.
"""
import json
import sys
import time
import argparse
from urllib.request import Request, urlopen
from urllib.error import URLError, HTTPError


def http_json(url, method="GET", body=None, timeout=60):
    data = json.dumps(body).encode() if body is not None else None
    req = Request(url, data=data, method=method)
    if data is not None:
        req.add_header("Content-Type", "application/json")
    with urlopen(req, timeout=timeout) as r:
        return json.loads(r.read().decode())


def discover_scenes(preview_base):
    """Walk the GP preview's active-entities endpoint and return a
    {scene_id: pointers[]} dict for every distinct scene returned by the
    canonical GP parcel set.
    """
    # GP parcels per the live catalog: this is the worst-case bounding set,
    # the preview will return a subset of distinct entity IDs that actually
    # cover the cached scene files.
    gp_pointers = []
    for x in range(-3, 4):
        for y in range(-4, 10):
            gp_pointers.append(f"{x},{y}")
    body = {"pointers": gp_pointers}
    url = f"{preview_base}/content/entities/active"
    entries = http_json(url, method="POST", body=body)
    out = {}
    for e in entries:
        sid = e.get("id")
        if sid and sid not in out:
            out[sid] = e.get("pointers", [])
    return out


def submit_scene(asset_server, scene_id, content_base_url, output_hash=None):
    body = {
        "scene_hash": scene_id,
        "content_base_url": content_base_url,
        "cache_only": False,
    }
    if output_hash:
        body["output_hash"] = output_hash
    r = http_json(f"{asset_server}/process-scene", method="POST", body=body)
    return r


def poll_batch(asset_server, batch_id, scene_id, every=2.0, deadline_s=600):
    t0 = time.time()
    last_done = -1
    while time.time() - t0 < deadline_s:
        try:
            s = http_json(f"{asset_server}/status/{batch_id}", timeout=10)
        except (URLError, HTTPError) as e:
            print(f"  status fetch failed: {e}", flush=True)
            time.sleep(every)
            continue
        jobs = s.get("jobs", [])
        done = sum(1 for j in jobs if j.get("status") in ("completed", "failed"))
        total = len(jobs)
        unfin = total - done
        host_zips = len(s.get("individual_zips", []))
        bs = s.get("status", "?")
        if done != last_done:
            print(
                f"  scene={scene_id[:32]}... batch={batch_id[:12]} status={bs} done={done}/{total} unfin={unfin} host_zips={host_zips}",
                flush=True,
            )
            last_done = done
        if bs in ("completed", "failed"):
            failed = sum(1 for j in jobs if j.get("status") == "failed")
            return {"status": bs, "done": done, "total": total, "failed": failed, "host_zips": host_zips}
        time.sleep(every)
    return {"status": "timeout", "done": last_done, "total": -1, "failed": -1, "host_zips": -1}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--preview", default="http://127.0.0.1:8000", help="GP preview base URL")
    ap.add_argument("--asset-server", default="http://127.0.0.1:8080", help="Asset processor URL")
    ap.add_argument("--scene", action="append", default=[], help="Process only this scene id (repeatable)")
    ap.add_argument("--deadline", type=int, default=900, help="Per-scene poll deadline (seconds)")
    ap.add_argument(
        "--container-preview",
        default="http://host.docker.internal:8000",
        help="GP preview URL as reachable from inside the asset-processor container (default: host.docker.internal:8000)",
    )
    args = ap.parse_args()

    content_base = args.container_preview.rstrip("/") + "/content/contents/"
    print(f"[preprocess] preview={args.preview} processor={args.asset_server}", flush=True)
    print(f"[preprocess] content_base_url={content_base}", flush=True)

    if args.scene:
        scenes = {s: [] for s in args.scene}
    else:
        scenes = discover_scenes(args.preview)
    print(f"[preprocess] {len(scenes)} distinct scene IDs to process", flush=True)
    for sid in sorted(scenes.keys()):
        print(f"  - {sid}", flush=True)

    results = {}
    total_zips = 0
    for sid in sorted(scenes.keys()):
        print(f"[preprocess] submit {sid[:40]}...", flush=True)
        try:
            r = submit_scene(args.asset_server, sid, content_base)
        except (HTTPError, URLError) as e:
            print(f"  submit failed: {e}", flush=True)
            results[sid] = {"status": "submit_failed"}
            continue
        batch_id = r.get("batch_id") or r.get("job_id") or r.get("output_hash") or sid
        print(f"  batch={batch_id} total_assets={r.get('total_assets', '?')}", flush=True)
        out = poll_batch(args.asset_server, batch_id, sid, deadline_s=args.deadline)
        results[sid] = out
        total_zips += max(0, out.get("host_zips", 0))
        print(
            f"  -> {out['status']} done={out['done']}/{out['total']} failed={out['failed']} host_zips={out['host_zips']}",
            flush=True,
        )

    ok = sum(1 for v in results.values() if v.get("status") == "completed")
    failed_scenes = sum(1 for v in results.values() if v.get("status") not in ("completed",))
    print(
        f"[preprocess] DONE scenes_ok={ok} scenes_failed={failed_scenes} total_host_zips={total_zips}",
        flush=True,
    )
    return 0 if failed_scenes == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
