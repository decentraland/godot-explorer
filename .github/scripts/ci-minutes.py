#!/usr/bin/env python3
"""Billable-runner minutes per runner label over the last N days (default 7).

Usage: .github/scripts/ci-minutes.py [days]   (needs an authenticated `gh`)
Counts only paid runners (warp-*, self-hosted excluded); cancelled/failed jobs count too, since
they bill. Compare against the WarpBuild usage page to calibrate the $/min of each label.
"""
import collections
import datetime
import json
import subprocess
import sys

days = int(sys.argv[1]) if len(sys.argv) > 1 else 7
since = (datetime.datetime.now(datetime.timezone.utc) - datetime.timedelta(days=days)).isoformat()


def parse(ts):
    return datetime.datetime.fromisoformat(ts.replace("Z", "+00:00"))


repo = "decentraland/godot-explorer"
runs = []
for page in range(1, 11):
    r = json.loads(subprocess.run(
        ["gh", "api", f"repos/{repo}/actions/runs?per_page=100&page={page}&created=>={since[:10]}"],
        capture_output=True, text=True, check=True).stdout)
    runs += r["workflow_runs"]
    if len(r["workflow_runs"]) < 100:
        break

mins = collections.defaultdict(float)
count = collections.Counter()
by_job = collections.defaultdict(float)
for run in runs:
    res = subprocess.run(["gh", "api", f"repos/{repo}/actions/runs/{run['id']}/jobs?per_page=100"],
                         capture_output=True, text=True)
    if res.returncode:
        continue
    jobs = json.loads(res.stdout)["jobs"]
    for j in jobs:
        label = ",".join(j["labels"])
        if not label.startswith("warp") or not j.get("completed_at"):
            continue
        m = (parse(j["completed_at"]) - parse(j["started_at"])).total_seconds() / 60
        mins[label] += m
        count[label] += 1
        by_job[(label, j["conclusion"])] += m

print(f"last {days} days, {len(runs)} runs")
for label, m in sorted(mins.items(), key=lambda x: -x[1]):
    print(f"{label:32} {count[label]:4} jobs {m:8.0f} min")
    for (l, c), v in sorted(by_job.items()):
        if l == label:
            print(f"    {c or 'running':12} {v:8.0f} min")
