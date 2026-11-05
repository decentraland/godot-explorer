#!/usr/bin/env python3
"""Aggregate the 2.x-vs-1.x runs into the tables the report and the protocol quote (#3206).

    summarize.py --root docs/bench/data/2x-vs-1x-3206/android-a54 --out docs/bench/data/...

Medians come from the per-run medians the protocol asks for (>= 5 alternating runs), never from
pooled frames: a run is the unit, otherwise a single bad run would vanish into the pool.
"""

from __future__ import annotations

import argparse
import json
import re
import statistics as st
from pathlib import Path

METRICS = [
    ("fps_avg", "fps avg", "{:.1f}", True),
    ("fps_p1_low", "fps p1 (1% low)", "{:.1f}", True),
    ("fps_p5_low", "fps p5 (5% low)", "{:.1f}", True),
    ("interval_p50_ms", "frame time p50 (ms)", "{:.1f}", False),
    ("interval_p95_ms", "frame time p95 (ms)", "{:.1f}", False),
    ("interval_p99_ms", "frame time p99 (ms)", "{:.1f}", False),
    ("interval_max_ms", "frame time max (ms)", "{:.1f}", False),
    ("rss_steady_mb", "RSS steady (MB, PSS)", "{:.0f}", False),
    ("rss_peak_mb", "RSS peak (MB, PSS)", "{:.0f}", False),
    ("render_ready_t100_ms", "scene draw-ready t100 (ms)", "{:.0f}", False),
    ("render_ready_t90_ms", "scene draw-ready t90 (ms)", "{:.0f}", False),
    ("cold_launch_to_render_ready_ms", "cold launch -> draw-ready (ms)", "{:.0f}", False),
    ("am_total_time_ms", "am start -W TotalTime (ms)", "{:.0f}", False),
    ("cpu_mean_pct", "CPU mean (% of all cores)", "{:.1f}", False),
]


def read_run(run_dir: Path) -> dict:
    meta = json.loads((run_dir / "meta.json").read_text())
    row = {
        "label": meta["label"],
        "build": meta["build"],
        "version": meta.get("version_name"),
    }
    for key, value in (meta.get("frames") or {}).items():
        try:
            row[key] = float(value)
        except (TypeError, ValueError):
            pass
    for key in ("rss_peak_kb", "rss_steady_kb", "cpu_mean_pct", "thermal_end_c"):
        if meta.get(key) is not None:
            row[key] = meta[key]
    for key in ("rss_peak_kb", "rss_steady_kb"):
        if key in row:
            row[key.replace("_kb", "_mb")] = row[key] / 1024.0
    launch = run_dir.parent / f"{meta['label']}-launch.json"
    if launch.exists():
        row.update(json.loads(launch.read_text()))
    log = run_dir.parent / f"launch-{meta['label']}.log"
    if log.exists():
        text = log.read_text(errors="replace")
        rr = re.findall(r"\[RenderReady\] (\{.*?\})\s*$", text, re.M)
        if rr:
            payload = json.loads(rr[-1])
            row["render_ready_t100_ms"] = payload.get("t100_ms")
            row["render_ready_t90_ms"] = payload.get("t90_ms")
            row["scene_id"] = payload.get("scene_id")
            row["graphics_profile"] = payload.get("profile")
            row["position"] = payload.get("position")
    return row


def medians(rows: list[dict], build: str) -> dict:
    subset = [r for r in rows if r["build"] == build]
    out = {"runs": len(subset)}
    for key, *_ in METRICS:
        values = [r[key] for r in subset if isinstance(r.get(key), (int, float))]
        out[key] = st.median(values) if values else None
    return out


def fmt(value, spec: str) -> str:
    return "n/d" if value is None else spec.format(value)


def delta(before, after, higher_is_better: bool) -> str:
    if not before or not after:
        return "n/d"
    pct = (after - before) / before * 100.0
    arrow = "better" if (pct > 0) == higher_is_better else "worse"
    return f"{pct:+.1f}% ({arrow})"


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", required=True)
    parser.add_argument("--out", default="")
    args = parser.parse_args()
    root = Path(args.root)

    sections = []
    summary = {}
    for scenario_dir in sorted(p for p in root.iterdir() if p.is_dir()):
        # A scenario whose runs broke the protocol carries an INVALID.md next to them; its numbers are
        # not results and must not reach the table.
        if (scenario_dir / "INVALID.md").exists():
            continue
        runs = [read_run(d) for d in sorted(scenario_dir.iterdir()) if (d / "meta.json").exists()]
        if not runs:
            continue
        a, b = medians(runs, "1x"), medians(runs, "2x")
        # A comparison needs both sides; a scenario measured on one build only (a soak, say) is not
        # one, and half a table reads as if the other build had been measured.
        if not (a["runs"] and b["runs"]):
            continue
        summary[scenario_dir.name] = {"1x": a, "2x": b, "runs": [r["label"] for r in runs]}
        verdict_fps = "PASS" if (b["fps_avg"] or 0) >= (a["fps_avg"] or 0) else "FAIL"
        rss_budget = (a["rss_steady_mb"] or 0) + 100
        verdict_rss = "PASS" if (b["rss_steady_mb"] or 0) <= rss_budget else "FAIL"
        lines = [
            f"## {scenario_dir.name}",
            "",
            f"Median of {a['runs']} runs (1.x) and {b['runs']} runs (2.x), alternating.",
            "",
            "| metric | 1.x | 2.x | Δ (2.x vs 1.x) |",
            "|---|---:|---:|---:|",
        ]
        for key, label, spec, higher in METRICS:
            lines.append(
                f"| {label} | {fmt(a[key], spec)} | {fmt(b[key], spec)} | {delta(a[key], b[key], higher)} |"
            )
        lines += [
            "",
            f"**Budget:** fps ≥ 1.x → **{verdict_fps}**; RSS ≤ 1.x + 100 MB "
            f"({rss_budget:.0f} MB) → **{verdict_rss}**",
            "",
        ]
        sections.append("\n".join(lines))

    text = "\n".join(sections)
    if args.out:
        Path(args.out).write_text(text + "\n")
        Path(args.out).with_suffix(".json").write_text(json.dumps(summary, indent=2) + "\n")
    print(text)


if __name__ == "__main__":
    main()
