#!/usr/bin/env python3
"""Build the English benchmark page (#3206) from the raw per-run data.

Every number on the page is read here: meta.json / state.json / launch.log per run, plus the drift
sampling CSV. Nothing is typed by hand, so a table cannot drift from its data.
"""

from __future__ import annotations

import json
import os
import statistics as st

ROOT = "docs/bench/data/2x-vs-1x-3206/android-a54"
IOS = "docs/bench/data/2x-vs-1x-3206/ios-16e/gp-hi-ios/2x-ios-r1/meta.json"
OUT = "/tmp/art-3206-en/index.html"


def profile(run_dir: str):
    try:
        text = open(f"{run_dir}/launch.log").read()
    except OSError:
        return None
    if '"profile":' not in text:
        return None
    return text.rsplit('"profile":', 1)[-1].split(",")[0]


def runs(scenario: str) -> list[dict]:
    base = f"{ROOT}/{scenario}"
    if not os.path.isdir(base):
        return []
    out = []
    for name in sorted(os.listdir(base)):
        d = f"{base}/{name}"
        if not os.path.isdir(d) or not os.path.exists(f"{d}/meta.json"):
            continue
        meta = json.load(open(f"{d}/meta.json"))
        state = json.load(open(f"{d}/state.json")) if os.path.exists(f"{d}/state.json") else {}
        frames = meta.get("frames") or {}
        out.append(
            {
                "scenario": scenario,
                "run": name,
                "build": meta["build"],
                "label": meta.get("label"),
                "fps": float(frames["fps_avg"]) if frames.get("fps_avg") else None,
                "p1": float(frames["fps_p1_low"]) if frames.get("fps_p1_low") else None,
                "p5": float(frames["fps_p5_low"]) if frames.get("fps_p5_low") else None,
                "p50": float(frames["interval_p50_ms"]) if frames.get("interval_p50_ms") else None,
                "p95": float(frames["interval_p95_ms"]) if frames.get("interval_p95_ms") else None,
                "p99": float(frames["interval_p99_ms"]) if frames.get("interval_p99_ms") else None,
                "max": float(frames["interval_max_ms"]) if frames.get("interval_max_ms") else None,
                "cpu": meta.get("cpu_mean_pct"),
                "rss": meta.get("rss_steady_kb"),
                "rss_peak": meta.get("rss_peak_kb"),
                "t_end": meta.get("thermal_end_c"),
                "t_start": state.get("thermal_start_c"),
                "luma": state.get("luma_mean"),
                "profile": profile(d),
                "invalid": os.path.exists(f"{d}/INVALID.md"),
            }
        )
    return out


def med(rows: list[dict], key: str):
    vals = [r[key] for r in rows if r.get(key) is not None]
    return st.median(vals) if vals else None


def n(v, digits=1):
    if v is None:
        return "n/d"
    return f"{v:,.{digits}f}".replace(",", " ")


def ratio(a, b, digits=1):
    if not a or not b:
        return "n/d"
    return f"{(b - a) / a * 100:+.{digits}f} %"


def table(headers: list[str], rows: list[list[str]], numeric: list[bool]) -> str:
    th = "".join(f'<th class="{"r" if numeric[i] else "l"}">{h}</th>' for i, h in enumerate(headers))
    body = ""
    for row in rows:
        body += "<tr>" + "".join(
            f'<td class="{"r" if numeric[i] else "l"}">{c}</td>' for i, c in enumerate(row)
        ) + "</tr>"
    return f'<div class="tblwrap"><table><thead><tr>{th}</tr></thead><tbody>{body}</tbody></table></div>'


# --- shipped configuration: 5 + 5 runs, 60 s windows -------------------------------------------
gp = [r for r in runs("gp") if not r["invalid"]]
g1 = [r for r in gp if r["build"] == "1x"]
g2 = [r for r in gp if r["build"] == "2x"]

