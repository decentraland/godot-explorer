#!/usr/bin/env python3
"""
Skybox A/B comparison: Godot capture vs Unity reference, with iteration support.

Usage:
    python3 skybox_compare.py <unity_dir> <godot_dir> <output_dir> \
        [--approved-dir DIR] [--target-hours HH[,HH,...]]

Per-pair status (drives the report layout):
    TARGET  → hour is in --target-hours, compare current Godot vs Unity (improving toward goal)
    LOCKED  → hour NOT in target AND we have an approved snapshot, compare current vs approved
              (regression check — should stay near zero so we don't break a tuned hour)
    PENDING → hour NOT in target AND no approved snapshot, compare vs Unity for context only

The Unity reference screenshots have HUD overlays (sidebar, minimap, time selector). We crop
both inputs to a "safe area" — central 88% width × 87% height — that excludes the UI bands
before comparing.
"""

import argparse
import json
from pathlib import Path
from PIL import Image, ImageChops

# Safe-area crop ratios. We compare only the upper-right region:
#  - left 25%: skip Unity's UI sidebar + notification panel (Godot has no equivalent)
#  - bottom 50%: skip Unity's water (Godot renders floor color, not water)
#  - tiny top + right margin: skip Unity's top notification and minimap
CROP_LEFT = 0.25
CROP_RIGHT = 0.96
CROP_TOP = 0.05
CROP_BOTTOM = 0.50

# Sample zones for per-band color analysis (after crop). With CROP_BOTTOM=0.5 the cropped
# region is just the upper half, so zone fractions are within that half.
ZONES = {
    "zenith": 0.05,
    "upper_sky": 0.25,
    "mid_sky": 0.50,
    "lower_sky": 0.75,
    "horizon": 0.95,
}

DIRECTIONS = ["N", "E", "S", "W", "U"]
HOURS = list(range(24))

THUMB_WIDTH = 480

# Severity thresholds (mean Manhattan delta across all RGB channels)
DELTA_OK = 15
DELTA_WARN = 35

# Regression tolerance: how much the LOCKED diff is allowed to grow before flagging
LOCKED_REGRESSION_TOLERANCE = 8


def crop_safe(img: Image.Image) -> Image.Image:
    w, h = img.size
    box = (int(w * CROP_LEFT), int(h * CROP_TOP), int(w * CROP_RIGHT), int(h * CROP_BOTTOM))
    return img.crop(box)


def make_thumb(img: Image.Image, target_w: int = THUMB_WIDTH) -> Image.Image:
    if img.width <= target_w:
        return img
    ratio = target_w / img.width
    return img.resize((target_w, int(img.height * ratio)), Image.LANCZOS)


def to_rgb(img: Image.Image) -> Image.Image:
    return img.convert("RGB") if img.mode != "RGB" else img


def normalize_size(a: Image.Image, b: Image.Image):
    if a.size == b.size:
        return a, b
    target = (min(a.width, b.width), min(a.height, b.height))
    return a.resize(target, Image.LANCZOS), b.resize(target, Image.LANCZOS)


