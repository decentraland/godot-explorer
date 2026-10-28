#!/usr/bin/env python3
"""Compare RN preview captures with the committed baselines.

  python3 tools/ui-spec/visual_diff.py optimize DIR
  python3 tools/ui-spec/visual_diff.py compare --new DIR [--baseline rn/visual-baselines] [--out OUT]
  python3 tools/ui-spec/visual_diff.py promote --new DIR [--baseline rn/visual-baselines]

`optimize` rewrites every PNG in DIR as an opaque, losslessly recompressed RGB image (the status bar alpha and
metadata go), so the baselines stay as small as plain git allows without touching a pixel.
`compare` walks the ids in rn/src/gallery/ids.json and classifies each: same, changed, new (no baseline) or
missing (registered but not captured). It writes OUT/report.json, OUT/report.md (the PR comment) and, for each
changed id, OUT/changed/<id>.png = baseline | new | diff. Exit 1 only when an id is missing: a screen that
could not be captured is an infrastructure problem, a pixel change is only reported.
"""
import argparse, json, os, shutil, sys
from PIL import Image, ImageChops

Image.MAX_IMAGE_PIXELS = None
ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
IDS_JSON = os.path.join(ROOT, "rn", "src", "gallery", "ids.json")
IGNORE_JSON = os.path.join(os.path.dirname(__file__), "visual-ignore.json")
DEFAULT_BASELINE = os.path.join(ROOT, "rn", "visual-baselines")

# A pixel differs when any channel moves by more than CHANNEL_THRESHOLD (of 255); a screen is "changed"
# when more than RATIO_THRESHOLD of its pixels differ. Tune here after measuring run-to-run noise.
CHANNEL_THRESHOLD = 16
RATIO_THRESHOLD = 0.0005
# The gesture-bar pill is system UI and its width changes between runs: the bottom rows are not compared.
NAV_STRIP = 56
THUMB_HEIGHT = 640
MARKER = "<!-- rn-visual-snapshots -->"
LISTED_MAX = 40


def optimize(path):
    Image.open(path).convert("RGB").save(path, optimize=True, compress_level=9)


def png_files(directory):
    for base, _, names in os.walk(directory):
        for name in names:
            if name.endswith(".png"):
                yield os.path.join(base, name)


def load_rgb(path):
    return Image.open(path).convert("RGB")


def diff_mask(a, b):
    r, g, bl = ImageChops.difference(a, b).split()
    peak = ImageChops.lighter(ImageChops.lighter(r, g), bl)
    mask = peak.point(lambda v: 255 if v > CHANNEL_THRESHOLD else 0)
    mask.paste(0, (0, mask.height - NAV_STRIP, mask.width, mask.height))
    return mask


def triptych(baseline, new, mask, dest):
    """baseline | new | diff, scaled to THUMB_HEIGHT, for the artifact and the reviewer."""
    scale = THUMB_HEIGHT / max(baseline.height, new.height)
    size = (int(new.width * scale), int(new.height * scale))
    red = Image.new("RGB", new.size, (255, 40, 40))
    dim = Image.blend(new, Image.new("RGB", new.size, (0, 0, 0)), 0.65)
    heat = Image.composite(red, dim, mask)
    panels = [baseline.resize(size), new.resize(size), heat.resize(size)]
    sheet = Image.new("RGB", (size[0] * 3 + 8, size[1]), (24, 24, 28))
    for i, panel in enumerate(panels):
        sheet.paste(panel, (i * (size[0] + 4), 0))
    os.makedirs(os.path.dirname(dest), exist_ok=True)
    sheet.save(dest)


