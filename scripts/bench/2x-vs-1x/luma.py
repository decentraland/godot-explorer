#!/usr/bin/env python3
"""Light record from one screenshot (#3206).

    luma.py docs/bench/data/.../screen.png          # human output
    luma.py docs/bench/data/.../screen.png --json   # one JSON object

`luma_full` is the Rec.601 mean of the whole frame and is the historical `state.json:luma_mean`, so
it stays comparable with the runs already on record. `luma_crop` is the middle 70 % x 70 % of the
frame: the two shells draw different HUD chrome, so the full-frame mean carries a build-correlated
UI bias that the crop drops.

Both numbers are Rec.601 luma (Y = 0.299 R + 0.587 G + 0.114 B) over the sRGB-encoded values, which
is exactly what PIL's convert("L") computes; the per-band means are combined explicitly so the crop
and the full frame use the same weights.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

REC601 = (0.299, 0.587, 0.114)
CROP_FRACTION = 0.70  # centred 70 % x 70 %


def _pil():
    try:
        from PIL import Image, ImageStat
    except ImportError as exc:  # the existing scripts already depend on PIL for this
        raise SystemExit(
            f"luma.py needs Pillow ({exc}). Install it with: python3 -m pip install pillow"
        ) from exc
    return Image, ImageStat


def mean_luma(image) -> float:
    """Rec.601 mean of an RGB image."""
    from PIL import ImageStat

    red, green, blue = ImageStat.Stat(image).mean[:3]
    return REC601[0] * red + REC601[1] * green + REC601[2] * blue


def luma(path: Path) -> dict:
    Image, _ = _pil()
    with Image.open(path) as source:
        rgb = source.convert("RGB")
        width, height = rgb.size
        margin_x = round(width * (1.0 - CROP_FRACTION) / 2.0)
        margin_y = round(height * (1.0 - CROP_FRACTION) / 2.0)
        box = (margin_x, margin_y, width - margin_x, height - margin_y)
        full = mean_luma(rgb)
        crop = mean_luma(rgb.crop(box))
    return {
        "image": str(path),
        "width": width,
        "height": height,
        "luma_full": round(full, 2),
        "luma_crop": round(crop, 2),
        "crop_box": list(box),
        "method": "rec601",
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Rec.601 luma of one screenshot: full frame and centred 70 % x 70 % crop",
        usage="luma.py IMAGE [--json]",
    )
    parser.add_argument("image", type=Path, help="screenshot (screen.png of a run)")
    parser.add_argument("--json", action="store_true", help="print one JSON object")
    args = parser.parse_args(argv)

    if not args.image.is_file():
        print(f"luma.py: no such file: {args.image}", file=sys.stderr)
        return 2
    try:
        record = luma(args.image)
    except OSError as exc:
        print(f"luma.py: cannot read {args.image}: {exc}", file=sys.stderr)
        return 2

    if args.json:
        print(json.dumps(record))
    else:
        print(f"{record['image']}  {record['width']}x{record['height']}")
        print(f"  luma_full  {record['luma_full']:.2f}")
        print(f"  luma_crop  {record['luma_crop']:.2f}   box {record['crop_box']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
