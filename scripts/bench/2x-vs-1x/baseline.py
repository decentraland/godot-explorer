#!/usr/bin/env python3
"""Re-runnable performance baseline for 2.x.

Two commands, one file format:

    baseline.py run     --out base/2x-<version>.json [--windows 6] [--scene ...] [--position ...]
    baseline.py compare --baseline base/2x-<version>.json [--windows 6]

`run` measures the build that is installed right now and writes a baseline: the per-window numbers,
the state each window was measured in, and the median the future runs are compared against.
`compare` measures again and reports the difference against that baseline with a bootstrap interval
and an exact Mann-Whitney p, so a later 2.x build can be judged against this one without re-running
anything else.

The window itself is the same one the #3206 report used: a 30 s Perfetto frame-timeline capture on a
device already in the scene, gated on the platform's thermal status, with the engine's own profile,
scene id and position read back afterwards (fingerprint.py). Nothing here needs the phone to be
touched between windows.

    # one-time: put the app in the scene at the fixed hour, then
    PROFILE=3 SCENE=bafkrei... POSITION=2,3 python3 scripts/bench/2x-vs-1x/baseline.py run --out base/2x.json
"""

from __future__ import annotations

import argparse
import json
import os
import random
import statistics as st
import subprocess
import sys
import time
from datetime import datetime, timezone

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
DATA = os.path.join(ROOT, "docs", "bench", "data", "2x-vs-1x-3206", "android-a54")
SERIAL = os.environ.get("ANDROID_SERIAL", "R5CW60624NE")
os.environ["ANDROID_SERIAL"] = SERIAL
PKG = "org.decentraland.godotexplorer"


def adb(*args: str) -> str:
    out = subprocess.run(["adb", "-s", SERIAL, *args], capture_output=True, text=True, timeout=300)
    return out.stdout


def thermal_status() -> int | None:
    text = adb("shell", "dumpsys", "thermalservice")
    for line in text.splitlines():
        if "Thermal Status:" in line:
            try:
                return int(line.split(":")[1])
            except (IndexError, ValueError):
                return None
    return None


def wait_for_thermal(max_status: int, timeout_s: int = 900) -> None:
    start = time.time()
    while True:
        status = thermal_status()
        if status is not None and status <= max_status:
            return
        if time.time() - start > timeout_s:
            raise RuntimeError(f"thermal status stayed above {max_status}")
        time.sleep(5)


def measure_window(label: str, out_dir: str, seconds: int, expect: dict) -> dict:
    os.makedirs(out_dir, exist_ok=True)
    subprocess.run(
        [sys.executable, os.path.join(HERE, "bench_android.py"), "window",
         "--label", label, "--build", "2x", "--scenario", os.path.basename(os.path.dirname(out_dir)),
         "--seconds", str(seconds), "--out", out_dir],
        check=True, timeout=1800,
    )
    args = [sys.executable, os.path.join(HERE, "fingerprint.py"), "--dir", out_dir, "--label", label,
            "--build", "2x", "--max-thermal-status", str(expect["max_thermal_status"])]
    if expect.get("profile") is not None:
        args += ["--expect-profile", str(expect["profile"])]
    if expect.get("scene"):
        args += ["--expect-scene", expect["scene"]]
    if expect.get("position"):
        args += ["--expect-position", expect["position"]]
    check = subprocess.run(args, capture_output=True, text=True, timeout=900)
    state = json.load(open(os.path.join(out_dir, "state.json")))
    meta = json.load(open(os.path.join(out_dir, "meta.json")))
    return {"label": label, "dir": out_dir, "state": state, "frames": meta.get("frames") or {},
            "usable": check.returncode == 0}


def collect(windows: list[dict]) -> dict:
    fps = [float(w["frames"]["fps_avg"]) for w in windows if w["usable"] and w["frames"].get("fps_avg")]
    p99 = [float(w["frames"]["interval_p99_ms"]) for w in windows if w["usable"] and w["frames"].get("interval_p99_ms")]
    if not fps:
        raise RuntimeError("no usable window, nothing to record")
    return {
        "windows": len(fps),
        "fps_median": round(st.median(fps), 3),
        "fps_min": round(min(fps), 3),
        "fps_max": round(max(fps), 3),
        "p99_ms_median": round(st.median(p99), 3) if p99 else None,
        "fps_per_window": [round(x, 3) for x in fps],
    }