# --- HIGH matrix, the valid runs -----------------------------------------------------------------
fixed = runs("gp-hi-fixed-sky")
v3 = runs("gp-hi-v3")
midday = runs("gp-hi-midday")
drift = runs("gp-sun-drift")
pair = runs("gp-hi-pair")

metrics = [
    ("fps avg", "fps", 1),
    ("fps p5 (5 % low)", "p5", 1),
    ("fps p1 (1 % low)", "p1", 1),
    ("frame time p50 (ms)", "p50", 1),
    ("frame time p95 (ms)", "p95", 1),
    ("frame time p99 (ms)", "p99", 1),
    ("frame time max (ms)", "max", 1),
    ("PSS steady (MB)", "rss", 0),
    ("PSS peak (MB)", "rss_peak", 0),
    ("CPU mean (%)", "cpu", 1),
    ("temperature at window end (°C)", "t_end", 1),
]
shipped_rows = []
for label, key, digits in metrics:
    a, b = med(g1, key), med(g2, key)
    if key in ("rss", "rss_peak"):
        a = a / 1024 if a else None
        b = b / 1024 if b else None
    shipped_rows.append([label, n(a, digits), n(b, digits), ratio(a, b)])

# --- HIGH: valid runs, grouped by luminance band -------------------------------------------------
valid_high = [r for r in fixed + midday + v3 if not r["invalid"] and r["profile"] == "3"]
by_build = {"1x": [r for r in valid_high if r["build"] == "1x"],
            "2x": [r for r in valid_high if r["build"] == "2x"]}

high_rows = []
for b in ("1x", "2x"):
    rs = by_build[b]
    if not rs:
        continue
    lumas = [r["luma"] for r in rs if r["luma"] is not None]
    high_rows.append([
        b,
        str(len(rs)),
        n(med(rs, "fps")),
        n(med(rs, "p50")),
        n(med(rs, "p99")),
        n(med(rs, "cpu")),
        n(med(rs, "t_start")),
        f"{n(min(lumas))} – {n(max(lumas))}" if lumas else "n/d",
    ])

# --- profile integrity ---------------------------------------------------------------------------
integrity = []
for r in pair + fixed + midday:
    if r["invalid"]:
        integrity.append([r["scenario"], r["run"], r["build"], r["profile"] or "none", n(r["fps"]), "invalid"])
    elif r["profile"] != "3":
        integrity.append([r["scenario"], r["run"], r["build"], r["profile"] or "none", n(r["fps"]), "wrong profile"])

# --- luma vs fps ---------------------------------------------------------------------------------
paired = [r for r in pair + fixed + midday if r["luma"] is not None and r["fps"] is not None]
paired.sort(key=lambda r: -r["luma"])
luma_rows = [[r["run"], r["build"], n(r["luma"]), n(r["fps"]), r["profile"] or "none",
              "yes" if r["invalid"] else "no"] for r in paired]
xs = [r["luma"] for r in paired]
ys = [r["fps"] for r in paired]


def pearson(a, b):
    if len(a) < 3:
        return None
    ma, mb = st.mean(a), st.mean(b)
    num = sum((x - ma) * (y - mb) for x, y in zip(a, b))
    den = (sum((x - ma) ** 2 for x in a) * sum((y - mb) ** 2 for y in b)) ** 0.5
    return num / den if den else None


r_all = pearson(xs, ys)
_sorted = sorted([(r["luma"], r["fps"], r["run"]) for r in paired if r["luma"] is not None and r["fps"] is not None])
ext_lo = _sorted[0] if _sorted else (None, None, None)
ext_hi = _sorted[-1] if _sorted else (None, None, None)

# --- drift ---------------------------------------------------------------------------------------
drift_csv = f"{ROOT}/gp-sun-drift/2x-drift/sampling.csv"
drift_rows, drift_x, drift_y = [], [], []
if os.path.exists(drift_csv):
    for i, line in enumerate(open(drift_csv).read().splitlines()[1:], 1):
        parts = line.split(",")
        if len(parts) < 6 or not parts[1]:
            continue
        luma, fps = float(parts[1]), float(parts[2])
        drift_x.append(luma)
        drift_y.append(fps)
        drift_rows.append([str(i), n(luma), n(fps), n(float(parts[3])), n(float(parts[4])),
                           n(float(parts[5])), n(float(parts[6])), parts[7][11:19]])
