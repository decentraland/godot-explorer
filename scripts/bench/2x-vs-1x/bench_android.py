#!/usr/bin/env python3
"""Measurement window for the 2.x-vs-1.x benchmark (#3206), Android side.

One invocation captures what the device presents (Perfetto frame timeline) plus
memory, CPU and thermal samples of the app process, into one directory. It is
build-agnostic on purpose: 1.x has no Scene Inspector and no `--perf-log`, so a
number coming from the client's own logs would not be comparable across builds.

    bench_android.py window --label 1x-gp-r1 --seconds 60 --out docs/bench/data/...
    bench_android.py launch --label 1x-cold-1 --out ... --activity ...

Everything it writes is timestamped raw data; the aggregation lives in
summarize.py.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import threading
import time
from pathlib import Path

PKG = "org.decentraland.godotexplorer"
DEVICE_TRACE_DIR = "/data/misc/perfetto-traces"
DEVICE_CFG = "/data/misc/perfetto-configs/bench-2x-vs-1x.cfg"
TRACE_PROCESSOR = Path(
    os.environ.get("TRACE_PROCESSOR")
    or Path(__file__).resolve().parents[3] / ".bench-tmp" / "trace_processor_shell"
)


def adb(serial: str, *args: str, timeout: int = 120) -> str:
    out = subprocess.run(
        ["adb", "-s", serial, *args],
        capture_output=True,
        text=True,
        timeout=timeout,
    )
    if out.returncode != 0:
        raise RuntimeError(f"adb {' '.join(args)} failed: {out.stderr.strip()}")
    return out.stdout


def adb_shell(serial: str, command: str, timeout: int = 120) -> str:
    return adb(serial, "shell", command, timeout=timeout)


def app_pid(serial: str) -> int | None:
    out = adb_shell(serial, f"pidof {PKG}").strip()
    return int(out.split()[0]) if out else None


def read_cfg_template() -> str:
    return (Path(__file__).parent / "frame_trace.cfg").read_text()


def write_cfg(serial: str, seconds: float) -> None:
    cfg = re.sub(r"duration_ms: \d+", f"duration_ms: {int(seconds * 1000)}", read_cfg_template())
    tmp = Path("/tmp/bench-2x-vs-1x.cfg")
    tmp.write_text(cfg)
    adb(serial, "push", str(tmp), DEVICE_CFG)


class Sampler(threading.Thread):
    """Samples memory / cpu / thermal once per second while the trace runs."""

    def __init__(self, serial: str, out: Path, thread_scratch: Path):
        super().__init__(daemon=True)
        self.serial = serial
        self.out = out
        self.scratch = thread_scratch
        self.stop_flag = threading.Event()
        self.mem_rows: list[dict] = []
        self.thermal_rows: list[dict] = []
        self.cpu_rows: list[dict] = []

    def run(self) -> None:
        t0 = time.time()
        while not self.stop_flag.is_set():
            t = round(time.time() - t0, 2)
            try:
                pid = app_pid(self.serial)
                mem = adb_shell(self.serial, f"dumpsys meminfo {PKG}", timeout=60)
                row = {"t": t, "pid": pid or 0}
                row.update(parse_meminfo(mem))
                self.mem_rows.append(row)
                (self.scratch / f"meminfo-{int(t)}.log").write_text(mem)
                cpu = adb_shell(self.serial, "top -b -n 1 -o %CPU,ARGS", timeout=60)
                m = re.search(rf"\s([\d.]+)\s+\S*{re.escape(PKG)}", cpu)
                self.cpu_rows.append({"t": t, "cpu_pct": float(m.group(1)) if m else 0.0})
                thermal = adb_shell(self.serial, "dumpsys thermalservice", timeout=60)
                self.thermal_rows.append({"t": t, **parse_thermal(thermal)})
            except Exception as exc:  # a sample is optional, the trace is the deliverable
                print(f"[sample] {exc}", file=sys.stderr)
            self.stop_flag.wait(1.0)


def parse_meminfo(text: str) -> dict:
    """Total PSS/RSS of the process, in KB, plus the biggest category groups."""
    row: dict = {}
    m = re.search(r"TOTAL PSS:\s+(\d+)\s+TOTAL RSS:\s+(\d+)", text)
    if m:
        row["total_pss_kb"] = int(m.group(1))
        row["total_rss_kb"] = int(m.group(2))
    else:
        m = re.search(r"^\s*TOTAL\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+)", text, re.M)
        if m:
            row["total_pss_kb"] = int(m.group(1))
    for label in ("Java Heap", "Native Heap", "Code", "Stack", "Graphics", "Private Other", "System"):
        mm = re.search(rf"{label}:\s+(\d+)", text)
        if mm:
            row[f"pss_{label.lower().replace(' ', '_')}_kb"] = int(mm.group(1))
    m = re.search(r"TOTAL SWAP PSS:\s+(\d+)", text)
    if m:
        row["swap_pss_kb"] = int(m.group(1))
    return row


def parse_thermal(text: str) -> dict:
    temps = [float(v) for v in re.findall(r"Temperature\{mValue=([\d.]+), mType=\d+, mName=([A-Z_]+)", text) for v in [v[0]]]
    names = re.findall(r"mName=([A-Z_]+)", text)
    out = {"thermal_max_c": max(temps) if temps else None}
    for name, value in zip(names, temps):
        key = name.lower()
        if key in ("cpu", "gpu", "battery", "skin", "usb_port"):
            out[f"thermal_{key}_c"] = value
    return out


FRAMES_SQL_WHERE = (
    "p.name = '/system/bin/surfaceflinger' and af.ts >= {start} and af.ts <= {stop}"
)


def frames_query(seconds: float, start: int = 0, stop: int = 2**62) -> str:
    """Per-window frame stats from the frames the device actually presented.

    `dur` on a frame-timeline slice is the frame's expected window (one vsync), not the frame
    time, so the interval distribution is rebuilt from consecutive presentation timestamps.
    """
    where = FRAMES_SQL_WHERE.format(start=start, stop=stop)
    return f"""