def cmd_run(args: argparse.Namespace) -> int:
    version = adb("shell", "dumpsys", "package", PKG).split("versionName=")[-1].split()[0]
    expect = {"profile": args.profile, "scene": args.scene, "position": args.position,
              "max_thermal_status": args.max_thermal_status}
    scene_dir = os.path.join(DATA, args.scenario)
    windows = []
    for i in range(args.windows):
        wait_for_thermal(args.max_thermal_status)
        label = f"2x-{args.prefix}-r{i}"
        print(f"[baseline] window {i + 1}/{args.windows}: {label}")
        windows.append(measure_window(label, os.path.join(scene_dir, label), args.seconds, expect))

    usable = [w for w in windows if w["usable"]]
    baseline = {
        "created_utc": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "build": "2x",
        "version_name": version,
        "device": adb("shell", "getprop", "ro.product.model").strip(),
        "serial": SERIAL,
        "scenario": args.scenario,
        "window_seconds": args.seconds,
        "condition": {"profile": args.profile, "scene": args.scene, "position": args.position,
                      "max_thermal_status": args.max_thermal_status,
                      "note": "settings are whatever the app had when the run started; they are read "
                              "back per window and a window is discarded when they differ"},
        "result": collect(usable),
        "discarded": [{"label": w["label"], "reasons": w["state"].get("reasons")}
                      for w in windows if not w["usable"]],
        "windows": [{"label": w["label"], "frames": w["frames"], "state": w["state"]} for w in windows],
    }
    os.makedirs(os.path.dirname(os.path.abspath(args.out)) or ".", exist_ok=True)
    json.dump(baseline, open(args.out, "w"), indent=2)
    print(f"[baseline] wrote {args.out}: median {baseline['result']['fps_median']} fps over "
          f"{baseline['result']['windows']} windows ({len(baseline['discarded'])} discarded)")
    return 0


def cmd_compare(args: argparse.Namespace) -> int:
    base = json.load(open(args.baseline))
    args.scenario = base["scenario"]
    args.prefix = "cmp"
    args.seconds = base["window_seconds"]
    cond = base["condition"]
    args.profile = args.profile if args.profile is not None else cond.get("profile")
    args.scene = args.scene or cond.get("scene")
    args.position = args.position or cond.get("position")
    args.max_thermal_status = args.max_thermal_status if args.max_thermal_status is not None \
        else cond.get("max_thermal_status", 1)

    tmp = os.path.join(DATA, f"cmp-{int(time.time())}")
    expect = {"profile": args.profile, "scene": args.scene, "position": args.position,
              "max_thermal_status": args.max_thermal_status}
    windows = []
    for i in range(args.windows):
        wait_for_thermal(args.max_thermal_status)
        label = f"2x-cmp-r{i}"
        windows.append(measure_window(label, os.path.join(tmp, label), args.seconds, expect))
    current = collect([w for w in windows if w["usable"]])

    a = base["result"]["fps_per_window"]
    b = current["fps_per_window"]
    ma, mb = st.median(a), st.median(b)
    random.seed(7)
    boots = sorted(st.median(random.choices(b, k=len(b))) - st.median(random.choices(a, k=len(a)))
                   for _ in range(10000))
    allv = a + b
    n1, n2 = len(a), len(b)

    def rank(v: float) -> float:
        return sum(1 for x in allv if x < v) + (sum(1 for x in allv if x == v) + 1) / 2

    u1 = sum(rank(x) for x in a) - n1 * (n1 + 1) / 2
    obs = min(u1, n1 * n2 - u1)
    import itertools
    hits = total = 0
    for combo in itertools.combinations(range(n1 + n2), n1):
        u = sum(rank(allv[i]) for i in combo) - n1 * (n1 + 1) / 2
        total += 1
        hits += min(u, n1 * n2 - u) <= obs + 1e-9

    report = {
        "baseline": {"path": args.baseline, "version": base["version_name"], "median": ma, "runs": a},
        "current": {"version": current, "runs": b},
        "delta_fps": round(mb - ma, 3),
        "delta_pct": round(100 * (mb - ma) / ma, 1) if ma else None,
        "ci95_delta_fps": [round(boots[250], 3), round(boots[9750], 3)],
        "mwu_p_exact": round(hits / total, 4),
        "verdict": "regression" if boots[9750] < 0 else ("improvement" if boots[250] > 0 else "no change"),
    }
    print(json.dumps(report, indent=2))
    if args.json:
        json.dump(report, open(args.json, "w"), indent=2)
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)

    run = sub.add_parser("run", help="measure the installed 2.x build and write a baseline")
    run.add_argument("--out", required=True)
    run.add_argument("--scenario", default="2x-baseline")
    run.add_argument("--prefix", default="base")
    run.add_argument("--windows", type=int, default=6)
    run.add_argument("--seconds", type=int, default=30)
    run.add_argument("--profile", type=int, default=3)
    run.add_argument("--scene", default=None)
    run.add_argument("--position", default=None)
    run.add_argument("--max-thermal-status", type=int, default=1)
    run.set_defaults(func=cmd_run)

    cmp_ = sub.add_parser("compare", help="measure again and diff against a baseline")
    cmp_.add_argument("--baseline", required=True)
    cmp_.add_argument("--windows", type=int, default=6)
    cmp_.add_argument("--profile", type=int, default=None)
    cmp_.add_argument("--scene", default=None)
    cmp_.add_argument("--position", default=None)
    cmp_.add_argument("--max-thermal-status", type=int, default=None)
    cmp_.add_argument("--json", default=None)
    cmp_.set_defaults(func=cmd_compare)

    args = ap.parse_args()
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