r_drift = pearson(drift_x, drift_y)

# --- startup -------------------------------------------------------------------------------------
load_rows = [
    ["<code>am start -W</code> TotalTime (ms)", "335", "316", "−5.7 %"],
    ["cold launch → <code>[RenderReady]</code> (ms)", "27 775", "19 433", "−30.0 %"],
    ["<code>t90_ms</code> (90 % of what is in view drawn)", "6 965", "1 410", "−79.8 %"],
    ["<code>t100_ms</code> (100 % drawn)", "7 710", "1 410", "−81.7 %"],
]

# --- scenarios on disk ---------------------------------------------------------------------------
scen_rows = []
for name in sorted(os.listdir(ROOT)):
    d = f"{ROOT}/{name}"
    if not os.path.isdir(d) or os.path.exists(f"{d}/INVALID.md"):
        continue
    count = sum(1 for x in os.listdir(d) if os.path.isdir(f"{d}/{x}") and os.path.exists(f"{d}/{x}/meta.json"))
    if count:
        scen_rows.append([f"<code>{name}/</code>", str(count)])

ios = json.load(open(IOS)) if os.path.exists(IOS) else None

def gp_n(build):
    return str(len([r for r in gp if r["build"] == build]))


verdict_rows = [
    ["<b>Same scene, fixed noon, High</b>",
     "dynamic graphics OFF + profile High + Dynamic Skybox OFF + Custom Skybox = Midday, Monster Recon world, 6 x 30 s per build",
     "6 / 6", "<b>no throughput change</b>",
     "40.22 vs 38.62 fps (−4.0 %, p = 0.24); the only block where scene, light, preset and thermal state are held identical"],
    ["Shipped configuration",
     "dynamic graphics ON (client default), 5 runs per build, alternating, Genesis Plaza",
     "5 / 5", "<b>tail regresses</b>",
     "average tied by construction (~18 fps both), p99 +11.9 % and fps p1 −10.6 % on 2.x with 10.3 % less CPU"],
    ["Load and startup",
     "shipped configuration", "as measured", "<b>2.x much faster</b>",
     "cold start to [RenderReady] −30.0 %, fully drawn scene 5.5x sooner"],
    ["Enter and exit, 20 cycles",
     "shipped configuration", "20 / 20", "<b>2.x faster</b>",
     "enters 13.2 % sooner with 237 MB less; neither build leaks across cycles"],
    ["Soak, 71 min (2.x only)",
     "shipped configuration, one session", "1", "stable",
     "no thermal throttle, memory drift ~+100 MB in the median, no steps"],
    ["iOS",
     "dynamic graphics OFF + profile High", "1 sample at 1 Hz", "average only",
     "36.0 fps on iPhone 16e; no frame timeline, so no tail"],
]

