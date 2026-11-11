#!/usr/bin/env python3
"""Compare two arms of the #3206 benchmark without over-claiming.

The campaign reports a median per arm with min-max, a bootstrap interval for the difference of medians,
and the exact Mann-Whitney U p-value, because at n=4-6 a t-test would be a lie. With 6 vs 6 the exact
two-sided floor of the U test is 0.0022, so a p of 0.0022 is the smallest number this data can produce and
the tool says so instead of printing 0.

    compare.py --arm 1x=DIR --arm 2x=DIR [--metric fps_avg] [--boot 10000]

Each DIR is a scenario directory of per-run directories (meta.json / frames.csv / manifest.json).
"""

from __future__ import annotations

import argparse
import itertools
import json
import os
import random
import statistics
import sys


def run_value(run_dir, metric):
    manifest = os.path.join(run_dir, "manifest.json")
    if os.path.exists(manifest):
        try:
            m = json.load(open(manifest))
            if metric == "fps_avg":
                return float(m["frames"]["fps_avg"]) if m.get("frames", {}).get("fps_avg") else None
            if metric == "pct_on_modal":
                cad = m.get("cadence") or {}
                return cad.get("pct_frames_on_modal")
            if metric in ("interval_p99_ms", "cpu_mean_pct", "rss_steady_kb"):
                if metric == "interval_p99_ms":
                    return float(m["frames"]["interval_p99_ms"]) if m.get("frames", {}).get("interval_p99_ms") else None
                return m.get(metric)
        except (ValueError, KeyError, TypeError):
            pass
    meta = os.path.join(run_dir, "meta.json")
    if os.path.exists(meta) and metric == "fps_avg":
        try:
            frames = json.load(open(meta)).get("frames") or {}
            value = frames.get("fps_avg")
            return float(value) if value and value != "[NULL]" else None
        except (ValueError, TypeError, json.JSONDecodeError):
            return None
    return None


def collect(directory, metric):
    values, labels = [], []
    for name in sorted(os.listdir(directory)):
        run = os.path.join(directory, name)
        if not os.path.isdir(run):
            continue
        value = run_value(run, metric)
        if value is not None:
            values.append(float(value))
            labels.append(name)
    return values, labels


def bootstrap_diff(a, b, resamples=10000, seed=20261111):
    rng = random.Random(seed)
    diffs = []
    for _ in range(resamples):
        sa = [rng.choice(a) for _ in a]
        sb = [rng.choice(b) for _ in b]
        diffs.append(statistics.median(sb) - statistics.median(sa))
    diffs.sort()
    lo = diffs[int(0.025 * len(diffs))]
    hi = diffs[int(0.975 * len(diffs)) - 1]
    return lo, hi


def mann_whitney_exact(a, b):
    """Two-sided exact p for small samples: enumerate every split of the pooled ranks."""
    pooled = sorted([(v, 0) for v in a] + [(v, 1) for v in b])
    # average ranks for ties
    ranks = [0.0] * len(pooled)
    i = 0
    while i < len(pooled):
        j = i
        while j + 1 < len(pooled) and pooled[j + 1][0] == pooled[i][0]:
            j += 1
        avg = (i + j) / 2.0 + 1.0
        for k in range(i, j + 1):
            ranks[k] = avg
        i = j + 1
    n_a, n_b = len(a), len(b)
    u_obs = sum(ranks[i] for i, (_, g) in enumerate(pooled) if g == 0) - n_a * (n_a + 1) / 2.0
    total = 0
    extreme = 0
    mu = n_a * n_b / 2.0
    for combo in itertools.combinations(range(n_a + n_b), n_a):
        u = sum(ranks[i] for i in combo) - n_a * (n_a + 1) / 2.0
        total += 1
        if abs(u - mu) >= abs(u_obs - mu) - 1e-12:
            extreme += 1
    return extreme / total, total


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--arm", action="append", required=True, metavar="LABEL=DIR")
    ap.add_argument("--metric", default="fps_avg",
                    choices=["fps_avg", "pct_on_modal", "interval_p99_ms", "cpu_mean_pct", "rss_steady_kb"])
    ap.add_argument("--boot", type=int, default=10000)
    args = ap.parse_args()

    arms = {}
    for spec in args.arm:
        label, _, directory = spec.partition("=")
        if not os.path.isdir(directory):
            sys.exit(f"not a directory: {directory}")
        values, labels = collect(directory, args.metric)
        if not values:
            sys.exit(f"no {args.metric} values in {directory}")
        arms[label] = (values, labels)
        print(f"[arm] {label:6s} n={len(values):2d}  median={statistics.median(values):.3f}  "
              f"min={min(values):.3f}  max={max(values):.3f}  runs={','.join(labels)}")

    if len(arms) == 2:
        (la, (a, _)), (lb, (b, _)) = list(arms.items())
        diff = statistics.median(b) - statistics.median(a)
        lo, hi = bootstrap_diff(a, b, args.boot)
        p, n_enum = mann_whitney_exact(a, b)
        floor = 2.0 / n_enum
        print(f"[diff] median({lb}) - median({la}) = {diff:+.3f} "
              f"({diff / statistics.median(a) * 100:+.1f}%)")
        print(f"[boot] {args.boot} resamples, 95% interval [{lo:+.3f}, {hi:+.3f}]")
        print(f"[mwu ] exact two-sided p = {p:.4f} over {n_enum} splits (floor for this n = {floor:.4f})")
        if p <= floor + 1e-12:
            print("[note] p is at the floor: the data cannot distinguish this from the most extreme "
                  "outcome the sample size allows, so report the interval, not the p")
        if lo <= 0 <= hi:
            print("[note] the interval crosses zero: this campaign cannot claim a direction for "
                  f"{args.metric} at this n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
