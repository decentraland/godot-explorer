#!/usr/bin/env python3
"""In-game time of day, from the host's UTC clock (#3206).

    utc_phase.py                        # current phase and the seconds until it comes back
    utc_phase.py --phase 43200          # seconds until the next in-game noon
    utc_phase.py --wait-until 43200     # block until the phase is noon, then print what was hit
    utc_phase.py --wait-until 43200 --json

The world clock is a pure function of UTC while no scene overrides it (scene-free parcel, so the
setting governs): `DclGlobalTime::get_world_time()` multiplies the real clock by 86400 / (120 * 60) =
12 and wraps at 86400, so one in-game day takes 7200 real seconds and the sun repeats exactly every
7200 s of UTC (lib/crates/dcl_engine/src/godot_classes/dcl_global_time.rs:10-27).

Two arms started in the same phase therefore see the same sun to within `12 * tolerance` in-game
seconds. The tolerance is in *in-game* seconds; at 12x it is a 0.17 s real window for the default 2.
"""

from __future__ import annotations

import argparse
import json
import sys
import time

SPEED = 12.0  # in-game seconds per real second (86400 in-game s / 7200 real s)
DAY = 86400.0
PERIOD_S = DAY / SPEED  # 7200 s of UTC


def phase_at(epoch: float) -> float:
    """In-game seconds since in-game midnight at the given UTC epoch."""
    return (epoch * SPEED) % DAY


def clock(phase: float) -> str:
    whole = int(phase) % 86400
    return f"{whole // 3600:02d}:{whole % 3600 // 60:02d}:{whole % 60:02d}"


def next_epoch_for(target: float, now: float) -> float:
    """First UTC epoch >= now whose phase equals `target`."""
    period = int((now * SPEED - target) // DAY)
    candidate = (period * DAY + target) / SPEED
    return candidate if candidate >= now else candidate + PERIOD_S


def record(target: float | None, now: float, tolerance: float) -> dict:
    current = phase_at(now)
    out = {
        "utc_epoch": round(now, 3),
        "utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(now)),
        "world_phase_s": round(current, 3),
        "world_clock": clock(current),
        "period_s": round(PERIOD_S, 1),
        "phase_target_s": target,
        "seconds_until_target": None,
        "within_tolerance": None,
        "tolerance_s": tolerance,
    }
    if target is not None:
        out["seconds_until_target"] = round(next_epoch_for(target, now) - now, 3)
        out["within_tolerance"] = abs(current - target) <= tolerance
    return out


def wait_until(target: float, tolerance: float) -> dict:
    started = time.time()
    while True:
        now = time.time()
        if abs(phase_at(now) - target) <= tolerance:
            out = record(target, now, tolerance)
            out["waited_s"] = round(now - started, 3)
            out["achieved_phase_s"] = round(phase_at(now), 3)
            return out
        remaining = next_epoch_for(target, now) - now
        # 12x: sleeping the whole remaining time lands on the target, but the last chunk has to be
        # short enough that scheduler overshoot stays inside the tolerance.
        time.sleep(min(remaining, 0.05) if remaining < 2.0 else min(remaining, 5.0))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="In-game time of day and the next matching phase (7200 s period)",
        usage="utc_phase.py [--phase S | --wait-until S] [--tolerance 2] [--json]",
    )
    target = parser.add_mutually_exclusive_group()
    target.add_argument("--phase", type=float, help="report seconds until this in-game phase")
    target.add_argument(
        "--wait-until", type=float, metavar="S", help="block until the phase is S in-game seconds"
    )
    parser.add_argument("--tolerance", type=float, default=2.0, help="in-game seconds (default 2)")
    parser.add_argument("--json", action="store_true", help="print one JSON object")
    args = parser.parse_args(argv)

    if args.tolerance <= 0:
        print("utc_phase.py: --tolerance must be positive", file=sys.stderr)
        return 2
    for label, value in (("--phase", args.phase), ("--wait-until", args.wait_until)):
        if value is not None and not 0 <= value < DAY:
            print(f"utc_phase.py: {label} must be in [0, 86400)", file=sys.stderr)
            return 2

    if args.wait_until is not None:
        out = wait_until(args.wait_until, args.tolerance)
        if not out["within_tolerance"]:
            print(
                f"utc_phase.py: phase {out['achieved_phase_s']:.1f} missed target "
                f"{args.wait_until} by more than {args.tolerance} in-game s",
                file=sys.stderr,
            )
            return 2
    else:
        out = record(args.phase, time.time(), args.tolerance)

    if args.json:
        print(json.dumps(out))
        return 0

    print(f"in-game time {out['world_clock']}  (phase {out['world_phase_s']:.1f} of 86400 s)")
    print(f"utc {out['utc']}  period {out['period_s']:.0f} s")
    if out["phase_target_s"] is None:
        print(f"the phase repeats every {out['period_s']:.0f} s")
    else:
        state = "inside" if out["within_tolerance"] else "outside"
        print(
            f"target phase {out['phase_target_s']:.0f} ({clock(out['phase_target_s'])}): "
            f"{state} tolerance {out['tolerance_s']:.0f} in-game s, "
            f"next in {out['seconds_until_target']:.3f} s"
        )
    if "achieved_phase_s" in out:
        print(
            f"reached phase {out['achieved_phase_s']:.2f} ({clock(out['achieved_phase_s'])}) "
            f"after {out['waited_s']:.3f} s"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
