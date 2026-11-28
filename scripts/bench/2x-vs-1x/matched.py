#!/usr/bin/env python3
"""Compare two builds that were measured at the same in-game hours, pair by pair.

Genesis Plaza does not pin the sky: the same scene costs 27.6 fps at 10:13 in-game and 18.4 at 19:12, so
two arms measured at different hours differ by more than the thing being measured. The fix is to run
each build at the same set of in-game hours, which the campaign records as `target_phase` in
`phases.csv` (in-game seconds since in-game midnight). This tool joins the two arms on that column and
reports the per-pair differences, a bootstrap interval for their median, and an exact sign test.

    matched.py --dir data/gp-noon [--metric fps_avg]

Only phases present in both arms are used. A phase measured twice in one arm is an error, not something
to average silently.
"""

from __future__ import annotations

import argparse
import csv
import json
import os
import random
import statistics
import sys

METRICS = {
    "fps_avg": lambda m: m["frames"].get("fps_avg"),
    "interval_p99_ms": lambda m: m["frames"].get("interval_p99_ms"),
    "cpu_mean_pct": lambda m: m.get("cpu_mean_pct"),
}


def _phase(row: dict) -> int:
    """In-game phase of a run: the ledger column when present, otherwise derived from its UTC stamp.

    Two ledgers record the same campaign: the phase-waiting one writes `target_phase`, the plain one only
    writes the wall clock. Deriving the hour from `utc_iso` keeps both readable, because the world clock is
    a pure function of UTC.
    """
    if row.get("target_phase"):
        return int(round(float(row["target_phase"])))
    import calendar
    import time

    epoch = calendar.timegm(time.strptime(row["utc_iso"], "%Y-%m-%dT%H:%M:%SZ"))
    return int(round((epoch * 12) % 86400))


def load(directory: str, metric: str) -> dict[int, tuple[float, str]]:
    """target_phase -> (value, label) per build, read from the ledger and each run's manifest."""
    ledger = os.path.join(directory, "phases.csv")
    if not os.path.exists(ledger):
        sys.exit(f"no ledger: {ledger}")
    out: dict[int, dict[str, tuple[float, str]]] = {}
    for row in csv.DictReader(open(ledger)):
        label, build = row["label"], row["build"]
        run = os.path.join(directory, label, "manifest.json")
        if not os.path.exists(run):
            continue
        value = METRICS[metric](json.load(open(run)))
        if value is None:
            continue
        phase = _phase(row)
        out.setdefault(phase, {})
        if build in out[phase]:
            sys.exit(f"phase {phase} has two {build} runs ({out[phase][build][1]}, {label})")
        out[phase][build] = (float(value), label)
    return out


def bootstrap_median(diffs: list[float], resamples: int = 20000, seed: int = 20261112):
    rng = random.Random(seed)
    meds = sorted(statistics.median([rng.choice(diffs) for _ in diffs]) for _ in range(resamples))
    return meds[int(0.025 * len(meds))], meds[int(0.975 * len(meds)) - 1]


def sign_test_exact(diffs: list[float]) -> tuple[float, int]:
    """Two-sided exact p for 'the sign of the difference is arbitrary'."""
    n = len([d for d in diffs if d != 0])
    if n == 0:
        return 1.0, 1
    k = min(len([d for d in diffs if d > 0]), len([d for d in diffs if d < 0]))
    total = 2 ** n
    tail = sum(_comb(n, i) for i in range(0, k + 1))
    return min(1.0, 2 * tail / total), total


def _comb(n: int, k: int) -> int:
    from math import comb

    return comb(n, k)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--base-dir", required=True)
    ap.add_argument("--new-dir", required=True)
    ap.add_argument("--metric", default="fps_avg", choices=sorted(METRICS))
    ap.add_argument("--arm", nargs=2, default=["1x", "2x"], metavar=("BASE", "NEW"))
    args = ap.parse_args()

    base, new = args.arm
    base_data = load(args.base_dir, args.metric)
    new_data = load(args.new_dir, args.metric)
    phases = sorted(set(base_data) | set(new_data))
    data = {p: {**base_data.get(p, {}), **new_data.get(p, {})} for p in phases}
    pairs = [(p, base_data[p][base], new_data[p][new])
             for p in sorted(set(base_data) & set(new_data))
             if base in base_data[p] and new in new_data[p]]
    if not pairs:
        sys.exit(f"no phase has both a {base} run in {args.base_dir} and a {new} run in {args.new_dir}")

    only = [p for p in sorted(data) if (base in data[p]) != (new in data[p])]
    if only:
        print(f"[note] {len(only)} phase(s) measured in one arm only and left out: {only}")

    print(f"{'in-game':>8}{'phase':>8}{base:>9}{new:>9}{'delta':>8}  pair")
    diffs = []
    for phase, (a, la), (b, lb) in pairs:
        d = b - a
        diffs.append(d)
        hh, mm = divmod(phase, 3600)
        print(f"{hh:02d}:{mm // 60:02d}{'':>3}{phase:>8}{a:>9.2f}{b:>9.2f}{d:>+8.2f}  {la} / {lb}")

    med = statistics.median(diffs)
    base_med = statistics.median([a for _, (a, _), _ in pairs])
    lo, hi = bootstrap_median(diffs)
    p, total = sign_test_exact(diffs)
    print(f"\n[median] {base} {base_med:.2f} -> {new} {statistics.median([b for _, _, (b, _) in pairs]):.2f}")
    print(f"[paired] median of the {len(diffs)} per-pair differences = {med:+.2f} "
          f"({med / base_med * 100:+.1f} % of the {base} median)")
    print(f"[boot  ] 95% interval [{lo:+.2f}, {hi:+.2f}] over {20000} resamples of the pairs")
    print(f"[sign  ] exact two-sided p = {p:.4f} over {total} sign assignments")
    if lo <= 0 <= hi:
        print("[note] the interval crosses zero: no direction for this metric at this n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
