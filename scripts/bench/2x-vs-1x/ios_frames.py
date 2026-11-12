#!/usr/bin/env python3
"""Reduce an Instruments trace to the same per-frame record the Android side produces.

iOS has a frame history: `xctrace` with the Game Performance template records
`displayed-surfaces-interval`, one row per displayed frame with its duration. That is enough for the
average and for the tail, which is what the Android bench reports, so both platforms can be put in
the same table.

    ios_frames.py --trace RUN.trace --out RUN_DIR [--label L] [--window-s 30]
"""

from __future__ import annotations

import argparse
import csv
import json
import os
import statistics as st
import subprocess
import sys
import time
import xml.etree.ElementTree as ET


def export_table(trace: str, schema: str) -> str:
    last = ""
    for attempt in range(3):
        out = subprocess.run(
            ["xcrun", "xctrace", "export", "--input", trace, "--xpath", f'//table[@schema="{schema}"]'],
            capture_output=True, text=True, timeout=900,
        )
        if out.returncode == 0 and "<trace-query-result" in out.stdout:
            return out.stdout
        last = out.stderr[-600:]
        time.sleep(5)
    print(f"[ios] no {schema} table in this trace ({last.strip()[:120]})", file=sys.stderr)
    return ""


def intervals(xml_text: str) -> list[float]:
    """Frame durations in ms, in order, from displayed-surfaces-interval."""
    try:
        root = ET.fromstring(xml_text)
    except ET.ParseError:
        return []
    rows = [[v.text for v in row] for row in root.iter("row")]
    ms = []
    for row in rows:
        dur = row[1] if len(row) > 1 else None
        if dur and dur.isdigit():
            value = int(dur) / 1e6
            if value > 0:
                ms.append(value)
    return ms


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--trace", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--label", default=None)
    ap.add_argument("--window-s", type=float, default=None)
    args = ap.parse_args()

    os.makedirs(args.out, exist_ok=True)
    table = export_table(args.trace, "displayed-surfaces-interval")
    ms = intervals(table) if table else []
    if not ms:
        print("[ios] window has no frame rows, skipping", file=sys.stderr)
        return 3

    span = sum(ms) / 1000.0
    ordered = sorted(ms)
    q = lambda p: ordered[min(len(ordered) - 1, int(p * len(ordered)))]
    meta = {
        "label": args.label or os.path.basename(args.out.rstrip("/")),
        "build": "2x",
        "platform": "ios",
        "source": "xctrace/Game Performance/displayed-surfaces-interval",
        "frames": {
            "frames": str(len(ms)),
            "fps_avg": f"{len(ms) / span:.6f}" if span else None,
            "span_s": f"{span:.6f}",
            "interval_p50_ms": f"{st.median(ms):.6f}",
            "interval_p95_ms": f"{q(0.95):.6f}",
            "interval_p99_ms": f"{q(0.99):.6f}",
            "interval_max_ms": f"{max(ms):.6f}",
            "interval_min_ms": f"{min(ms):.6f}",
        },
    }
    with open(os.path.join(args.out, "frames.csv"), "w", newline="") as fh:
        w = csv.writer(fh)
        w.writerow(["ts", "interval_ms"])
        for i, value in enumerate(ms):
            w.writerow([i, f"{value:.6f}"])
    with open(os.path.join(args.out, "meta.json"), "w") as fh:
        json.dump(meta, fh, indent=2)
    print(f"[ios] {meta['label']} frames={len(ms)} fps={meta['frames']['fps_avg']} "
          f"p50={meta['frames']['interval_p50_ms']} p99={meta['frames']['interval_p99_ms']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