with f as (
  select af.ts, af.dur / 1e6 as window_ms, af.jank_type, af.on_time_finish, af.present_type
  from actual_frame_timeline_slice af
  join process p on p.upid = af.upid
  where {where}
), d as (
  select ts, ts - lag(ts) over (order by ts) as dt_ns, jank_type from f
), i as (
  select dt_ns / 1e6 as dt_ms, jank_type from d where dt_ns is not null and dt_ns > 0
)
select
  (select count(*) from f) as frames,
  round((select count(*) from f) * 1000.0 / ({"%.3f" % seconds} * 1000.0), 2) as fps_avg,
  round((select (max(ts) - min(ts)) / 1e9 from f), 2) as span_s,
  round(percentile(dt_ms, 50), 2) as interval_p50_ms,
  round(percentile(dt_ms, 95), 2) as interval_p95_ms,
  round(percentile(dt_ms, 99), 2) as interval_p99_ms,
  round(max(dt_ms), 2) as interval_max_ms,
  round(1000.0 / percentile(dt_ms, 99), 2) as fps_p1_low,
  round(1000.0 / percentile(dt_ms, 95), 2) as fps_p5_low,
  round(1000.0 / percentile(dt_ms, 50), 2) as fps_median,
  round(100.0 * (select sum(case when on_time_finish = 0 then 1 else 0 end) from f)
        / (select count(*) from f), 2) as pct_missed_deadline,
  (select sum(case when jank_type <> 'None' then 1 else 0 end) from f) as jank_any
