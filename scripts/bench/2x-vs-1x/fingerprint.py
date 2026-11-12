#!/usr/bin/env python3
"""Writes the state a run was measured in, and says whether that state is the one that was asked for.

A window is only usable if the engine was on the profile the run claims, in the scene and position the
run claims, and the phone was not being throttled. Those are read back from the engine's own
[RenderReady] line and from the platform, after the window, and written next to the frames, so a table
cannot be built from a run that silently drifted. Screen brightness is deliberately not a check: it
follows the camera angle and where the avatar stands, so an identical scene reads 95 or 144 depending on
where the player looks.

The engine's own frame counter is recorded as well: it is a second, independent fps source, and when the
two disagree the window is flagged instead of averaged in.

    fingerprint.py --dir DIR --label L --build 2x [--expect-profile 3]
                   [--expect-scene bafkrei…] [--expect-position 0,-1] [--max-thermal-status 0]

Writes DIR/state.json always, and DIR/INVALID.md with the reasons when the window is not usable.
Exit code 0 for a usable window, 2 for an unusable one, so a shell runner can stop on it.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from datetime import datetime, timezone

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)


def sh(args: list[str]) -> str:
    out = subprocess.run(args, capture_output=True, text=True, timeout=120)
    return out.stdout


def serial() -> str:
    return os.environ.get("ANDROID_SERIAL") or "R5CW60624NE"


def device_temps(s: str) -> dict:
    text = sh(["adb", "-s", s, "shell", "dumpsys", "thermalservice"])
    def one(name: str):
        m = re.search(rf"mValue=([0-9.]+), mType=[0-9]+, mName={name}\b", text)
        return float(m.group(1)) if m else None
    status = re.search(r"Thermal Status: *([0-9]+)", text)
    return {"thermal_status": int(status.group(1)) if status else None,
            "ap_c": one("AP"), "skin_c": one("SKIN")}


def engine_state(s: str, pid: str) -> dict:
    """The engine's own counters, straight from its log line for the last frame sync."""
    log = sh(["adb", "-s", s, "logcat", "-d", "-t", "20000"])
    lines = [ln for ln in log.splitlines() if pid and f" {pid} " in ln and "godot" in ln]
    state = {"profile": None, "profile_changes": 0, "engine_fps": None,
             "rt_p95_us": None, "frames_missed": None, "blocked": None, "scenes": None,
             "scene_id": None, "position": None, "shadow_quality": None, "bloom_quality": None}
    for ln in lines:
        if "[RenderReady]" in ln and "scene_id" in ln:
            try:
                payload = json.loads(ln[ln.index("[RenderReady]") + len("[RenderReady]"):])
            except ValueError:
                payload = None
            if payload:
                state["scene_id"] = payload.get("scene_id")
                state["position"] = payload.get("position")
                state["shadow_quality"] = payload.get("shadow_quality")
                state["bloom_quality"] = payload.get("bloom_quality")
                if payload.get("profile") is not None:
                    state["profile"] = int(payload["profile"])
        for m in re.finditer(r'"profile":(\d+)', ln):
            state["profile"] = int(m.group(1))
        if "Profile changed" in ln:
            state["profile_changes"] += 1
        m = re.search(r"frame sync: frames=\d+ fps=([0-9.]+)", ln)
        if m:
            state["engine_fps"] = float(m.group(1))
            m2 = re.search(r"rt_p95=(\d+)us", ln)
            if m2:
                state["rt_p95_us"] = int(m2.group(1))
            m3 = re.search(r"frames_missed=(\d+)", ln)
            if m3:
                state["frames_missed"] = int(m3.group(1))
            m4 = re.search(r"blocked=(\d+)", ln)
            if m4:
                state["blocked"] = int(m4.group(1))
            m5 = re.search(r"scenes=(\d+)", ln)
            if m5:
                state["scenes"] = int(m5.group(1))
    return state


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--dir", required=True)
    ap.add_argument("--label", required=True)
    ap.add_argument("--build", required=True)
    ap.add_argument("--expect-profile", type=int, default=None)
    ap.add_argument("--expect-scene", default=None)
    ap.add_argument("--expect-position", default=None)
    ap.add_argument("--max-thermal-status", type=int, default=None)
    args = ap.parse_args()

    s = serial()
    pid = sh(["adb", "-s", s, "shell", "pidof", "org.decentraland.godotexplorer"]).strip()
    state = {
        "label": args.label,
        "build": args.build,
        "utc": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "pid": pid or None,
        "package": "org.decentraland.godotexplorer",
        "version_name": sh(["adb", "-s", s, "shell", "dumpsys", "package", "org.decentraland.godotexplorer"])
        .split("versionName=")[-1].split()[0] if pid else None,
    }
    state.update(device_temps(s))
    state.update(engine_state(s, pid))
    for name in ("launch.json", "launch.log"):
        path = os.path.join(args.dir, name)
        if state["profile"] is not None or not os.path.exists(path):
            continue
        text = open(path, errors="replace").read()
        found = re.findall(r'"profile":\s*(\d+)', text)
        if found:
            state["profile"] = int(found[-1])
            state["profile_source"] = name

    meta_path = os.path.join(args.dir, "meta.json")
    if os.path.exists(meta_path):
        frames = (json.load(open(meta_path)).get("frames") or {})
        state["window_fps"] = float(frames["fps_avg"]) if frames.get("fps_avg") else None
        state["window_frames"] = int(frames["frames"]) if frames.get("frames") else None
        state["interval_p99_ms"] = float(frames["interval_p99_ms"]) if frames.get("interval_p99_ms") else None
    else:
        state["window_fps"] = None

    reasons = []
    if args.expect_profile is not None:
        if state["profile"] is None:
            reasons.append("engine never printed a profile for this pid")
        elif state["profile"] != args.expect_profile:
            reasons.append(f"engine profile {state['profile']} != expected {args.expect_profile}")
        if state["profile_changes"]:
            reasons.append(f"profile changed {state['profile_changes']}x inside the window")
    if args.max_thermal_status is not None and state["thermal_status"] is not None:
        if state["thermal_status"] > args.max_thermal_status:
            reasons.append(f"thermal status {state['thermal_status']} > {args.max_thermal_status}")
    if args.expect_scene and state.get("scene_id") != args.expect_scene:
        reasons.append(f"scene {state.get('scene_id')} != expected {args.expect_scene}")
    if args.expect_position and state.get("position") != args.expect_position:
        reasons.append(f"position {state.get('position')} != expected {args.expect_position}")
    if state.get("window_fps") and state.get("engine_fps"):
        drift = abs(state["window_fps"] - state["engine_fps"]) / state["window_fps"]
        state["fps_source_delta_pct"] = round(100 * drift, 1)
        if drift > 0.15:
            reasons.append(f"frame timeline {state['window_fps']:.1f} fps vs engine "
                           f"{state['engine_fps']:.1f} fps ({100 * drift:.0f} % apart)")

    state["usable"] = not reasons
    state["reasons"] = reasons
    with open(os.path.join(args.dir, "state.json"), "w") as fh:
        json.dump(state, fh, indent=2)

    invalid = os.path.join(args.dir, "INVALID.md")
    if reasons:
        with open(invalid, "w") as fh:
            fh.write(f"# {args.label} is not usable\n\n")
            for r in reasons:
                fh.write(f"- {r}\n")
            fh.write(f"\nState recorded in `state.json`.\n")
        print(f"[fingerprint] INVALID: {'; '.join(reasons)}", file=sys.stderr)
        return 2
    if os.path.exists(invalid):
        os.remove(invalid)
    fps = state.get("window_fps")
    engine = state.get("engine_fps")
    print(f"[fingerprint] ok profile={state['profile']} scene={state['scene_id']} "
          f"pos={state['position']} fps={fps} engine_fps={engine} "
          f"thermal={state['thermal_status']} ap={state['ap_c']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