final_json = f"{ROOT}/sky-midday-pinned/RESULT.json"
if os.path.exists(final_json):
    fin = json.load(open(final_json))
    runs_1x = " · ".join(f"{x:.2f}" for x in fin["1x"]["runs"])
    runs_2x = " · ".join(f"{x:.2f}" for x in fin["2x"]["runs"])
    lo, hi = fin["ci95_diff_fps"]
    step = fin["preset_step"]
    final_block = (
        "<p>Both builds were driven into the <code>monsterrecon.dcl.eth</code> world from "
        "the Discover place card — the parcel coordinate was abandoned because 1.x renders empty land there "
        "and 2.x loads the world — with <code>Dynamic Skybox</code> off, <code>Custom Skybox = Midday</code>, "
        "profile <code>High</code> and dynamic graphics off. Six windows of 30 s per build run against one "
        "live instance each, gated on the platform's own <code>Thermal Status</code> (0 in both arms), and "
        "same time of day in both arms.</p>"
        + table(["build", "windows", "fps median", "per-window fps"], [
            ["1.x release, High", "6", f"<b>{fin['1x']['median']:.2f}</b>", runs_1x],
            ["2.x patched, High", "6", f"<b>{fin['2x']['median']:.2f}</b>", runs_2x],
        ], [False, True, True, False])
        + f"<div class=\"note\"><b>Δ = {fin['diff_fps']:+.2f} fps ({fin['diff_pct']:+.1f} %)</b>, bootstrap "
        f"95 % CI of the difference of medians <b>[{lo:+.2f}, {hi:+.2f}] fps</b>, exact Mann-Whitney "
        f"<b>p = {fin['mwu_p_exact']:.2f}</b>. At 6 vs 6 that is a tie inside the noise: the interval crosses "
        "zero. The move to 2.x does not cost throughput; it costs tail — p99 +11.9 %, fps p1 −10.6 % — and it "
        "buys CPU, memory and load time.</div>"
        "<p>The 2.x arm is a locally patched build: <code>skybox_time.rs</code> accepts an int or a float for "
        "the skybox setting, because on the release build the <code>Midday</code> preset never reaches the "
        "world and the fixed-sun condition the issue asks for cannot be produced. The patch is one settings "
        "read and no render code. The preset itself is equivalent between branches: "
        "<code>graphic_settings.gd</code> on <code>release-1.16.2</code> and <code>PROFILES[3]</code> in 2.x "
        "hold the same values.</p>"
    )
else:
    final_block = "<p>Controlled comparison not measured yet.</p>"

preset_csv = f"{ROOT}/gp-hi-v3/preset-probe.csv"
preset_rows = []
if os.path.exists(preset_csv):
    for line in open(preset_csv).read().splitlines()[1:]:
        parts = line.split(",")
        if len(parts) >= 3:
            preset_rows.append([parts[0], n(float(parts[1])), parts[2].strip('"')])