def compare(new_dir, baseline_dir, out_dir):
    ids = [row["id"] for row in json.load(open(IDS_JSON))]
    ignored = json.load(open(IGNORE_JSON)) if os.path.exists(IGNORE_JSON) else {}
    rows = []
    for ident in ids:
        new_path = os.path.join(new_dir, ident + ".png")
        base_path = os.path.join(baseline_dir, ident + ".png")
        row = {"id": ident, "status": "same", "ratio": 0.0}
        if ident in ignored:
            row["status"] = "ignored"
        elif not os.path.exists(new_path):
            row["status"] = "missing"
        elif not os.path.exists(base_path):
            row["status"] = "new"
        else:
            new, base = load_rgb(new_path), load_rgb(base_path)
            if new.size != base.size:
                row.update(status="changed", ratio=1.0, note=f"size {base.size} -> {new.size}")
            else:
                mask = diff_mask(base, new)
                ratio = mask.histogram()[255] / (new.width * new.height)
                row["ratio"] = round(ratio, 6)
                if ratio > RATIO_THRESHOLD:
                    row["status"] = "changed"
                    triptych(base, new, mask, os.path.join(out_dir, "changed", ident + ".png"))
        rows.append(row)
    return rows


def report_markdown(rows):
    count = lambda status: sum(1 for r in rows if r["status"] == status)
    lines = [
        MARKER,
        "### RN visual snapshots",
        "",
        f"{len(rows)} screens: **{count('changed')} changed**, {count('new')} new, "
        f"{count('missing')} not captured, {count('same')} unchanged"
        + (f", {count('ignored')} ignored" if count("ignored") else "")
        + ". Report only: this check does not block the PR.",
        "",
    ]
    changed = sorted((r for r in rows if r["status"] == "changed"), key=lambda r: -r["ratio"])
    if changed:
        lines += ["| Screen | Pixels changed |", "|---|---|"]
        for r in changed[:LISTED_MAX]:
            lines.append(f"| `{r['id']}` | {r['ratio'] * 100:.2f}% {r.get('note', '')} |")
        if len(changed) > LISTED_MAX:
            lines.append(f"| … {len(changed) - LISTED_MAX} more | see `report.json` |")
        lines += ["", "Side-by-side images (baseline | new | diff) are in the `rn-visual-report` artifact of this run."]
    missing = [r["id"] for r in rows if r["status"] == "missing"]
    if missing:
        lines += ["", "**Not captured:** " + ", ".join(f"`{i}`" for i in missing[:LISTED_MAX])]
    new = [r["id"] for r in rows if r["status"] == "new"]
    if new:
        lines += ["", f"**New (no baseline yet):** {len(new)} screens."]
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawTextHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    q = sub.add_parser("optimize")
    q.add_argument("dir")
    c = sub.add_parser("compare")
    c.add_argument("--new", required=True)
    c.add_argument("--baseline", default=DEFAULT_BASELINE)
    c.add_argument("--out", default="visual-report")
    p = sub.add_parser("promote")
    p.add_argument("--new", required=True)
    p.add_argument("--baseline", default=DEFAULT_BASELINE)
    args = parser.parse_args()

    if args.cmd == "optimize":
        for path in png_files(args.dir):
            optimize(path)
    elif args.cmd == "compare":
        os.makedirs(args.out, exist_ok=True)
        rows = compare(args.new, args.baseline, args.out)
        json.dump(rows, open(os.path.join(args.out, "report.json"), "w"), indent=1)
        markdown = report_markdown(rows)
        open(os.path.join(args.out, "report.md"), "w").write(markdown)
        print(markdown)
        sys.exit(1 if any(r["status"] == "missing" for r in rows) else 0)
    elif args.cmd == "promote":
        wanted = {row["id"] + ".png" for row in json.load(open(IDS_JSON))}
        if os.path.isdir(args.baseline):
            shutil.rmtree(args.baseline)
        for path in png_files(args.new):
            rel = os.path.relpath(path, args.new)
            if rel in wanted:
                dest = os.path.join(args.baseline, rel)
                os.makedirs(os.path.dirname(dest), exist_ok=True)
                shutil.copyfile(path, dest)


if __name__ == "__main__":
    main()
