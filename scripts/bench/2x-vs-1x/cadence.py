#!/usr/bin/env python3
"""Cadence structure of one run, from its frames.csv (#3206).

    cadence.py docs/bench/data/.../frames.csv          # human table
    cadence.py docs/bench/data/.../frames.csv --json   # one JSON object

Frame pacing is reported as *cadence*, not mean fps: the panel is vsync-locked, so a run sits on an
integer number of vsync periods and the interesting quantity is how much of the window falls outside
that cadence. `--vsync-hz` is the panel the run was pinned to; without it a 60 Hz run would be read
as if it were 120 Hz and every period count would come out doubled.

Reads only frames.csv (one row per presented frame). The first row has no predecessor, so
trace_processor writes `[NULL]` there; a frames.csv with only a header (the soak run) has no
intervals at all. Both are reported, neither is an error.
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import sys
from collections import Counter
from pathlib import Path

NULL = "[NULL]"
ON_MODAL_TOLERANCE = 0.25  # periods


def read_frames(path: Path) -> list[dict]:
    """Every data row of frames.csv; a header-only file yields []."""
    with path.open(newline="") as fh:
        return list(csv.DictReader(fh))


def read_intervals(rows: list[dict]) -> list[float]:
    """Intervals in ms, dropping the NULL sentinel and non-positive gaps."""
    out = []
    for row in rows:
        raw = (row.get("interval_ms") or "").strip()
        if not raw or raw == NULL:
            continue
        try:
            value = float(raw)
        except ValueError:
            continue
        if value > 0:
            out.append(value)
    return out


def percentile(values: list[float], pct: float) -> float | None:
    """Linear-interpolation percentile, as trace_processor's `percentile()` computes it."""
    if not values:
        return None
    ordered = sorted(values)
    pos = (len(ordered) - 1) * pct / 100.0
    low = math.floor(pos)
    high = min(low + 1, len(ordered) - 1)
    return ordered[low] + (ordered[high] - ordered[low]) * (pos - low)


def span_seconds(rows: list[dict]) -> float | None:
    stamps = []
    for row in rows:
        try:
            stamps.append(int(row["ts"]))
        except (KeyError, TypeError, ValueError):
            continue
    if len(stamps) < 2:
        return None
    return (max(stamps) - min(stamps)) / 1e9


def cadence(frame_path: Path, vsync_hz: float = 120.0) -> dict:
    rows = read_frames(frame_path)
    intervals = read_intervals(rows)
    period_ms = 1000.0 / vsync_hz
    span = span_seconds(rows)
    out: dict = {
        "file": str(frame_path),
        "vsync_hz": vsync_hz,
        "vsync_ms": round(period_ms, 3),
        "frames": len(rows),
        "intervals": len(intervals),
        "span_s": round(span, 3) if span is not None else None,
        "jank_type_counts": dict(Counter((r.get("jank_type") or "").strip() for r in rows)),
    }
    # fps over the observed span: meta.json's fps_avg divides the frame count by the *nominal* trace
    # wall instead, which is a different (and slightly larger) base for the same run.
    if span:
        out["fps_avg"] = round(len(intervals) / span, 2)
    else:
        out["fps_avg"] = None
    for name, pct in (("interval_p50_ms", 50), ("interval_p95_ms", 95), ("interval_p99_ms", 99)):
        value = percentile(intervals, pct)
        out[name] = round(value, 2) if value is not None else None
    out["interval_max_ms"] = round(max(intervals), 2) if intervals else None
    for name, pct in (("fps_median", 50), ("fps_p5_low", 95), ("fps_p1_low", 99)):
        value = percentile(intervals, pct)
        out[name] = round(1000.0 / value, 2) if value else None

    if not intervals:
        out.update(
            modal_periods=None,
            modal_interval_ms=None,
            pct_frames_on_modal=None,
            hiccups_ge1=0,
            hiccups_ge1_per_min=None,
            stalls_ge2=0,
            stalls_ge2_per_min=None,
            pct_window_in_stalls=None,
            stall_window_s=None,
        )
        return out

    periods = [ms / period_ms for ms in intervals]
    modal_periods = Counter(round(p) for p in periods).most_common(1)[0][0]
    on_modal = sum(1 for p in periods if abs(p - modal_periods) <= ON_MODAL_TOLERANCE)
    hiccups = sum(1 for p in periods if modal_periods + 1 <= p < modal_periods + 2)
    stalls = [ms for p, ms in zip(periods, intervals) if p >= modal_periods + 2]
    span_min = span / 60.0 if span else None
    out.update(
        modal_periods=modal_periods,
        modal_interval_ms=round(modal_periods * period_ms, 2),
        pct_frames_on_modal=round(on_modal / len(periods), 3),
        hiccups_ge1=hiccups,
        hiccups_ge1_per_min=round(hiccups / span_min, 2) if span_min else None,
        stalls_ge2=len(stalls),
        stalls_ge2_per_min=round(len(stalls) / span_min, 2) if span_min else None,
        # share of window *time* outside the cadence by >= 2 periods: scale-free across arms that
        # sit on different modal cadences.
        pct_window_in_stalls=round(sum(stalls) / sum(intervals), 4),
        stall_window_s=round(sum(stalls) / 1000.0, 3),
    )
    return out