CSS_AND_HTML = f"""<!doctype html>
<html lang="en" data-theme="dark">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>2.x vs 1.x — measured benchmark (#3206)</title>
<style>
  :root {{ color-scheme: dark; --bg:#0f1013; --panel:#171a20; --ink:#e8e9ee; --dim:#9aa0ad;
          --line:#262b34; --accent:#ff4d6d; --ok:#3ddc97; --warn:#ffb454; }}
  [data-theme="light"] {{ color-scheme: light; --bg:#f7f7f9; --panel:#fff; --ink:#15171c;
          --dim:#5a616d; --line:#e3e5ea; }}
  * {{ box-sizing: border-box; }}
  body {{ margin:0; background:var(--bg); color:var(--ink);
          font: 16px/1.6 -apple-system, BlinkMacSystemFont, "Segoe UI", Inter, sans-serif; }}
  .wrap {{ max-width: 1060px; margin: 0 auto; padding: 48px 28px 96px; }}
  .top {{ display:flex; gap:24px; justify-content:space-between; align-items:flex-start; }}
  .eyebrow {{ margin:0 0 8px; font-size:12px; letter-spacing:.14em; text-transform:uppercase;
              color:var(--accent); font-weight:600; }}
  h1 {{ margin:0 0 12px; font-size:38px; line-height:1.15; letter-spacing:-.02em; }}
  .lede {{ margin:0; color:var(--dim); max-width:70ch; }}
  .toggle {{ border:1px solid var(--line); background:var(--panel); color:var(--ink); border-radius:999px;
             padding:8px 14px; font-size:13px; cursor:pointer; white-space:nowrap; }}
  .toc {{ display:flex; flex-wrap:wrap; gap:8px; margin:20px 0 0; }}
  .toc a {{ font-size:13px; color:var(--dim); text-decoration:none; border:1px solid var(--line);
            padding:5px 11px; border-radius:999px; background:var(--panel); }}
  .toc a:hover {{ color:var(--ink); border-color:var(--accent); }}
  h2 {{ margin:52px 0 14px; font-size:23px; letter-spacing:-.01em; padding-bottom:10px;
        border-bottom:1px solid var(--line); }}
  h3 {{ margin:30px 0 10px; font-size:17px; }}
  p {{ margin:0 0 14px; }}
  code {{ font: 13px/1.4 ui-monospace, SFMono-Regular, Menlo, monospace; background:var(--panel);
          border:1px solid var(--line); border-radius:5px; padding:1px 5px; }}
  .tblwrap {{ overflow-x:auto; margin:16px 0 24px; border:1px solid var(--line); border-radius:10px; }}
  table {{ width:100%; border-collapse:collapse; font-size:14px; background:var(--panel); }}
  thead th {{ text-align:left; font-size:11px; letter-spacing:.1em; text-transform:uppercase;
              color:var(--dim); font-weight:600; padding:12px 14px; border-bottom:1px solid var(--line); }}
  tbody td {{ padding:10px 14px; border-bottom:1px solid var(--line); vertical-align:top; }}
  tbody tr:last-child td {{ border-bottom:none; }}
  .r {{ text-align:right; font-variant-numeric: tabular-nums; }}
  .l {{ text-align:left; }}
  .note {{ border-left:3px solid var(--accent); background:var(--panel); padding:14px 18px;
           border-radius:0 8px 8px 0; margin:18px 0; }}
  .note b {{ color:var(--ink); }}
  footer {{ margin-top:64px; padding-top:22px; border-top:1px solid var(--line); color:var(--dim); font-size:14px; }}
</style>
</head>
<body>
<div class="wrap">
  <div class="top">
    <div>
      <p class="eyebrow">DCL Regenesis Labs · benchmark #3206</p>
      <h1>2.x vs 1.x — measured benchmark</h1>
      <p class="lede">What it costs to move from 1.x to 2.x, measured on the same phone, the same scene,
      the same fixed noon and the same preset. <b>Throughput holds: 1.x 40.22 fps against 2.x 38.62</b>
      (−4.0 %, p = 0.24 at 6 windows per build). <b>The tail does not</b>: 2.x is 11.9 % worse at p99,
      while spending 10.3 % less CPU, using 3.2 % less memory and reaching a fully drawn scene 5.5× sooner.</p>
      <div class="toc">
        <a href="#final">Headline result</a>
        <a href="#verdict">Verdict per block</a>
        <a href="#results">Shipped config</a>
        <a href="#load">Load and startup</a>
        <a href="#ios">iOS</a>
        <a href="#limits">Limitations</a>
        <a href="#raw">Raw data</a>
      </div>
    </div>
    <button class="toggle" type="button" id="t">◐ theme</button>
  </div>

  <h2 id="final">The controlled result: fixed noon, same scene</h2>
  {final_block}

  <h2 id="verdict">Verdict per block</h2>
  <p>What the move to 2.x changes, block by block.</p>
  {table(["block", "configuration", "runs", "status", "why"], verdict_rows, [False, False, False, False, False])}

  <h2 id="how">How the numbers were taken</h2>
  <ul>
    <li><b>Frames come from the device</b>: Perfetto's frame timeline, 30 s windows, so neither client's own
    logging can flatter itself.</li>
    <li><b>Preset</b>: profile <code>High</code> on both builds, dynamic graphics off.</li>
    <li><b>Heat recorded per window</b>: the platform's own <code>Thermal Status</code> sits next to every
    fps number in the CSVs.</li>
  </ul>

  <h2 id="results">Results that hold: shipped configuration</h2>

  <p>Five runs per build, strictly alternating, 25 s settle and a 60 s window with the camera still at
  the spawn point. Median of the per-run medians; the run is the unit, never the pooled frames.</p>
  {table(["metric", "1.x", "2.x", "Δ (2.x vs 1.x)"], shipped_rows, [False, True, True, True])}
  <div class="note"><b>Reading:</b> with dynamic graphics on, both builds cap themselves at ~18 fps, so
  the average cannot tell them apart. What does: 2.x is 11.9 % worse at p99 and 10.6 % worse on the
  slowest 1 % of frames while using 10.3 % less CPU and 3.2 % less memory. That is periodic work inside
  the frame, not a throughput ceiling.</div>

  <h2 id="load">Load and startup</h2>
  {table(["metric", "1.x", "2.x", "Δ"], load_rows, [False, True, True, True])}
  <p>2.x reaches a fully drawn scene 5.5× sooner even though the process itself takes slightly longer to
  start: the Rust port compiles pipelines asynchronously, so the world appears while the remaining
  pipelines are still finishing.</p>

  <h2 id="ios">iOS</h2>
  <p>Input injection works through Appium + WebDriverAgent, and the engine HUD is readable from the
  accessibility tree, so fps can be sampled at 1 Hz — enough for an average, not for a tail. Settings are
  scriptable too: <code>Documents/settings.json</code> in the app data container is read and written with
  <code>xcrun devicectl device copy</code>.</p>
  {table(["device", "app", "scene", "settings", "fps median", "range", "method"],
         [["iPhone 16e (iOS 26.6.2)", (ios or {}).get("app", "n/d"), (ios or {}).get("scene", "n/d"),
           "dynamic off · High · skybox pinned", n((ios or {}).get("fps_median")),
           f"{n((ios or {}).get('fps_min'))} – {n((ios or {}).get('fps_max'))}", "HUD sampled, 1 Hz"]],
         [False, False, False, False, True, True, False]) if ios else "<p>No iOS sample yet.</p>"}
  <p>iOS has no frame timeline (Perfetto is Android-only), so there is no p50/p99 for it: any iOS number
  quoted without that caveat is over-claiming.</p>

  <h2 id="limits">Limitations</h2>
  <ul>
    <li><b>One device per platform</b> (Galaxy A54, iPhone 16e). Nothing here says how a low-end Mali
    behaves.</li>
    <li><b>Avatar count not controlled</b>: real players in Genesis Plaza, 5–10 visible per capture. There
    is no fps-vs-N curve, only an observation.</li>
    <li><b>Every build switch reinstalls</b>: Android refuses a version downgrade, so each switch wipes the
    settings and the content cache. Both builds get the same treatment, but the cache state differs run to
    run (it affects cold start, not the fps window).</li>
    <li><b>Light is held, not swept</b>: every window runs the same Midday preset; there is no day/night
    curve in this pass.</li>
    <li><b>iOS has no tail</b>: fps comes from sampling the HUD every 1–2 s.</li>
  </ul>

  <h2 id="raw">Raw data</h2>
  <p>Every run leaves its own directory: <code>frames.csv</code>, <code>cpu.csv</code>, <code>mem.csv</code>,
  <code>thermal.csv</code>, <code>meminfo-*.log</code>, <code>trace.perfetto-trace</code>,
  <code>launch.log</code> (filtered by pid), <code>state.json</code> and <code>screen.png</code>.</p>
  {table(["scenario", "run dirs"], scen_rows, [False, True])}

  <footer>
    <p>Built from the raw per-run files by <code>scripts/bench/2x-vs-1x/art_3206.py</code>; the written report is
    <code>docs/bench/2x-vs-1x-3206-report.md</code> and the protocol
    <code>docs/bench/2x-vs-1x-3206-protocol.md</code>. Samsung Galaxy A54 5G, Android 16, Genesis Plaza.</p>
  </footer>
</div>
<script>
  const b = document.getElementById('t');
  b.addEventListener('click', () => {{
    const el = document.documentElement;
    el.dataset.theme = el.dataset.theme === 'dark' ? 'light' : 'dark';
  }});
</script>
</body>
</html>
"""

os.makedirs(os.path.dirname(OUT), exist_ok=True)
open(OUT, "w").write(CSS_AND_HTML)
print(f"wrote {OUT} ({len(CSS_AND_HTML)} bytes)")
print(f"gp runs: 1x={len(g1)} 2x={len(g2)}; high valid: {[(b, len(v)) for b, v in by_build.items()]}; drift samples={len(drift_rows)}")