from i
"""


def run_trace_processor(trace: Path, sql: str) -> str:
    if not TRACE_PROCESSOR.exists():
        raise RuntimeError(
            f"trace_processor_shell not found at {TRACE_PROCESSOR}; download it from\n"
            "https://get.perfetto.dev/trace_processor and point TRACE_PROCESSOR at it"
        )
    out = subprocess.run(
        [str(TRACE_PROCESSOR), str(trace), "-Q", sql],
        capture_output=True,
        text=True,
        timeout=600,
    )
    if out.returncode != 0:
        raise RuntimeError(f"trace_processor failed: {out.stderr[-2000:]}")
    return out.stdout


def parse_tp_csv(text: str) -> list[dict]:
    import csv
    import io

    lines = [ln for ln in text.splitlines() if ln and not ln.startswith("Trace loaded") and not ln.startswith("Query executed")]
    if not lines:
        return []
    return list(csv.DictReader(io.StringIO("\n".join(lines))))


def cmd_window(args: argparse.Namespace) -> None:
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    serial = args.serial
    pid_before = app_pid(serial)
    write_cfg(serial, args.seconds)

    sampler = Sampler(serial, out, out) if not args.no_sample else None
    if sampler:
        sampler.start()
    trace_name = f"{args.label}.perfetto-trace"
    adb_shell(serial, f"rm -f {DEVICE_TRACE_DIR}/{trace_name}")
    started = time.time()
    adb_shell(
        serial,
        f"perfetto --txt -c {DEVICE_CFG} -o {DEVICE_TRACE_DIR}/{trace_name}",
        timeout=int(args.seconds) + 180,
    )
    wall = round(time.time() - started, 2)
    trace_local = out / "trace.perfetto-trace"
    adb(serial, "pull", f"{DEVICE_TRACE_DIR}/{trace_name}", str(trace_local), timeout=600)

    frames = parse_tp_csv(run_trace_processor(trace_local, frames_query(wall)))
    per_frame = parse_tp_csv(
        run_trace_processor(
            trace_local,
            """select af.ts, round((af.ts - lag(af.ts) over (order by af.ts)) / 1e6, 3) as interval_ms,
                       af.jank_type, af.on_time_finish, af.present_type, af.display_frame_token
                from actual_frame_timeline_slice af join process p on p.upid = af.upid
                where p.name = '/system/bin/surfaceflinger' order by af.ts""",
        )
    )
    import csv

    with (out / "frames.csv").open("w", newline="") as fh:
        writer = csv.DictWriter(fh, fieldnames=list(per_frame[0].keys()) if per_frame else ["dur_ms"])
        writer.writeheader()
        writer.writerows(per_frame)

    if sampler:
        sampler.stop_flag.set()
        sampler.join(timeout=10)
        for name, rows in (("mem.csv", sampler.mem_rows), ("thermal.csv", sampler.thermal_rows), ("cpu.csv", sampler.cpu_rows)):
            if not rows:
                continue
            with (out / name).open("w", newline="") as fh:
                writer = csv.DictWriter(fh, fieldnames=sorted({k for r in rows for k in r}))
                writer.writeheader()
                writer.writerows(rows)

    summary = frames[0] if frames else {}
    with (out / "screen.png").open("wb") as fh:
        shot = subprocess.run(["adb", "-s", serial, "exec-out", "screencap -p"], capture_output=True, timeout=120)
        fh.write(shot.stdout)
    mem_rows = sampler.mem_rows if sampler else []
    peak = max((r.get("total_pss_kb", 0) for r in mem_rows), default=0)
    steady = sorted(r.get("total_pss_kb", 0) for r in mem_rows)
    meta = {
        "label": args.label,
        "build": args.build,
        "scenario": args.scenario,
        "device": adb_shell(serial, "getprop ro.product.model").strip(),
        "device_serial": serial,
        "pkg": PKG,
        "version_name": package_version(serial),
        "pid_before": pid_before,
        "trace_wall_seconds": wall,
        "started_at": time.strftime("%Y-%m-%dT%H:%M:%S%z", time.localtime(started)),
        "frames": summary,
        "rss_peak_kb": peak,
        "rss_steady_kb": steady[len(steady) // 2] if steady else 0,
        "mem_samples": len(mem_rows),
        "notes": args.note or "",
        "thermal_end_c": sampler.thermal_rows[-1].get("thermal_max_c") if sampler and sampler.thermal_rows else None,
        "cpu_mean_pct": round(sum(r["cpu_pct"] for r in sampler.cpu_rows) / len(sampler.cpu_rows), 1)
        if sampler and sampler.cpu_rows
        else None,
    }
    (out / "meta.json").write_text(json.dumps(meta, indent=2) + "\n")
    print(json.dumps(summary, indent=2))
    print(f"wrote {out}")


def package_version(serial: str) -> str:
    try:
        dump = adb_shell(serial, f"dumpsys package {PKG}")
        name = re.search(r"versionName=(\S+)", dump)
        code = re.search(r"versionCode=(\d+)", dump)
        debuggable = "DEBUGGABLE" in dump
        return f"{name.group(1) if name else '?'}({code.group(1) if code else '?'}){'-debuggable' if debuggable else ''}"
    except Exception:
        return "?"


def cmd_launch(args: argparse.Namespace) -> None:
    """Cold launch: force-stop, clear logcat, time to the first presented frame and to TotalTime."""
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    serial = args.serial
    adb_shell(serial, f"am force-stop {PKG}")
    adb_shell(serial, "logcat -c")
    t0 = time.time()
    out_start = adb_shell(serial, f"am start -W -n {args.activity}", timeout=180)
    total_time = re.search(r"TotalTime:\s+(\d+)", out_start)
    first_frame = None
    deadline = time.time() + args.wait
    while time.time() < deadline:
        frames = parse_tp_csv(
            run_trace_processor(Path(args.trace), frames_query(60)) if args.trace else []
        )
        if frames:
            first_frame = frames
            break
        time.sleep(1)
    meta = {
        "label": args.label,
        "build": args.build,
        "device_serial": serial,
        "activity": args.activity,
        "version_name": package_version(serial),
        "am_start_total_time_ms": int(total_time.group(1)) if total_time else None,
        "wall_to_total_time_s": round(time.time() - t0, 2),
        "raw_am_start": out_start.strip().splitlines()[-6:],
        "logcat": adb_shell(serial, "logcat -d -v time")[-200000:],
    }
    (out / "meta.json").write_text(json.dumps(meta, indent=2) + "\n")
    (out / "logcat.txt").write_text(meta["logcat"])
    print(json.dumps({k: v for k, v in meta.items() if k != "logcat"}, indent=2))


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--serial", default=os.environ.get("ANDROID_SERIAL", ""))
    sub = parser.add_subparsers(dest="cmd", required=True)

    w = sub.add_parser("window", help="capture a measurement window")
    w.add_argument("--label", required=True)
    w.add_argument("--build", choices=["1x", "2x"], required=True)
    w.add_argument("--scenario", required=True)
    w.add_argument("--seconds", type=float, default=60)
    w.add_argument("--out", required=True)
    w.add_argument("--note", default="")
    w.add_argument("--no-sample", action="store_true")
    w.set_defaults(func=cmd_window)

    lp = sub.add_parser("launch", help="cold-launch timing")
    lp.add_argument("--label", required=True)
    lp.add_argument("--build", choices=["1x", "2x"], required=True)
    lp.add_argument("--activity", required=True)
    lp.add_argument("--wait", type=float, default=60)
    lp.add_argument("--trace", default="")
    lp.add_argument("--out", required=True)
    lp.set_defaults(func=cmd_launch)

    args = parser.parse_args()
    if not args.serial:
        raise SystemExit("no --serial and ANDROID_SERIAL is empty")
    args.func(args)


if __name__ == "__main__":
    main()
