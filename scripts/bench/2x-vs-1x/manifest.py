#!/usr/bin/env python3
"""Per-run manifest for the 2.x-vs-1.x benchmark (#3206).

Every run already leaves raw artefacts (frames.csv, meta.json, screen.png, launch log). What it does not
leave is one file that says what the run was and whether it can be used, which is why every later
re-analysis has to re-derive the profile, the cadence and the light by hand. This writes that file and
then checks it against the invalidation checklist, so a run can be rejected by a reviewer without
touching the phone again.

    manifest.py build RUN_DIR [--gate gates.jsonl] [--campaign campaign.json]
    manifest.py check RUN_DIR [--gate gates.jsonl]
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

try:
    from cadence import cadence as cadence_fn
except Exception:  # pragma: no cover - the tool must still report missing pieces
    cadence_fn = None
try:
    from luma import luma as luma_fn
except Exception:  # pragma: no cover
    luma_fn = None

PROFILE_RE = re.compile(r'"profile"\s*:\s*(\d+)')


def read_json(path):
    try:
        with open(path) as fh:
            return json.load(fh)
    except FileNotFoundError:
        return None
    except json.JSONDecodeError:
        return None


def launch_logs(run_dir):
    """The run's own log, or the scenario-level one named after the label."""
    label = os.path.basename(run_dir.rstrip("/"))
    scenario = os.path.dirname(run_dir.rstrip("/"))
    out = [os.path.join(run_dir, n) for n in ("launch.log", "launch-output.log")]
    out.append(os.path.join(scenario, f"launch-{label}.log"))
    return [p for p in out if os.path.exists(p)]


def last_profile(run_dir):
    """The last [RenderReady] profile in the run's logs, plus how many times it appeared."""
    best = None
    for path in launch_logs(run_dir):
        try:
            with open(path, errors="replace") as fh:
                text = fh.read()
        except OSError:
            continue
        hits = PROFILE_RE.findall(text)
        if hits:
            best = {"source": os.path.relpath(path, run_dir), "profile": int(hits[-1]), "lines": len(hits)}
    return best


def gate_entry(gate_path, label):
    if not gate_path or not os.path.exists(gate_path):
        return None
    found = None
    with open(gate_path) as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            try:
                rec = json.loads(line)
            except json.JSONDecodeError:
                continue
            if rec.get("label") in (None, label):
                found = rec
    return found


def build(run_dir, gate_path=None, campaign_path=None):
    meta = read_json(os.path.join(run_dir, "meta.json")) or {}
    state = read_json(os.path.join(run_dir, "state.json")) or {}
    label = meta.get("label") or os.path.basename(run_dir.rstrip("/"))

    frames_csv = os.path.join(run_dir, "frames.csv")
    cad = None
    if os.path.exists(frames_csv) and cadence_fn is not None:
        try:
            from pathlib import Path as _Path
            cad = cadence_fn(_Path(frames_csv))
        except Exception as exc:  # noqa: BLE001 - recorded, not raised
            cad = {"error": str(exc)}

    screen = os.path.join(run_dir, "screen.png")
    lum = None
    if os.path.exists(screen) and luma_fn is not None:
        try:
            from pathlib import Path as _Path
            lum = luma_fn(_Path(screen))
        except Exception as exc:  # noqa: BLE001
            lum = {"error": str(exc)}

    campaign = read_json(campaign_path) if campaign_path else None
    manifest = {
        "label": label,
        "build": meta.get("build"),
        "scenario": meta.get("scenario") or os.path.basename(os.path.dirname(run_dir.rstrip("/"))),
        "version_name": meta.get("version_name") or "missing",
        "apk_sha256": (campaign or {}).get("apk_sha256", {}).get(meta.get("build"), "missing")
        if campaign
        else "missing",
        "profile": last_profile(run_dir) or "missing",
        "frames": meta.get("frames", "missing"),
        "cpu_mean_pct": meta.get("cpu_mean_pct", "missing"),
        "rss_steady_kb": meta.get("rss_steady_kb", "missing"),
        "thermal_end_c": meta.get("thermal_end_c", "missing"),
        "condition": {
            "luma_mean": state.get("luma_mean", "missing"),
            "thermal_start_c": state.get("thermal_start_c", "missing"),
            "thermal_skin_start_c": state.get("thermal_skin_start_c", "missing"),
            "note": state.get("note", ""),
        },
        "gate": gate_entry(gate_path, label),
        "cadence": cad if cad is not None else "missing",
        "luma": lum if lum is not None else "missing",
        "artefacts": {
            name: os.path.exists(os.path.join(run_dir, name))
            for name in ("meta.json", "frames.csv", "cpu.csv", "mem.csv", "thermal.csv", "screen.png",
                         "state.json", "trace.perfetto-trace")
        },
    }
    with open(os.path.join(run_dir, "manifest.json"), "w") as fh:
        json.dump(manifest, fh, indent=2)
    return manifest


