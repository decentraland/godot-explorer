#!/usr/bin/env python3
"""Pixel measurements for Figma-vs-implementation comparisons (needs ImageMagick `magick`).

  python3 -I measure.py <figma.png> <impl.png> name:x,y,w,h:snow  [name:x,y,w,h:ruby ...]

Both images must be the same size (720x1600). For every region it prints the bounding box of the
pixels matching the predicate in each image and the delta (impl - figma): left, top, right, bottom.
Predicates: snow (text/white), ruby (CTA red), white (>240), dark, modal, link, any:<r>,<g>,<b>,<tol>.
Use it for position and size; sample colours with `magick img -format '%[pixel:p{x,y}]' info:`.
"""
import re
import subprocess
import sys

PRED = {
    "snow": lambda r, g, b: r > 235 and g > 235 and b > 235,
    "white": lambda r, g, b: r > 240 and g > 240 and b > 240,
    "ruby": lambda r, g, b: r > 235 and g < 80 and b < 120,
    "dark": lambda r, g, b: r < 60 and g < 40 and b < 100 and b > 20,
    "modal": lambda r, g, b: abs(r - 76) < 10 and abs(g - 20) < 10 and abs(b - 124) < 12,
    "link": lambda r, g, b: r > 130 and g > 140 and b > 230,
}


def points(path, x0, y0, w, h):
    out = subprocess.check_output(["magick", path, "-crop", f"{w}x{h}+{x0}+{y0}", "+repage", "txt:-"]).decode()
    for line in out.splitlines():
        m = re.match(r"(\d+),(\d+): \((\d+),(\d+),(\d+)", line)
        if m:
            x, y, r, g, b = map(int, m.groups())
            yield x + x0, y + y0, r, g, b


def bbox(path, region, pred):
    xs, ys = [], []
    for x, y, r, g, b in points(path, *region):
        if pred(r, g, b):
            xs.append(x)
            ys.append(y)
    return (min(xs), min(ys), max(xs), max(ys)) if xs else None


def runs_y(path, x, y0, y1, pred, gap=3, minlen=10):
    """Vertical runs of matching pixels along one column: the rows of stacked buttons or fields."""
    ys = sorted(y for _, y, r, g, b in points(path, x, y0, 1, y1 - y0) if pred(r, g, b))
    runs, start, prev = [], None, None
    for y in ys:
        if start is None:
            start = y
        elif y - prev > gap:
            runs.append((start, prev))
            start = y
        prev = y
    if start is not None:
        runs.append((start, prev))
    return [(a, b) for a, b in runs if b - a > minlen]


def main(argv):
    figma, impl, specs = argv[1], argv[2], argv[3:]
    for spec in specs:
        name, region, kind = spec.split(":", 2)
        reg = tuple(int(v) for v in region.split(","))
        pred = PRED.get(kind)
        if pred is None and kind.startswith("any"):
            r0, g0, b0, tol = (int(v) for v in kind.split(":")[1].split(","))
            pred = lambda r, g, b: abs(r - r0) <= tol and abs(g - g0) <= tol and abs(b - b0) <= tol
        a, b = bbox(figma, reg, pred), bbox(impl, reg, pred)
        delta = tuple(y - x for x, y in zip(a, b)) if a and b else None
        print(f"{name:12} figma={a} impl={b} delta={delta}")


if __name__ == "__main__":
    main(sys.argv)