def fmt(value: object, width: int = 10) -> str:
    if value is None:
        return "missing".rjust(width)
    if isinstance(value, float):
        return f"{value:.3f}".rjust(width)
    if isinstance(value, int):
        return str(value).rjust(width)
    return str(value).rjust(width)


def print_table(summary: dict) -> None:
    print(f"{summary['file']}")
    print(f"  vsync {summary['vsync_hz']} Hz ({summary['vsync_ms']} ms per period)")
    rows = [
        ("frames", summary["frames"]),
        ("intervals", summary["intervals"]),
        ("span_s", summary["span_s"]),
        ("fps_avg (frames/span)", summary["fps_avg"]),
        ("fps_median", summary["fps_median"]),
        ("fps_p5_low", summary["fps_p5_low"]),
        ("fps_p1_low", summary["fps_p1_low"]),
        ("interval_p50_ms", summary["interval_p50_ms"]),
        ("interval_p95_ms", summary["interval_p95_ms"]),
        ("interval_p99_ms", summary["interval_p99_ms"]),
        ("interval_max_ms", summary["interval_max_ms"]),
        ("modal_periods", summary["modal_periods"]),
        ("modal_interval_ms", summary["modal_interval_ms"]),
        ("pct_frames_on_modal", summary["pct_frames_on_modal"]),
        ("hiccups_ge1", summary["hiccups_ge1"]),
        ("hiccups_ge1_per_min", summary["hiccups_ge1_per_min"]),
        ("stalls_ge2", summary["stalls_ge2"]),
        ("stalls_ge2_per_min", summary["stalls_ge2_per_min"]),
        ("pct_window_in_stalls", summary["pct_window_in_stalls"]),
        ("stall_window_s", summary["stall_window_s"]),
    ]
    for name, value in rows:
        print(f"  {name:<24}{fmt(value, 12)}")
    counts = summary["jank_type_counts"]
    if counts:
        print("  jank_type_counts")
        for name in sorted(counts, key=lambda k: (-counts[k], k)):
            print(f"    {name or '(empty)':<44}{fmt(counts[name], 6)}")
    if not summary["intervals"]:
        print("  (no intervals: header-only or empty frames.csv)")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Cadence metrics for one run, from its frames.csv",
        usage="cadence.py FILE [--vsync-hz 120] [--json]",
    )
    parser.add_argument("file", type=Path, help="frames.csv of one run")
    parser.add_argument("--vsync-hz", type=float, default=120.0, help="pinned panel rate (default 120)")
    parser.add_argument("--json", action="store_true", help="print one JSON object instead of a table")
    args = parser.parse_args(argv)

    if not args.file.is_file():
        print(f"cadence.py: no such file: {args.file}", file=sys.stderr)
        return 2
    if args.vsync_hz <= 0:
        print("cadence.py: --vsync-hz must be positive", file=sys.stderr)
        return 2

    summary = cadence(args.file, args.vsync_hz)
    if args.json:
        print(json.dumps(summary))
    else:
        print_table(summary)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
