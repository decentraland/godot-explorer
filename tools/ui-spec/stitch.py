#!/usr/bin/env python3
"""Stitch the Figma screenshots in docs/rn-ui-spec/shots into big contact sheets.

  python3 tools/ui-spec/stitch.py                      # one sheet per source + ALL.png
  python3 tools/ui-spec/stitch.py --source hud --th 700
  python3 tools/ui-spec/stitch.py --compare rn-shots   # Figma | RN side by side (+ diff score)

Cells are labelled `seq id  WxH`. A screen with no downloaded PNG is drawn as a red MISSING tile, so a
sheet doubles as a completeness check. RN renders are matched by manifest id (`<id>.png`).
"""
import argparse, json, os, sys
from PIL import Image, ImageDraw, ImageFont, ImageChops, ImageStat

Image.MAX_IMAGE_PIXELS = None
ROOT = os.path.join(os.path.dirname(__file__), "..", "..", "docs", "rn-ui-spec")
BG, FG, WARN = (24, 24, 28), (235, 235, 240), (190, 40, 40)


def font(n):
    for p in ("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",):
        if os.path.exists(p):
            return ImageFont.truetype(p, n)
    return ImageFont.load_default(n)


def letterbox(src, w, h):
    k = min(w / src.width, h / src.height)
    fit = src.resize((max(1, round(src.width * k)), max(1, round(src.height * k))), Image.LANCZOS)
    im = Image.new("RGB", (w, h), BG)
    im.paste(fit, ((w - fit.width) // 2, (h - fit.height) // 2))
    return im


def tile(e, th, root, subdir=None):
    w = max(1, round(e["w"] * th / e["h"]))
    cap = 4000
    path = os.path.join(root, subdir or "", e["file"] if subdir is None else e["id"] + ".png")
    if os.path.exists(path):
        try:
            src = Image.open(path).convert("RGB")
            if w > cap:  # wide component strips: fit the width, keep aspect, pad to the row height
                im = Image.new("RGB", (cap, th), BG)
                im.paste(src.resize((cap, max(1, round(src.height * cap / src.width))), Image.LANCZOS), (0, 0))
                return im, True
            if subdir is not None:  # RN capture: letterbox to the Figma aspect instead of stretching
                return letterbox(src, w, th), True
            return src.resize((w, th), Image.LANCZOS), True
        except Exception:
            pass
    im = Image.new("RGB", (min(w, cap), th), WARN)
    ImageDraw.Draw(im).text((10, th // 2), "MISSING", fill=FG, font=font(28))
    return im, False


def sheet(entries, th, width, root, title, cmp_dir=None):
    pad, lab = 16, 26
    f, ft = font(18), font(40)
    cells = []
    for e in entries:
        a, ok = tile(e, th, root)
        label = f'{e["seq"]:03d} {e["name"][:34]} {e["w"]}x{e["h"]}'
        if cmp_dir:
            b, okb = tile(e, th, cmp_dir, subdir="")
            score = ""
            if ok and okb:
                d = ImageChops.difference(a, b.resize(a.size))
                score = f"  diff={sum(ImageStat.Stat(d).mean) / 3 / 255:.3f}"
            c = Image.new("RGB", (a.width + b.width + 8, th), BG)
            c.paste(a, (0, 0)); c.paste(b, (a.width + 8, 0))
            a, label = c, label + score + ("" if okb else "  [no RN render]")
        cells.append((a, label, ok))
    rows, row, x = [], [], 0
    for c in cells:
        if row and x + c[0].width > width:
            rows.append(row); row, x = [], 0
        row.append(c); x += c[0].width + pad
    if row:
        rows.append(row)
    H = 80 + sum(th + lab + pad for _ in rows)
    W = max([width] + [sum(c[0].width + pad for c in r) for r in rows])
    im = Image.new("RGB", (W, H), BG)
    d = ImageDraw.Draw(im)
    miss = sum(1 for c in cells if not c[2])
    d.text((pad, 16), f"{title} - {len(cells)} screens, {miss} missing", fill=FG, font=ft)
    y = 80
    for r in rows:
        x = 0
        for img, label, ok in r:
            im.paste(img, (x, y + lab))
            d.text((x, y + 2), label, fill=FG if ok else (255, 120, 120), font=f)
            x += img.width + pad
        y += th + lab + pad
    return im


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--source"); ap.add_argument("--th", type=int, default=520)
    ap.add_argument("--width", type=int, default=7000); ap.add_argument("--compare")
    ap.add_argument("--kind", default="screen,strip,fragment")
    a = ap.parse_args()
    m = json.load(open(os.path.join(ROOT, "manifest.json")))["screens"]
    kinds = a.kind.split(",")
    m = [e for e in m if e["kind"] in kinds]
    out = os.path.join(ROOT, "out"); os.makedirs(out, exist_ok=True)
    srcs = [a.source] if a.source else list(dict.fromkeys(e["source"] for e in m))
    cmp_dir = os.path.join(ROOT, a.compare) if a.compare else None
    made = []
    for s in srcs:
        es = [e for e in m if e["source"] == s]
        im = sheet(es, a.th, a.width, ROOT, s, cmp_dir)
        p = os.path.join(out, f'{"compare-" if cmp_dir else ""}{s}.png'); im.save(p); made.append(p)
        print(p, im.size)
    if not a.source:
        ims = [Image.open(p) for p in made]
        W = max(i.width for i in ims)
        big = Image.new("RGB", (W, sum(i.height for i in ims)), BG)
        y = 0
        for i in ims:
            big.paste(i, (0, y)); y += i.height
        p = os.path.join(out, f'{"compare-" if cmp_dir else ""}ALL.png'); big.save(p)
        print(p, big.size)


if __name__ == "__main__":
    sys.exit(main())