def check(run_dir, gate_path=None):
    failures = []
    manifest = read_json(os.path.join(run_dir, "manifest.json"))
    if manifest is None:
        return [f"manifest.json missing or unreadable in {run_dir}"]

    frames = manifest.get("frames")
    if not isinstance(frames, dict) or not frames.get("fps_avg") or str(frames.get("fps_avg")) in "[NULL]":
        failures.append("meta.json has no usable frames.fps_avg")

    luma = manifest.get("luma")
    if luma in (None, "missing", {}):
        failures.append("screen.png missing, so there is no light record")
    elif isinstance(luma, dict) and "luma_crop" not in luma:
        failures.append("luma could not be computed from screen.png")

    profile = manifest.get("profile")
    if profile in (None, "missing"):
        failures.append("no [RenderReady] profile line for this run")
    elif profile.get("profile") != 3:
        failures.append(f"profile {profile.get('profile')} is not High (3)")
    elif profile.get("lines", 1) > 1:
        failures.append(f"{profile['lines']} [RenderReady] lines: the profile changed inside the run")

    cad = manifest.get("cadence")
    if not isinstance(cad, dict) or not cad.get("frames"):
        failures.append("frames.csv produced no cadence data")

    path = os.path.abspath(run_dir)
    stop = os.path.abspath(os.path.join(path, "..", ".."))
    cur = path
    while cur.startswith(stop):
        if os.path.exists(os.path.join(cur, "INVALID.md")):
            failures.append(f"INVALID.md at {os.path.relpath(cur, stop)}")
            break
        parent = os.path.dirname(cur)
        if parent == cur:
            break
        cur = parent

    gate = manifest.get("gate")
    if gate is None:
        failures.append("no thermal gate ledger entry")
    else:
        if not gate.get("passed"):
            failures.append("thermal gate did not pass")
        since = gate.get("since_prev_end_s")
        if since is None:
            if not gate.get("first_run"):
                failures.append("gate ledger has no since_prev_end_s and the run is not marked first_run")
        elif since < gate.get("floor_s", 480):
            failures.append(f"only {since}s since the previous window")
    return failures


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    sub = ap.add_subparsers(dest="cmd", required=True)
    b = sub.add_parser("build")
    b.add_argument("run_dir")
    b.add_argument("--gate", default=os.environ.get("LEDGER"))
    b.add_argument("--campaign", default=os.environ.get("CAMPAIGN"))
    c = sub.add_parser("check")
    c.add_argument("run_dir")
    c.add_argument("--gate", default=os.environ.get("LEDGER"))
    args = ap.parse_args()

    if args.cmd == "build":
        manifest = build(args.run_dir, args.gate, getattr(args, "campaign", None))
        print(f"[manifest] wrote {os.path.join(args.run_dir, 'manifest.json')}")
        print(f"[manifest] {manifest['label']}: fps={manifest['frames'].get('fps_avg') if isinstance(manifest['frames'], dict) else 'missing'} "
              f"profile={manifest['profile']} luma_crop={manifest['luma'].get('luma_crop') if isinstance(manifest['luma'], dict) else 'missing'}")
        return 0

    failures = check(args.run_dir, args.gate)
    if failures:
        print("[manifest] FAIL")
        for f in failures:
            print(f"  - {f}")
        return 1
    print("[manifest] PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
