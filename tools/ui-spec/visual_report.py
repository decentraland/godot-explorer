#!/usr/bin/env python3
"""Plumbing around reg-cli for the RN visual snapshots (the comparison itself is `reg-cli`, see rn_visual.yml).

  python3 tools/ui-spec/visual_report.py optimize DIR
  python3 tools/ui-spec/visual_report.py prepare --new DIR [--baseline rn/visual-baselines] [--report visual-report]
  python3 tools/ui-spec/visual_report.py summary [--report visual-report] [--run-url URL]
  python3 tools/ui-spec/visual_report.py promote --new DIR [--baseline rn/visual-baselines]

`optimize` recompresses every PNG in DIR losslessly (opaque RGB, no metadata): baselines stay as small as plain
git allows without touching a pixel.
`prepare` lays out REPORT/actual and REPORT/expected (minus the ids in visual-ignore.json) for reg-cli, which then
writes REPORT/index.html, REPORT/reg.json and REPORT/diff.
`summary` turns reg.json into REPORT/report.md (the PR comment). Exit 1 only when a registered screen was not
captured: that is an infrastructure problem, a pixel change is only reported.
`promote` replaces the baselines with the registered screens of DIR.
"""
import argparse, json, os, shutil, sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
IDS_JSON = os.path.join(ROOT, "rn", "src", "gallery", "ids.json")
IGNORE_JSON = os.path.join(os.path.dirname(__file__), "visual-ignore.json")
DEFAULT_BASELINE = os.path.join(ROOT, "rn", "visual-baselines")
MARKER = "<!-- rn-visual-snapshots -->"
LISTED_MAX = 40


def optimize(path):
    from PIL import Image  # only the capture jobs have Pillow

    Image.open(path).convert("RGB").save(path, optimize=True, compress_level=9)


def png_files(directory):
    for base, _, names in os.walk(directory):
        for name in names:
            if name.endswith(".png"):
                yield os.path.join(base, name)


def ignored_ids():
    return set(json.load(open(IGNORE_JSON))) if os.path.exists(IGNORE_JSON) else set()


def registered_ids():
    return [row["id"] for row in json.load(open(IDS_JSON))]


def copy_pngs(src, dest, skip):
    for path in png_files(src):
        rel = os.path.relpath(path, src)
        if rel[: -len(".png")] in skip:
            continue
        target = os.path.join(dest, rel)
        os.makedirs(os.path.dirname(target), exist_ok=True)
        shutil.copyfile(path, target)


def prepare(new_dir, baseline_dir, report_dir):
    if os.path.isdir(report_dir):
        shutil.rmtree(report_dir)
    skip = ignored_ids()
    copy_pngs(new_dir, os.path.join(report_dir, "actual"), skip)
    copy_pngs(baseline_dir, os.path.join(report_dir, "expected"), skip)


def summary(report_dir, run_url):
    reg = json.load(open(os.path.join(report_dir, "reg.json")))
    skip = ignored_ids()
    captured = {name[: -len(".png")] for name in reg["actualItems"]}
    missing = [i for i in registered_ids() if i not in skip and i not in captured]
    strip = lambda names: [n[: -len(".png")] for n in names]
    changed, new = strip(reg["failedItems"]), strip(reg["newItems"])
    lines = [
        MARKER,
        "### RN visual snapshots",
        "",
        f"{len(captured)} screens: **{len(changed)} changed**, {len(new)} new, {len(missing)} not captured, "
        f"{len(reg['passedItems'])} unchanged" + (f", {len(skip)} ignored" if skip else "")
        + ". Report only: this check does not block the PR.",
    ]
    if changed:
        lines += ["", "**Changed:** " + ", ".join(f"`{i}`" for i in changed[:LISTED_MAX])]
        if len(changed) > LISTED_MAX:
            lines.append(f"… and {len(changed) - LISTED_MAX} more")
    if new:
        lines += ["", f"**New (no baseline yet):** {len(new)} screens."]
    if missing:
        lines += ["", "**Not captured:** " + ", ".join(f"`{i}`" for i in missing[:LISTED_MAX])]
    lines += ["", "Open `index.html` of the `rn-visual-report` artifact for the baseline | new | diff images."]
    if run_url:
        lines.append(f"Run: {run_url}")
    markdown = "\n".join(lines) + "\n"
    open(os.path.join(report_dir, "report.md"), "w").write(markdown)
    print(markdown)
    return 1 if missing else 0


def promote(new_dir, baseline_dir):
    wanted = {i + ".png" for i in registered_ids()}
    if os.path.isdir(baseline_dir):
        shutil.rmtree(baseline_dir)
    for path in png_files(new_dir):
        rel = os.path.relpath(path, new_dir)
        if rel in wanted:
            dest = os.path.join(baseline_dir, rel)
            os.makedirs(os.path.dirname(dest), exist_ok=True)
            shutil.copyfile(path, dest)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawTextHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    sub.add_parser("optimize").add_argument("dir")
    p = sub.add_parser("prepare")
    p.add_argument("--new", required=True)
    p.add_argument("--baseline", default=DEFAULT_BASELINE)
    p.add_argument("--report", default="visual-report")
    s = sub.add_parser("summary")
    s.add_argument("--report", default="visual-report")
    s.add_argument("--run-url", default="")
    r = sub.add_parser("promote")
    r.add_argument("--new", required=True)
    r.add_argument("--baseline", default=DEFAULT_BASELINE)
    args = parser.parse_args()

    if args.cmd == "optimize":
        for path in png_files(args.dir):
            optimize(path)
    elif args.cmd == "prepare":
        prepare(args.new, args.baseline, args.report)
    elif args.cmd == "summary":
        sys.exit(summary(args.report, args.run_url))
    elif args.cmd == "promote":
        promote(args.new, args.baseline)


if __name__ == "__main__":
    main()