def sample_zone(img: Image.Image, y_fraction: float):
    w, h = img.size
    y = max(0, min(h - 1, int(h * y_fraction)))
    strip_h = max(1, h // 50)
    y0 = max(0, y - strip_h // 2)
    y1 = min(h, y0 + strip_h)
    strip = img.crop((0, y0, w, y1))
    px = strip.resize((1, 1), Image.LANCZOS).getpixel((0, 0))
    return px[:3] if isinstance(px, tuple) else (px, px, px)


def color_delta(a: tuple, b: tuple) -> dict:
    dr, dg, db = a[0] - b[0], a[1] - b[1], a[2] - b[2]
    return {
        "ref": list(a),
        "got": list(b),
        "dr": dr, "dg": dg, "db": db,
        "manhattan": abs(dr) + abs(dg) + abs(db),
    }


def diff_image(a: Image.Image, b: Image.Image) -> Image.Image:
    diff = ImageChops.difference(a, b)
    return diff.point(lambda p: min(255, p * 4))


def mean_pixel_delta(a: Image.Image, b: Image.Image) -> dict:
    diff = ImageChops.difference(a, b)
    small = diff.resize((100, 100), Image.LANCZOS)
    pixels = list(small.getdata())
    n = len(pixels)
    sums = [0, 0, 0]
    for px in pixels:
        rgb = px[:3] if isinstance(px, tuple) else (px, px, px)
        sums[0] += rgb[0]
        sums[1] += rgb[1]
        sums[2] += rgb[2]
    return {
        "mean_dr": round(sums[0] / n, 2),
        "mean_dg": round(sums[1] / n, 2),
        "mean_db": round(sums[2] / n, 2),
        "mean_manhattan": round((sums[0] + sums[1] + sums[2]) / n, 2),
    }


def severity_for_target(mean_manhattan: float) -> str:
    if mean_manhattan < DELTA_OK:
        return "ok"
    if mean_manhattan < DELTA_WARN:
        return "warn"
    return "bad"


def severity_for_locked(mean_manhattan: float) -> str:
    # LOCKED should be ≈0 — any non-trivial delta means regression
    if mean_manhattan < LOCKED_REGRESSION_TOLERANCE:
        return "ok"
    if mean_manhattan < LOCKED_REGRESSION_TOLERANCE * 3:
        return "warn"
    return "bad"


def compare_pair(ref_path: Path, got_path: Path, out_dir: Path, key: str, status: str) -> dict:
    ref_img = to_rgb(Image.open(ref_path))
    got_img = to_rgb(Image.open(got_path))

    ref_crop = crop_safe(ref_img)
    got_crop = crop_safe(got_img)
    ref_n, got_n = normalize_size(ref_crop, got_crop)

    pixel_stats = mean_pixel_delta(ref_n, got_n)

    zones = {}
    for name, y_frac in ZONES.items():
        ref_z = sample_zone(ref_n, y_frac)
        got_z = sample_zone(got_n, y_frac)
        zones[name] = color_delta(ref_z, got_z)

    ref_thumb = make_thumb(ref_n)
    got_thumb = make_thumb(got_n)
    diff_thumb = make_thumb(diff_image(ref_n, got_n))

    (out_dir / "thumbs").mkdir(parents=True, exist_ok=True)
    ref_thumb_path = out_dir / "thumbs" / f"{key}_ref.png"
    got_thumb_path = out_dir / "thumbs" / f"{key}_got.png"
    diff_thumb_path = out_dir / "thumbs" / f"{key}_diff.png"
    ref_thumb.save(ref_thumb_path)
    got_thumb.save(got_thumb_path)
    diff_thumb.save(diff_thumb_path)

    if status == "LOCKED":
        sev = severity_for_locked(pixel_stats["mean_manhattan"])
    else:
        sev = severity_for_target(pixel_stats["mean_manhattan"])

    return {
        "key": key,
        "status": status,
        "severity": sev,
        "ref_thumb": f"thumbs/{key}_ref.png",
        "got_thumb": f"thumbs/{key}_got.png",
        "diff_thumb": f"thumbs/{key}_diff.png",
        "pixel_stats": pixel_stats,
        "zones": zones,
    }


STATUS_ORDER = {"LOCKED": 0, "TARGET": 1, "PENDING": 2}
STATUS_LABEL = {
    "LOCKED": "vs approved (regression check)",
    "TARGET": "vs Unity (tuning target)",
    "PENDING": "vs Unity (not yet targeted)",
}


def render_html(results: list, out_dir: Path, summary: dict, target_hours: list, approved_keys: set) -> Path:
    sections = {"LOCKED": [], "TARGET": [], "PENDING": []}
    for r in results:
        sections[r["status"]].append(r)
    for k in sections:
        sections[k].sort(key=lambda x: x["key"])

    section_html = []
    for status_key in ["TARGET", "LOCKED", "PENDING"]:
        rows = sections[status_key]
        if not rows:
            continue
        title = "%s — %s (%d)" % (status_key, STATUS_LABEL[status_key], len(rows))
        body_rows = []
        for r in rows:
            zones_html = "".join(
                f"<div class='z'><b>{name}</b>"
                f"<span class='swatch' style='background: rgb({d['ref'][0]},{d['ref'][1]},{d['ref'][2]})'></span>"
                f" → <span class='swatch' style='background: rgb({d['got'][0]},{d['got'][1]},{d['got'][2]})'></span>"
                f" Δ {d['manhattan']}</div>"
                for name, d in r["zones"].items()
            )
            ps = r["pixel_stats"]
            sev = r["severity"]
            ref_label = "Approved" if r["status"] == "LOCKED" else "Unity"
            body_rows.append(f"""
<tr class='sev-{sev}'>
  <td><b>{r['key']}</b><br><span class='sev sev-{sev}'>{sev.upper()}</span></td>
  <td><a href='{r['ref_thumb']}' target='_blank'><img src='{r['ref_thumb']}'></a><div class='lbl'>{ref_label}</div></td>
  <td><a href='{r['got_thumb']}' target='_blank'><img src='{r['got_thumb']}'></a><div class='lbl'>Godot</div></td>
  <td><a href='{r['diff_thumb']}' target='_blank'><img src='{r['diff_thumb']}'></a><div class='lbl'>Diff ×4</div></td>
  <td class='stats'>
    <div>Mean Δ R: <b>{ps['mean_dr']}</b></div>
    <div>Mean Δ G: <b>{ps['mean_dg']}</b></div>
    <div>Mean Δ B: <b>{ps['mean_db']}</b></div>
    <div>Mean Δ total: <b>{ps['mean_manhattan']}</b></div>
    <div class='zones'>{zones_html}</div>
  </td>
</tr>""")
        section_html.append(f"""
<h2>{title}</h2>
<table>
<thead><tr><th>Key</th><th>Reference</th><th>Godot</th><th>Diff</th><th>Stats</th></tr></thead>
<tbody>{''.join(body_rows)}</tbody>
</table>""")

    html = f"""<!doctype html>
<html><head><meta charset='utf-8'><title>Skybox A/B Report</title>
<style>
  body {{ font-family: -apple-system, sans-serif; margin: 16px; background: #1a1a1a; color: #e0e0e0; }}
  h1 {{ margin: 0 0 8px; }}
  h2 {{ margin: 24px 0 8px; padding: 8px 12px; background: #252525; border-left: 4px solid #4a90e2; }}
  .summary {{ background: #2a2a2a; padding: 12px; border-radius: 6px; margin-bottom: 16px; line-height: 1.7; }}
  .summary span {{ display: inline-block; margin-right: 24px; }}
  table {{ border-collapse: collapse; width: 100%; }}
  td, th {{ vertical-align: top; padding: 8px; border-top: 1px solid #333; text-align: left; }}
  td img {{ display: block; max-width: 100%; }}
  .lbl {{ text-align: center; font-size: 11px; color: #888; margin-top: 2px; }}
  .stats {{ font-size: 12px; min-width: 320px; }}
  .stats > div {{ margin-bottom: 4px; }}
  .zones {{ margin-top: 8px; padding-top: 8px; border-top: 1px solid #333; }}
  .z {{ font-size: 11px; margin-bottom: 2px; }}
  .swatch {{ display: inline-block; width: 14px; height: 14px; border: 1px solid #555; vertical-align: middle; margin: 0 2px; }}
  .sev {{ padding: 2px 6px; border-radius: 3px; font-size: 10px; font-weight: bold; }}
  .sev-ok    {{ background: #1f5f1f; color: #fff; }}
  .sev-warn  {{ background: #6f5f1f; color: #fff; }}
  .sev-bad   {{ background: #6f1f1f; color: #fff; }}
  tr.sev-bad  td:first-child {{ border-left: 3px solid #c33; }}
  tr.sev-warn td:first-child {{ border-left: 3px solid #cc3; }}
  tr.sev-ok   td:first-child {{ border-left: 3px solid #3c3; }}
</style></head>
<body>
<h1>Skybox A/B: Godot vs Unity (iterative)</h1>
<div class='summary'>
  <div><b>Target hours:</b> {', '.join(target_hours) if target_hours else '(none)'}</div>
  <div><b>Approved snapshots:</b> {len(approved_keys)} files</div>
  <div>
    <span>TARGET pairs: <b>{summary['target_count']}</b> (avg Δ {summary['target_avg']:.1f})</span>
    <span>LOCKED pairs: <b>{summary['locked_count']}</b> (avg Δ {summary['locked_avg']:.1f})</span>
    <span>PENDING pairs: <b>{summary['pending_count']}</b> (avg Δ {summary['pending_avg']:.1f})</span>
  </div>
  <div>
    <span>Crop: x[{int(CROP_LEFT*100)}–{int(CROP_RIGHT*100)}%] y[{int(CROP_TOP*100)}–{int(CROP_BOTTOM*100)}%]</span>
    <span>OK threshold: target Δ&lt;{DELTA_OK} • locked Δ&lt;{LOCKED_REGRESSION_TOLERANCE}</span>
  </div>
</div>
{''.join(section_html)}
</body></html>
"""
    path = out_dir / "index.html"
    path.write_text(html)
    return path


def main():
    p = argparse.ArgumentParser()
    p.add_argument("unity_dir", help="Unity reference screenshots dir")
    p.add_argument("godot_dir", help="Godot capture dir")
    p.add_argument("out_dir", help="Output dir for report")
    p.add_argument("--approved-dir", default="", help="Approved Godot snapshots (regression baseline)")
    p.add_argument("--target-hours", default="", help="Comma-separated hours to compare vs Unity (e.g. 00,12)")
    args = p.parse_args()

    unity_dir = Path(args.unity_dir).expanduser().resolve()
    godot_dir = Path(args.godot_dir).expanduser().resolve()
    out_dir = Path(args.out_dir).expanduser().resolve()
    approved_dir = Path(args.approved_dir).expanduser().resolve() if args.approved_dir else None
    target_hours = [h.strip().zfill(2) for h in args.target_hours.split(",") if h.strip()]

    if not unity_dir.is_dir() or not godot_dir.is_dir():
        print(f"Error: dirs must exist: {unity_dir} {godot_dir}")
        return 1

    out_dir.mkdir(parents=True, exist_ok=True)
    approved_keys = set()
    if approved_dir and approved_dir.is_dir():
        for f in approved_dir.glob("*.png"):
            approved_keys.add(f.stem)

    print(f"📷 Unity:    {unity_dir}")
    print(f"📷 Godot:    {godot_dir}")
    print(f"✅ Approved: {approved_dir if approved_dir else '(none)'}  ({len(approved_keys)} files)")
    print(f"🎯 Target hours: {target_hours if target_hours else '(none)'}")
    print(f"📊 Output:   {out_dir}")

    results = []
    for direction in DIRECTIONS:
        for hour in HOURS:
            hh = f"{hour:02d}"
            key = f"{direction}{hh}"
            godot_path = godot_dir / f"{key}.png"
            if not godot_path.exists():
                continue

            if hh in target_hours:
                ref_path = unity_dir / f"{key}.png"
                status = "TARGET"
            elif key in approved_keys:
                ref_path = approved_dir / f"{key}.png"
                status = "LOCKED"
            else:
                ref_path = unity_dir / f"{key}.png"
                status = "PENDING"

            if not ref_path.exists():
                continue

            try:
                r = compare_pair(ref_path, godot_path, out_dir, key, status)
                results.append(r)
                tag = {"TARGET": "🎯", "LOCKED": "🔒", "PENDING": "⏳"}[status]
                print(f"  {tag} {key}  Δ {r['pixel_stats']['mean_manhattan']:.1f}  ({r['severity']})")
            except Exception as e:
                print(f"  ✗ {key}: {e}")

    if not results:
        print("\nNo comparable pairs found.")
        return 1

    def avg(rs):
        if not rs: return 0.0
        return sum(r["pixel_stats"]["mean_manhattan"] for r in rs) / len(rs)

    target = [r for r in results if r["status"] == "TARGET"]
    locked = [r for r in results if r["status"] == "LOCKED"]
    pending = [r for r in results if r["status"] == "PENDING"]

    summary = {
        "total": len(results),
        "target_count": len(target),
        "locked_count": len(locked),
        "pending_count": len(pending),
        "target_avg": avg(target),
        "locked_avg": avg(locked),
        "pending_avg": avg(pending),
    }

    (out_dir / "data.json").write_text(json.dumps({
        "summary": summary, "results": results,
        "target_hours": target_hours,
        "approved_keys": sorted(approved_keys),
    }, indent=2))
    html_path = render_html(results, out_dir, summary, target_hours, approved_keys)
    print(f"\n✅ Report: {html_path}")
    print(f"   Target avg Δ: {summary['target_avg']:.1f}  |  Locked avg Δ: {summary['locked_avg']:.1f}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
